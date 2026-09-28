use majstack_core::{MajstackError, PermissionLevel, Result};
use majstack_execution::run_shell;
use majstack_policy::Policy;
use majstack_state::Store;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub output_schema: Value,
    pub permission: PermissionLevel,
    pub side_effects: bool,
    pub timeout_seconds: u64,
    pub retryable: bool,
    pub audit: bool,
}

impl ToolSpec {
    pub fn new(name: &str, description: &str, permission: PermissionLevel) -> Self {
        ToolSpec {
            name: name.to_string(),
            description: description.to_string(),
            input_schema: Value::Object(serde_json::Map::new()),
            output_schema: Value::Object(serde_json::Map::new()),
            permission,
            side_effects: permission != PermissionLevel::ReadOnly,
            timeout_seconds: 120,
            retryable: false,
            audit: true,
        }
    }
}

#[derive(Clone)]
pub struct ToolContext {
    pub root: PathBuf,
    pub policy: Policy,
    pub store: Arc<Store>,
    pub run_id: Option<String>,
    pub task_id: Option<String>,
}

impl ToolContext {
    pub fn new(root: impl Into<PathBuf>, policy: Policy, store: Arc<Store>) -> Self {
        ToolContext {
            root: root.into(),
            policy,
            store,
            run_id: None,
            task_id: None,
        }
    }

    pub fn with_scope(mut self, run_id: Option<String>, task_id: Option<String>) -> Self {
        self.run_id = run_id;
        self.task_id = task_id;
        self
    }

    fn resolve(&self, path: &str) -> Result<PathBuf> {
        let candidate = Path::new(path);
        if !self.policy.path_allowed(&self.root, candidate) {
            return Err(MajstackError::Policy(format!(
                "path '{path}' is outside the allowed boundary"
            )));
        }
        let joined = if candidate.is_absolute() {
            candidate.to_path_buf()
        } else {
            self.root.join(candidate)
        };
        Ok(joined)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool: String,
    pub ok: bool,
    pub output: String,
    pub duration_ms: u64,
    pub permission: PermissionLevel,
}

pub trait Tool: Send + Sync {
    fn spec(&self) -> &ToolSpec;
    fn call(&self, ctx: &ToolContext, input: &Value) -> Result<String>;
}

pub struct ToolRegistry {
    tools: BTreeMap<String, Arc<dyn Tool>>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::builtin()
    }
}

impl ToolRegistry {
    pub fn builtin() -> Self {
        let mut registry = ToolRegistry {
            tools: BTreeMap::new(),
        };
        registry.register(Arc::new(ReadFile));
        registry.register(Arc::new(WriteFile));
        registry.register(Arc::new(ListDir));
        registry.register(Arc::new(Search));
        registry.register(Arc::new(Shell));
        registry.register(Arc::new(Which));
        registry.register(Arc::new(HttpGet));
        registry.register(Arc::new(GitTool));
        registry
    }

    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.spec().name.clone(), tool);
    }

    pub fn names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    pub fn call(&self, ctx: &ToolContext, name: &str, input: &Value) -> Result<ToolResult> {
        let tool = self
            .get(name)
            .ok_or_else(|| MajstackError::NotFound(format!("unknown tool '{name}'")))?;
        let spec = tool.spec().clone();
        let started = Instant::now();
        let result = if spec.permission == PermissionLevel::ReadOnly {
            tool.call(ctx, input)
        } else {
            ctx.policy
                .require(spec.permission)
                .and_then(|_| tool.call(ctx, input))
        };
        let duration_ms = started.elapsed().as_millis() as u64;
        let ok = result.is_ok();
        if spec.audit {
            let _ = ctx.store.insert_tool_call(
                ctx.run_id.as_deref(),
                ctx.task_id.as_deref(),
                name,
                Some(&truncate(&input.to_string(), 2000)),
                result
                    .as_ref()
                    .ok()
                    .map(|output| truncate(output, 2000))
                    .as_deref(),
                Some(spec.permission.as_str()),
                if ok { "ok" } else { "error" },
                Some(duration_ms as i64),
            );
        }
        match result {
            Ok(output) => Ok(ToolResult {
                tool: name.to_string(),
                ok: true,
                output,
                duration_ms,
                permission: spec.permission,
            }),
            Err(error) => Err(error),
        }
    }
}

fn truncate(text: &str, limit: usize) -> String {
    if text.len() <= limit {
        text.to_string()
    } else {
        format!("{}... [{} bytes]", &text[..limit], text.len())
    }
}

fn input_str(input: &Value, key: &str) -> Result<String> {
    input
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| MajstackError::Invalid(format!("missing string field '{key}'")))
}

