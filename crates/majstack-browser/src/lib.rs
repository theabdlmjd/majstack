use base64::Engine;
use majstack_core::{MajstackError, Result};
use serde_json::{json, Value};
use std::io::Write;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use tungstenite::stream::MaybeTlsStream;
use tungstenite::Message;
use tungstenite::WebSocket;

pub fn candidate_browsers() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(env) = std::env::var("CHROME_PATH") {
        candidates.push(PathBuf::from(env));
    }
    for name in [
        "chrome",
        "google-chrome",
        "google-chrome-stable",
        "chromium",
        "chromium-browser",
        "msedge",
        "brave",
    ] {
        if let Ok(path) = which_binary(name) {
            candidates.push(path);
        }
    }
    if cfg!(windows) {
        for base in [
            "C:/Program Files/Google/Chrome/Application/chrome.exe",
            "C:/Program Files (x86)/Google/Chrome/Application/chrome.exe",
            "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
            "C:/Program Files/Microsoft/Edge/Application/msedge.exe",
        ] {
            candidates.push(PathBuf::from(base));
        }
    }
    candidates
        .into_iter()
        .filter(|path| path.exists())
        .collect()
}

fn which_binary(name: &str) -> std::result::Result<PathBuf, ()> {
    majstack_execution::which_path(name).ok_or(())
}

pub fn find_browser() -> Option<PathBuf> {
    candidate_browsers().into_iter().next()
}

fn free_port() -> Result<u16> {
    let listener =
        TcpListener::bind("127.0.0.1:0").map_err(|error| MajstackError::Tool(error.to_string()))?;
    let port = listener
        .local_addr()
        .map_err(|error| MajstackError::Tool(error.to_string()))?
        .port();
    drop(listener);
    Ok(port)
}

pub struct Cdp {
    socket: WebSocket<MaybeTlsStream<std::net::TcpStream>>,
    next_id: u64,
    pub events: Vec<Value>,
}

impl Cdp {
    pub fn connect(ws_url: &str) -> Result<Self> {
        let (socket, _) = tungstenite::connect(ws_url)
            .map_err(|error| MajstackError::Tool(format!("cdp connect: {error}")))?;
        Ok(Cdp {
            socket,
            next_id: 1,
            events: Vec::new(),
        })
    }

    pub fn call(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id;
        self.next_id += 1;
        let payload = json!({ "id": id, "method": method, "params": params }).to_string();
        self.socket
            .send(Message::Text(payload))
            .map_err(|error| MajstackError::Tool(format!("cdp send: {error}")))?;
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if Instant::now() > deadline {
                return Err(MajstackError::Timeout(format!("cdp {method} timed out")));
            }
            let message = self
                .socket
                .read()
                .map_err(|error| MajstackError::Tool(format!("cdp read: {error}")))?;
            let text = match message {
                Message::Text(text) => text,
                Message::Binary(bytes) => String::from_utf8_lossy(&bytes).to_string(),
                Message::Close(_) => return Err(MajstackError::Tool("cdp closed".into())),
                _ => continue,
            };
            let value: Value = match serde_json::from_str(&text) {
                Ok(value) => value,
                Err(_) => continue,
            };
            if value.get("id").and_then(Value::as_u64) == Some(id) {
                if let Some(error) = value.get("error") {
                    return Err(MajstackError::Tool(format!("cdp {method} error: {error}")));
                }
                return Ok(value.get("result").cloned().unwrap_or(Value::Null));
            }
            self.events.push(value);
        }
    }
}

pub struct BrowserSession {
    pub session_id: String,
    pub engine: String,
    child: Child,
    cdp: Cdp,
    pub screenshot_dir: PathBuf,
    pub artifacts: Vec<String>,
    pub console_errors: Vec<String>,
}

