use majstack_core::{MajstackError, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessSpec {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub env: BTreeMap<String, String>,
    pub stdin: Option<String>,
    pub timeout_seconds: u64,
}

impl ProcessSpec {
    pub fn new(program: impl Into<String>, args: Vec<String>, cwd: impl Into<PathBuf>) -> Self {
        ProcessSpec {
            program: program.into(),
            args,
            cwd: cwd.into(),
            env: BTreeMap::new(),
            stdin: None,
            timeout_seconds: 0,
        }
    }

    pub fn timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = seconds;
        self
    }

    pub fn stdin(mut self, input: impl Into<String>) -> Self {
        self.stdin = Some(input.into());
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessOutput {
    pub command: String,
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    pub timed_out: bool,
}

impl ProcessOutput {
    pub fn combined(&self) -> String {
        let mut out = self.stdout.clone();
        if !self.stderr.is_empty() {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(&self.stderr);
        }
        out
    }

    pub fn success(&self) -> bool {
        self.code == 0 && !self.timed_out
    }
}

fn read_stream<T: std::io::Read + Send + 'static>(reader: T) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let mut collected = String::new();
        let mut buf = BufReader::new(reader);
        let mut line = String::new();
        loop {
            line.clear();
            match buf.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => collected.push_str(&line),
                Err(_) => break,
            }
        }
        collected
    })
}

pub fn run_process(spec: &ProcessSpec) -> Result<ProcessOutput> {
    let mut command = Command::new(&spec.program);
    command
        .args(&spec.args)
        .current_dir(&spec.cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in &spec.env {
        command.env(key, value);
    }
    let start = Instant::now();
    let mut child = command
        .spawn()
        .map_err(|e| MajstackError::Tool(format!("failed to spawn '{}': {e}", spec.program)))?;

    if let Some(input) = &spec.stdin {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(input.as_bytes());
            let _ = stdin.flush();
        }
    } else {
        drop(child.stdin.take());
    }

    let stdout_handle = child.stdout.take().map(read_stream);
    let stderr_handle = child.stderr.take().map(read_stream);

    let mut timed_out = false;
    let status = if spec.timeout_seconds == 0 {
        child
            .wait()
            .map_err(|e| MajstackError::Tool(e.to_string()))?
    } else {
        let deadline = Instant::now() + Duration::from_secs(spec.timeout_seconds);
        loop {
            match child
                .try_wait()
                .map_err(|e| MajstackError::Tool(e.to_string()))?
            {
                Some(status) => break status,
                None => {
                    if Instant::now() >= deadline {
                        let _ = child.kill();
                        timed_out = true;
                        break child
                            .wait()
                            .map_err(|e| MajstackError::Tool(e.to_string()))?;
                    }
                    std::thread::sleep(Duration::from_millis(40));
                }
            }
        }
    };

    let stdout = stdout_handle
        .and_then(|h| h.join().ok())
        .unwrap_or_default();
    let stderr = stderr_handle
        .and_then(|h| h.join().ok())
        .unwrap_or_default();
    let mut display = spec.program.clone();
    for arg in &spec.args {
        display.push(' ');
        display.push_str(arg);
    }

    Ok(ProcessOutput {
        command: display,
        code: status.code().unwrap_or(-1),
        stdout,
        stderr,
        duration_ms: start.elapsed().as_millis() as u64,
        timed_out,
    })
}

pub fn run_shell(
    command: &str,
    cwd: impl AsRef<Path>,
    timeout_seconds: u64,
) -> Result<ProcessOutput> {
    let spec = if cfg!(windows) {
        ProcessSpec::new("cmd", vec!["/C".into(), command.into()], cwd.as_ref())
    } else {
        ProcessSpec::new("sh", vec!["-c".into(), command.into()], cwd.as_ref())
    };
    run_process(&spec.timeout(timeout_seconds))
}

pub fn command_exists(program: &str) -> bool {
    which::which(program).is_ok()
}

pub fn which_path(program: &str) -> Option<PathBuf> {
    which::which(program).ok()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorReport {
    pub checks: Vec<DoctorCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorCheck {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

impl DoctorReport {
    pub fn passing(&self) -> bool {
        self.checks.iter().all(|check| check.ok)
    }
}

pub fn doctor(required_tools: &[&str]) -> DoctorReport {
    let mut checks = Vec::new();
    for tool in ["git", "node", "npm", "cargo", "python"] {
        let ok = command_exists(tool);
        checks.push(DoctorCheck {
            name: format!("tool:{tool}"),
            ok,
            detail: if ok { "found".into() } else { "missing".into() },
        });
    }
    for tool in required_tools {
        let ok = command_exists(tool);
        checks.push(DoctorCheck {
            name: format!("required:{tool}"),
            ok,
            detail: if ok { "found".into() } else { "missing".into() },
        });
    }
    DoctorReport { checks }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captures_stdout_and_code() {
        let cwd = std::env::temp_dir();
        let spec = ProcessSpec::new("cmd", vec!["/C".into(), "echo hello".into()], &cwd);
        let spec = if cfg!(windows) {
            spec
        } else {
            ProcessSpec::new("sh", vec!["-c".into(), "echo hello".into()], &cwd)
        };
        let output = run_process(&spec).unwrap();
        assert!(output.stdout.contains("hello"));
        assert_eq!(output.code, 0);
    }

    #[test]
    fn enforces_timeout() {
        let cwd = std::env::temp_dir();
        let spec = if cfg!(windows) {
            ProcessSpec::new(
                "cmd",
                vec!["/C".into(), "ping -n 6 127.0.0.1 > NUL".into()],
                &cwd,
            )
        } else {
            ProcessSpec::new("sh", vec!["-c".into(), "sleep 5".into()], &cwd)
        };
        let output = run_process(&spec.timeout(1)).unwrap();
        assert!(output.timed_out);
    }
}