struct ReadFile;
impl Tool for ReadFile {
    fn spec(&self) -> &ToolSpec {
        static SPEC: std::sync::OnceLock<ToolSpec> = std::sync::OnceLock::new();
        SPEC.get_or_init(|| {
            ToolSpec::new(
                "read_file",
                "Read a UTF-8 file inside the workspace.",
                PermissionLevel::ReadOnly,
            )
        })
    }
    fn call(&self, ctx: &ToolContext, input: &Value) -> Result<String> {
        let path = ctx.resolve(&input_str(input, "path")?)?;
        std::fs::read_to_string(&path)
            .map_err(|error| MajstackError::Tool(format!("read {}: {error}", path.display())))
    }
}

struct WriteFile;
impl Tool for WriteFile {
    fn spec(&self) -> &ToolSpec {
        static SPEC: std::sync::OnceLock<ToolSpec> = std::sync::OnceLock::new();
        SPEC.get_or_init(|| {
            ToolSpec::new(
                "write_file",
                "Write a file inside the workspace.",
                PermissionLevel::Standard,
            )
        })
    }
    fn call(&self, ctx: &ToolContext, input: &Value) -> Result<String> {
        let path = ctx.resolve(&input_str(input, "path")?)?;
        let content = input_str(input, "content")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, &content)?;
        Ok(format!(
            "wrote {} bytes to {}",
            content.len(),
            path.display()
        ))
    }
}

struct ListDir;
impl Tool for ListDir {
    fn spec(&self) -> &ToolSpec {
        static SPEC: std::sync::OnceLock<ToolSpec> = std::sync::OnceLock::new();
        SPEC.get_or_init(|| {
            ToolSpec::new(
                "list_dir",
                "List directory entries.",
                PermissionLevel::ReadOnly,
            )
        })
    }
    fn call(&self, ctx: &ToolContext, input: &Value) -> Result<String> {
        let relative = input.get("path").and_then(Value::as_str).unwrap_or(".");
        let dir = ctx.resolve(relative)?;
        let mut entries: Vec<String> = std::fs::read_dir(&dir)?
            .flatten()
            .map(|entry| {
                let suffix = if entry.path().is_dir() { "/" } else { "" };
                format!("{}{suffix}", entry.file_name().to_string_lossy())
            })
            .collect();
        entries.sort();
        Ok(entries.join("\n"))
    }
}

struct Search;
impl Tool for Search {
    fn spec(&self) -> &ToolSpec {
        static SPEC: std::sync::OnceLock<ToolSpec> = std::sync::OnceLock::new();
        SPEC.get_or_init(|| {
            ToolSpec::new(
                "search",
                "Regex search across workspace files.",
                PermissionLevel::ReadOnly,
            )
        })
    }
    fn call(&self, ctx: &ToolContext, input: &Value) -> Result<String> {
        let pattern = input_str(input, "pattern")?;
        let matcher = regex::Regex::new(&pattern)
            .map_err(|error| MajstackError::Invalid(error.to_string()))?;
        let mut hits = Vec::new();
        for entry in walkdir::WalkDir::new(&ctx.root).into_iter().flatten() {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            let text_path = path.to_string_lossy();
            if text_path.contains(".git")
                || text_path.contains(".majstack")
                || text_path.contains("target")
            {
                continue;
            }
            if let Ok(content) = std::fs::read_to_string(path) {
                for (number, line) in content.lines().enumerate() {
                    if matcher.is_match(line) {
                        hits.push(format!(
                            "{}:{}: {}",
                            path.display(),
                            number + 1,
                            line.trim()
                        ));
                        if hits.len() >= 200 {
                            return Ok(hits.join("\n"));
                        }
                    }
                }
            }
        }
        Ok(hits.join("\n"))
    }
}

struct Shell;
impl Tool for Shell {
    fn spec(&self) -> &ToolSpec {
        static SPEC: std::sync::OnceLock<ToolSpec> = std::sync::OnceLock::new();
        SPEC.get_or_init(|| {
            ToolSpec::new(
                "shell",
                "Run a shell command in the workspace.",
                PermissionLevel::Autonomous,
            )
        })
    }
    fn call(&self, ctx: &ToolContext, input: &Value) -> Result<String> {
        let command = input_str(input, "command")?;
        ctx.policy.authorize_command(&command)?;
        let output = run_shell(&command, &ctx.root, 600)?;
        if output.timed_out {
            return Err(MajstackError::Timeout(format!(
                "command timed out: {command}"
            )));
        }
        if output.code != 0 {
            return Err(MajstackError::Tool(format!(
                "command exited {}: {}",
                output.code,
                truncate(&output.combined(), 4000)
            )));
        }
        Ok(output.combined())
    }
}