impl BrowserSession {
    pub fn launch(headless: bool, screenshot_dir: impl AsRef<Path>) -> Result<Self> {
        let binary = find_browser().ok_or_else(|| {
            MajstackError::Tool("no Chrome/Edge/Chromium found; set CHROME_PATH".into())
        })?;
        let port = free_port()?;
        let profile =
            std::env::temp_dir().join(format!("majstack-chrome-{}-{}", std::process::id(), port));
        let mut command = Command::new(&binary);
        if headless {
            command.arg("--headless=new");
        }
        command
            .arg("--disable-gpu")
            .arg("--no-first-run")
            .arg("--no-default-browser-check")
            .arg("--remote-allow-origins=*")
            .arg(format!("--remote-debugging-port={port}"))
            .arg(format!("--user-data-dir={}", profile.display()))
            .arg("about:blank")
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let child = command
            .spawn()
            .map_err(|error| MajstackError::Tool(format!("launch browser: {error}")))?;

        let ws_url = wait_for_debugger(port)?;
        let mut cdp = Cdp::connect(&ws_url)?;
        cdp.call("Page.enable", json!({}))?;
        cdp.call("Runtime.enable", json!({}))?;
        cdp.call("Log.enable", json!({}))?;

        let screenshot_dir = screenshot_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&screenshot_dir)?;
        let session_id = majstack_core::ids::new_id("browser");
        Ok(BrowserSession {
            session_id,
            engine: binary.to_string_lossy().to_string(),
            child,
            cdp,
            screenshot_dir,
            artifacts: Vec::new(),
            console_errors: Vec::new(),
        })
    }

    pub fn navigate(&mut self, url: &str) -> Result<u16> {
        let result = self.cdp.call("Page.navigate", json!({ "url": url }))?;
        let error = result.get("errorText").and_then(Value::as_str);
        if let Some(error) = error {
            return Err(MajstackError::Network(format!(
                "navigation failed: {error}"
            )));
        }
        self.drain_events();
        Ok(200)
    }

    fn drain_events(&mut self) {
        for event in self.cdp.events.drain(..) {
            let method = event.get("method").and_then(Value::as_str).unwrap_or("");
            if method == "Log.entryAdded" {
                let entry = event.get("params").and_then(|params| params.get("entry"));
                if let Some(entry) = entry {
                    if entry.get("level").and_then(Value::as_str) == Some("error") {
                        if let Some(text) = entry.get("text").and_then(Value::as_str) {
                            self.console_errors.push(text.to_string());
                        }
                    }
                }
            }
            if method == "Runtime.exceptionThrown" {
                if let Some(text) = event
                    .get("params")
                    .and_then(|params| params.get("exceptionDetails"))
                    .and_then(|details| details.get("text"))
                    .and_then(Value::as_str)
                {
                    self.console_errors.push(text.to_string());
                }
            }
        }
    }

    pub fn evaluate(&mut self, expression: &str) -> Result<Value> {
        let result = self.cdp.call(
            "Runtime.evaluate",
            json!({ "expression": expression, "returnByValue": true, "awaitPromise": true }),
        )?;
        if let Some(details) = result.get("exceptionDetails") {
            return Err(MajstackError::Tool(format!("evaluate error: {details}")));
        }
        Ok(result
            .get("result")
            .and_then(|result| result.get("value"))
            .cloned()
            .unwrap_or(Value::Null))
    }

    pub fn title(&mut self) -> Result<String> {
        Ok(self
            .evaluate("document.title")?
            .as_str()
            .unwrap_or("")
            .to_string())
    }

    pub fn text(&mut self) -> Result<String> {
        Ok(self
            .evaluate("document.body ? document.body.innerText : ''")?
            .as_str()
            .unwrap_or("")
            .to_string())
    }

    pub fn click(&mut self, selector: &str) -> Result<()> {
        let script = format!(
            "(() => {{ const el = document.querySelector({}); if (!el) return false; el.click(); return true; }})()",
            serde_json::to_string(selector)?
        );
        if self.evaluate(&script)?.as_bool() == Some(true) {
            Ok(())
        } else {
            Err(MajstackError::NotFound(format!(
                "selector not found: {selector}"
            )))
        }
    }

    pub fn fill(&mut self, selector: &str, text: &str) -> Result<()> {
        let script = format!(
            "(() => {{ const el = document.querySelector({}); if (!el) return false; el.focus(); el.value = {}; el.dispatchEvent(new Event('input', {{bubbles:true}})); return true; }})()",
            serde_json::to_string(selector)?,
            serde_json::to_string(text)?
        );
        if self.evaluate(&script)?.as_bool() == Some(true) {
            Ok(())
        } else {
            Err(MajstackError::NotFound(format!(
                "selector not found: {selector}"
            )))
        }
    }

    pub fn wait_for_selector(&mut self, selector: &str, timeout_ms: u64) -> Result<()> {
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        let script = format!(
            "document.querySelector({}) !== null",
            serde_json::to_string(selector)?
        );
        loop {
            if self.evaluate(&script)?.as_bool() == Some(true) {
                return Ok(());
            }
            if Instant::now() > deadline {
                return Err(MajstackError::Timeout(format!(
                    "selector not found: {selector}"
                )));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    pub fn set_viewport(&mut self, width: u64, height: u64) -> Result<()> {
        self.cdp.call(
            "Emulation.setDeviceMetricsOverride",
            json!({ "width": width, "height": height, "deviceScaleFactor": 1, "mobile": false }),
        )?;
        Ok(())
    }

    pub fn screenshot(&mut self, name: &str) -> Result<PathBuf> {
        let result = self
            .cdp
            .call("Page.captureScreenshot", json!({ "format": "png" }))?;
        let data = result
            .get("data")
            .and_then(Value::as_str)
            .ok_or_else(|| MajstackError::Tool("screenshot: no data".into()))?;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(data)
            .map_err(|error| MajstackError::Tool(error.to_string()))?;
        let path = self.screenshot_dir.join(format!("{name}.png"));
        let mut file = std::fs::File::create(&path)?;
        file.write_all(&bytes)?;
        self.artifacts.push(path.to_string_lossy().to_string());
        Ok(path)
    }

    pub fn close(mut self) {
        let _ = self.cdp.call("Browser.close", json!({}));
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn wait_for_debugger(port: u16) -> Result<String> {
    let deadline = Instant::now() + Duration::from_secs(20);
    let client = reqwest::blocking::Client::new();
    loop {
        if let Ok(response) = client
            .get(format!("http://127.0.0.1:{port}/json/list"))
            .timeout(Duration::from_secs(2))
            .send()
        {
            if let Ok(targets) = response.json::<Value>() {
                if let Some(target) = targets.as_array().and_then(|list| {
                    list.iter()
                        .find(|target| target.get("type").and_then(Value::as_str) == Some("page"))
                }) {
                    if let Some(url) = target.get("webSocketDebuggerUrl").and_then(Value::as_str) {
                        return Ok(url.to_string());
                    }
                }
            }
        }
        if Instant::now() > deadline {
            return Err(MajstackError::Timeout(
                "browser debugger did not start".into(),
            ));
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_browsers_do_not_panic() {
        let _ = candidate_browsers();
    }

    #[test]
    fn free_port_is_bindable() {
        let port = free_port().unwrap();
        assert!(port > 0);
    }
}
