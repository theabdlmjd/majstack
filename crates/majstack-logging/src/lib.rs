use regex::Regex;
use std::sync::OnceLock;
use tracing_subscriber::fmt;
use tracing_subscriber::EnvFilter;

pub fn init(level: &str, json: bool) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));
    if json {
        let _ = fmt().json().with_env_filter(filter).try_init();
    } else {
        let _ = fmt().with_env_filter(filter).try_init();
    }
}

fn secret_patterns() -> &'static [(Regex, &'static str)] {
    static PATTERNS: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        vec![
            (Regex::new(r"(?i)(sk-[a-z0-9]{16,})").unwrap(), "sk-REDACTED"),
            (Regex::new(r"(?i)(ghp_[a-z0-9]{20,})").unwrap(), "ghp_REDACTED"),
            (Regex::new(r"(?i)(github_pat_[a-z0-9_]{20,})").unwrap(), "github_pat_REDACTED"),
            (Regex::new(r"(?i)(aws_secret_access_key\s*[=:]\s*)\S+").unwrap(), "${1}REDACTED"),
            (Regex::new(r"(?i)(authorization:\s*bearer\s+)\S+").unwrap(), "${1}REDACTED"),
            (
                Regex::new(r"(?i)(-----BEGIN [A-Z ]*PRIVATE KEY-----)[\s\S]*?(-----END [A-Z ]*PRIVATE KEY-----)")
                    .unwrap(),
                "${1} REDACTED ${2}",
            ),
            (
                Regex::new(r"(?i)\b(password|passwd|secret|token|api[_-]?key)\b\s*[=:]\s*[^\s,;]+")
                    .unwrap(),
                "${1}=REDACTED",
            ),
        ]
    })
}

pub fn redact(text: &str) -> String {
    let mut out = text.to_string();
    for (pattern, replacement) in secret_patterns() {
        out = pattern.replace_all(&out, *replacement).to_string();
    }
    out
}

pub fn redact_value(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(text) => {
            *text = redact(text);
        }
        serde_json::Value::Array(items) => {
            for item in items.iter_mut() {
                redact_value(item);
            }
        }
        serde_json::Value::Object(map) => {
            for (_, item) in map.iter_mut() {
                redact_value(item);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_common_secrets() {
        assert!(!redact("token sk-abcdefghijklmnop1234").contains("sk-abcdefghijklmnop1234"));
        assert!(!redact("ghp_abcdefghijklmnopqrstuvwx").contains("ghp_abcdefghijklmnopqrstuvwx"));
        assert!(redact("hello world").contains("hello world"));
    }
}