struct Which;
impl Tool for Which {
    fn spec(&self) -> &ToolSpec {
        static SPEC: std::sync::OnceLock<ToolSpec> = std::sync::OnceLock::new();
        SPEC.get_or_init(|| {
            ToolSpec::new(
                "which",
                "Check whether an executable is on PATH.",
                PermissionLevel::ReadOnly,
            )
        })
    }
    fn call(&self, _ctx: &ToolContext, input: &Value) -> Result<String> {
        let program = input_str(input, "program")?;
        Ok(if majstack_execution::command_exists(&program) {
            "found".to_string()
        } else {
            "missing".to_string()
        })
    }
}

struct HttpGet;
impl Tool for HttpGet {
    fn spec(&self) -> &ToolSpec {
        static SPEC: std::sync::OnceLock<ToolSpec> = std::sync::OnceLock::new();
        SPEC.get_or_init(|| {
            ToolSpec::new(
                "http_get",
                "HTTP GET a URL and return the body.",
                PermissionLevel::Standard,
            )
        })
    }
    fn call(&self, _ctx: &ToolContext, input: &Value) -> Result<String> {
        let url = input_str(input, "url")?;
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|error| MajstackError::Network(error.to_string()))?;
        let response = client
            .get(&url)
            .send()
            .map_err(|error| MajstackError::Network(error.to_string()))?;
        let status = response.status();
        let body = response.text().unwrap_or_default();
        Ok(format!("status {status}\n{}", truncate(&body, 8000)))
    }
}

struct GitTool;
impl Tool for GitTool {
    fn spec(&self) -> &ToolSpec {
        static SPEC: std::sync::OnceLock<ToolSpec> = std::sync::OnceLock::new();
        SPEC.get_or_init(|| ToolSpec::new("git", "Run a git subcommand.", PermissionLevel::Safe))
    }
    fn call(&self, ctx: &ToolContext, input: &Value) -> Result<String> {
        let args = input
            .get("args")
            .and_then(Value::as_array)
            .map(|array| {
                array
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .ok_or_else(|| MajstackError::Invalid("missing 'args' array".into()))?;
        let command = format!("git {}", args.join(" "));
        ctx.policy.authorize_command(&command)?;
        let output = majstack_git_call(&ctx.root, &args)?;
        Ok(output)
    }
}

fn majstack_git_call(root: &Path, args: &[String]) -> Result<String> {
    let spec = majstack_execution::ProcessSpec::new("git", args.to_vec(), root).timeout(120);
    let output = majstack_execution::run_process(&spec)?;
    Ok(format!("{}\n{}", output.stdout, output.stderr))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(root: &Path) -> ToolContext {
        let store = Arc::new(Store::memory().unwrap());
        ToolContext::new(root, Policy::permissive(), store)
    }

    #[test]
    fn writes_and_reads_within_root() {
        let dir = majstack_testing_dir();
        let ctx = context(&dir);
        let registry = ToolRegistry::builtin();
        registry
            .call(
                &ctx,
                "write_file",
                &serde_json::json!({ "path": "notes/a.txt", "content": "hello" }),
            )
            .unwrap();
        let output = registry
            .call(
                &ctx,
                "read_file",
                &serde_json::json!({ "path": "notes/a.txt" }),
            )
            .unwrap();
        assert_eq!(output.output, "hello");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_path_traversal() {
        let dir = majstack_testing_dir();
        let ctx = context(&dir);
        let registry = ToolRegistry::builtin();
        assert!(registry
            .call(
                &ctx,
                "read_file",
                &serde_json::json!({ "path": "../../etc/passwd" })
            )
            .is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn audits_tool_calls() {
        let dir = majstack_testing_dir();
        let ctx = context(&dir);
        let registry = ToolRegistry::builtin();
        registry
            .call(&ctx, "list_dir", &serde_json::json!({ "path": "." }))
            .unwrap();
        assert!(ctx.store.count("tool_calls").unwrap() >= 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn policy_blocks_destructive_shell() {
        let dir = majstack_testing_dir();
        let store = Arc::new(Store::memory().unwrap());
        let ctx = ToolContext::new(
            &dir,
            Policy::new(PermissionLevel::Autonomous).with_careful(true),
            store,
        );
        let registry = ToolRegistry::builtin();
        assert!(registry
            .call(&ctx, "shell", &serde_json::json!({ "command": "rm -rf /" }))
            .is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn majstack_testing_dir() -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let dir =
            std::env::temp_dir().join(format!("majstack-tools-{}-{nanos}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
