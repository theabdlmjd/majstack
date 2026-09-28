use majstack_core::{MajstackError, Result};
use majstack_execution::{command_exists, run_process, ProcessSpec};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrSpec {
    pub title: String,
    pub body: String,
    pub base: String,
    pub draft: bool,
}

impl PrSpec {
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        PrSpec {
            title: title.into(),
            body: body.into(),
            base: "main".to_string(),
            draft: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrResult {
    pub number: u64,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CheckState {
    Passing,
    Failing,
    Pending,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecksSummary {
    pub state: CheckState,
    pub failing: Vec<String>,
    pub pending: Vec<String>,
    pub passing: Vec<String>,
}

impl ChecksSummary {
    pub fn green(&self) -> bool {
        self.state == CheckState::Passing
    }
}

pub struct GitHub {
    root: PathBuf,
    base: String,
    draft: bool,
    token: Option<String>,
}

impl GitHub {
    pub fn new(root: impl Into<PathBuf>, base: impl Into<String>, draft: bool) -> Self {
        GitHub {
            root: root.into(),
            base: base.into(),
            draft,
            token: std::env::var("GITHUB_TOKEN")
                .ok()
                .or_else(|| std::env::var("GH_TOKEN").ok()),
        }
    }

    pub fn cli_available(&self) -> bool {
        command_exists("gh")
    }

    pub fn available(&self) -> bool {
        self.cli_available() || self.token.is_some()
    }

    pub fn remote_slug(&self) -> Option<String> {
        let output = run_process(&ProcessSpec::new(
            "git",
            vec!["remote".into(), "get-url".into(), "origin".into()],
            &self.root,
        ))
        .ok()?;
        if !output.success() {
            return None;
        }
        parse_remote_url(output.stdout.trim())
    }

    pub fn create_pr(&self, spec: &PrSpec) -> Result<PrResult> {
        if self.cli_available() {
            return self.create_pr_cli(spec);
        }
        self.create_pr_rest(spec)
    }

    fn create_pr_cli(&self, spec: &PrSpec) -> Result<PrResult> {
        let body_file = self.root.join(".majstack").join("pr-body.md");
        if let Some(parent) = body_file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&body_file, &spec.body)?;
        let mut args = vec![
            "pr".to_string(),
            "create".to_string(),
            "--title".to_string(),
            spec.title.clone(),
            "--body-file".to_string(),
            body_file.to_string_lossy().to_string(),
            "--base".to_string(),
            spec.base.clone(),
        ];
        if spec.draft {
            args.push("--draft".to_string());
        }
        let output = run_process(&ProcessSpec::new("gh", args, &self.root).timeout(120))?;
        if !output.success() {
            return Err(MajstackError::Network(format!(
                "gh pr create failed: {}",
                output.combined()
            )));
        }
        let url = output
            .stdout
            .lines()
            .find(|line| line.starts_with("http"))
            .unwrap_or("")
            .trim()
            .to_string();
        let number = url
            .rsplit('/')
            .next()
            .and_then(|segment| segment.parse::<u64>().ok())
            .unwrap_or(0);
        Ok(PrResult { number, url })
    }

    fn create_pr_rest(&self, spec: &PrSpec) -> Result<PrResult> {
        let token = self
            .token
            .as_ref()
            .ok_or_else(|| MajstackError::Policy("no GitHub token or gh CLI available".into()))?;
        let slug = self
            .remote_slug()
            .ok_or_else(|| MajstackError::NotFound("no origin remote".into()))?;
        let client = reqwest::blocking::Client::new();
        let response = client
            .post(format!("https://api.github.com/repos/{slug}/pulls"))
            .header("User-Agent", "majstack-agent")
            .header("Authorization", format!("Bearer {token}"))
            .json(&serde_json::json!({
                "title": spec.title,
                "body": spec.body,
                "base": spec.base,
                "draft": spec.draft,
            }))
            .send()
            .map_err(|error| MajstackError::Network(error.to_string()))?;
        let status = response.status();
        let value: serde_json::Value = response.json().unwrap_or(serde_json::Value::Null);
        if !status.is_success() {
            return Err(MajstackError::Network(format!(
                "GitHub API {status}: {value}"
            )));
        }
        Ok(PrResult {
            number: value
                .get("number")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0),
            url: value
                .get("html_url")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string(),
        })
    }

    pub fn checks(&self) -> Result<ChecksSummary> {
        if !self.cli_available() {
            return Err(MajstackError::Policy(
                "gh CLI is required for checks".into(),
            ));
        }
        let output = run_process(
            &ProcessSpec::new("gh", vec!["pr".into(), "checks".into()], &self.root).timeout(120),
        )?;
        Ok(parse_checks(&output.combined()))
    }

    pub fn pr_comments(&self) -> Result<String> {
        if !self.cli_available() {
            return Err(MajstackError::Policy(
                "gh CLI is required for PR comments".into(),
            ));
        }
        let output = run_process(
            &ProcessSpec::new(
                "gh",
                vec!["pr".into(), "view".into(), "--comments".into()],
                &self.root,
            )
            .timeout(120),
        )?;
        Ok(output.combined())
    }

    pub fn comment_issue(&self, number: u64, body: &str) -> Result<()> {
        let output = run_process(
            &ProcessSpec::new(
                "gh",
                vec![
                    "issue".to_string(),
                    "comment".to_string(),
                    number.to_string(),
                    "--body".to_string(),
                    body.to_string(),
                ],
                &self.root,
            )
            .timeout(120),
        )?;
        if !output.success() {
            return Err(MajstackError::Network(format!(
                "gh issue comment failed: {}",
                output.combined()
            )));
        }
        Ok(())
    }

    pub fn create_issue(&self, title: &str, body: &str) -> Result<String> {
        let output = run_process(
            &ProcessSpec::new(
                "gh",
                vec![
                    "issue".to_string(),
                    "create".to_string(),
                    "--title".to_string(),
                    title.to_string(),
                    "--body".to_string(),
                    body.to_string(),
                ],
                &self.root,
            )
            .timeout(120),
        )?;
        if !output.success() {
            return Err(MajstackError::Network(format!(
                "gh issue create failed: {}",
                output.combined()
            )));
        }
        Ok(output.stdout.trim().to_string())
    }

    pub fn merge_pr(&self, squash: bool) -> Result<bool> {
        let mut args = vec!["pr".to_string(), "merge".to_string()];
        if squash {
            args.push("--squash".to_string());
        }
        args.push("--delete-branch".to_string());
        let output = run_process(&ProcessSpec::new("gh", args, &self.root).timeout(180))?;
        Ok(output.success())
    }

    pub fn config_base(&self) -> &str {
        &self.base
    }

    pub fn config_draft(&self) -> bool {
        self.draft
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

pub fn parse_remote_url(url: &str) -> Option<String> {
    let url = url.trim();
    if let Some(rest) = url.strip_prefix("git@github.com:") {
        return Some(rest.trim_end_matches(".git").to_string());
    }
    if let Some(rest) = url.strip_prefix("https://github.com/") {
        return Some(rest.trim_end_matches(".git").to_string());
    }
    if let Some(rest) = url.strip_prefix("ssh://git@github.com/") {
        return Some(rest.trim_end_matches(".git").to_string());
    }
    None
}

pub fn parse_checks(output: &str) -> ChecksSummary {
    let mut failing = Vec::new();
    let mut pending = Vec::new();
    let mut passing = Vec::new();
    for line in output.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 2 {
            continue;
        }
        let name = fields[0].to_string();
        let state = fields[1].to_ascii_lowercase();
        if state.starts_with("fail") || state.starts_with('x') {
            failing.push(name);
        } else if state.starts_with("pending") || state.starts_with('*') {
            pending.push(name);
        } else if state.starts_with("pass") || state.starts_with('v') {
            passing.push(name);
        }
    }
    let state = if !failing.is_empty() {
        CheckState::Failing
    } else if !pending.is_empty() {
        CheckState::Pending
    } else if !passing.is_empty() {
        CheckState::Passing
    } else {
        CheckState::Unknown
    };
    ChecksSummary {
        state,
        failing,
        pending,
        passing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_remote_urls() {
        assert_eq!(
            parse_remote_url("git@github.com:acme/app.git").as_deref(),
            Some("acme/app")
        );
        assert_eq!(
            parse_remote_url("https://github.com/acme/app.git").as_deref(),
            Some("acme/app")
        );
        assert_eq!(parse_remote_url("https://gitlab.com/acme/app.git"), None);
    }

    #[test]
    fn parses_checks_output() {
        let output = "build\tpass\t1m\ntest\tfail\t2m\nlint\tpending\t0s";
        let summary = parse_checks(output);
        assert_eq!(summary.state, CheckState::Failing);
        assert!(summary.failing.contains(&"test".to_string()));
        assert!(summary.pending.contains(&"lint".to_string()));
        assert!(summary.passing.contains(&"build".to_string()));
        assert!(!summary.green());
    }

    #[test]
    fn green_when_all_pass() {
        let summary = parse_checks("build\tpass\t1m\ntest\tpass\t2m");
        assert!(summary.green());
    }
}
