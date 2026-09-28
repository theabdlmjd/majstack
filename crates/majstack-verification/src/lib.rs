use majstack_core::{Result, ReviewVerdict, Severity, VerificationStatus};
use majstack_execution::run_shell;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationCheck {
    pub name: String,
    pub command: String,
    pub kind: String,
    pub required: bool,
}

impl VerificationCheck {
    pub fn new(name: &str, command: &str, kind: &str) -> Self {
        VerificationCheck {
            name: name.to_string(),
            command: command.to_string(),
            kind: kind.to_string(),
            required: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub name: String,
    pub kind: String,
    pub status: VerificationStatus,
    pub command: String,
    pub output: String,
    pub duration_ms: u64,
}

impl VerificationResult {
    pub fn passed(&self) -> bool {
        self.status == VerificationStatus::Passed
    }
}

pub fn detect_checks(root: &Path, verify_override: Option<&str>) -> Vec<VerificationCheck> {
    let mut checks = Vec::new();
    if let Some(command) = verify_override {
        if !command.is_empty() {
            checks.push(VerificationCheck::new("verify", command, "verify"));
        }
    }
    let has = |name: &str| root.join(name).exists();
    if has("package.json") {
        checks.push(VerificationCheck::new("test", "npm test --silent", "test"));
        if has("tsconfig.json") {
            checks.push(VerificationCheck::new(
                "types",
                "npx --no-install tsc --noEmit",
                "types",
            ));
        }
        if has(".eslintrc")
            || has(".eslintrc.json")
            || has("eslint.config.js")
            || has("eslint.config.mjs")
        {
            checks.push(VerificationCheck::new(
                "lint",
                "npx --no-install eslint .",
                "lint",
            ));
        }
    }
    if has("pyproject.toml") || has("setup.py") || has("requirements.txt") {
        checks.push(VerificationCheck::new(
            "test",
            "python -m pytest -q",
            "test",
        ));
        if majstack_execution::command_exists("ruff") {
            checks.push(VerificationCheck::new("lint", "ruff check .", "lint"));
        }
    }
    if has("Cargo.toml") {
        checks.push(VerificationCheck::new("test", "cargo test -q", "test"));
        checks.push(VerificationCheck::new("lint", "cargo clippy -q", "lint"));
    }
    if has("go.mod") {
        checks.push(VerificationCheck::new("test", "go test ./...", "test"));
        checks.push(VerificationCheck::new("lint", "go vet ./...", "lint"));
    }
    checks
}

pub fn run_check(
    root: &Path,
    check: &VerificationCheck,
    timeout_seconds: u64,
) -> Result<VerificationResult> {
    let output = run_shell(&check.command, root, timeout_seconds)?;
    let status = if output.timed_out {
        VerificationStatus::Error
    } else if output.code == 0 {
        VerificationStatus::Passed
    } else {
        VerificationStatus::Failed
    };
    Ok(VerificationResult {
        name: check.name.clone(),
        kind: check.kind.clone(),
        status,
        command: check.command.clone(),
        output: output.combined(),
        duration_ms: output.duration_ms,
    })
}

pub fn run_all(
    root: &Path,
    checks: &[VerificationCheck],
    timeout_seconds: u64,
) -> Vec<VerificationResult> {
    checks
        .iter()
        .map(|check| {
            run_check(root, check, timeout_seconds).unwrap_or_else(|error| VerificationResult {
                name: check.name.clone(),
                kind: check.kind.clone(),
                status: VerificationStatus::Error,
                command: check.command.clone(),
                output: error.to_string(),
                duration_ms: 0,
            })
        })
        .collect()
}

pub fn all_passed(results: &[VerificationResult]) -> bool {
    results.iter().all(|result| result.passed())
}

pub fn parse_verdict(output: &str) -> Option<ReviewVerdict> {
    let sigils = majstack_core::sigils::parse(output);
    if let Some(verdict) = sigils.verdict {
        return Some(verdict);
    }
    let upper = output.to_ascii_uppercase();
    if upper.contains("VERDICT: APPROVE") || upper.contains("APPROVE") {
        Some(ReviewVerdict::Approve)
    } else if upper.contains("VERDICT: REJECT") || upper.contains("REJECT") {
        Some(ReviewVerdict::Reject)
    } else {
        None
    }
}

pub fn severity_from(value: &str) -> Severity {
    Severity::parse(&value.to_ascii_lowercase()).unwrap_or(Severity::Minor)
}

pub const REVIEW_LENSES: &[&str] = &[
    "correctness and edge cases",
    "security and abuse",
    "code quality: simplicity, dead code, naming",
    "performance and scaling",
    "tests: are they proving behavior",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_nothing_for_empty_dir() {
        let dir = std::env::temp_dir();
        assert!(detect_checks(&dir, None).is_empty());
    }

    #[test]
    fn verify_override_prepends() {
        let dir = std::env::temp_dir();
        let checks = detect_checks(&dir, Some("true"));
        assert_eq!(checks[0].name, "verify");
    }

    #[test]
    fn parses_verdicts() {
        assert_eq!(
            parse_verdict("VERDICT: APPROVE"),
            Some(ReviewVerdict::Approve)
        );
        assert_eq!(
            parse_verdict("VERDICT: REJECT: nope"),
            Some(ReviewVerdict::Reject)
        );
        assert_eq!(parse_verdict("all good"), None);
    }

    #[test]
    fn runs_check() {
        let dir = std::env::temp_dir();
        let check = VerificationCheck::new("ok", "cmd /C exit 0", "test");
        let result = run_check(&dir, &check, 30);
        if cfg!(windows) {
            assert!(result.unwrap().passed());
        }
    }
}
