use crate::status::ReviewVerdict;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Sigils {
    pub done: bool,
    pub done_summary: Option<String>,
    pub failed: bool,
    pub failed_reason: Option<String>,
    pub promise: bool,
    pub blocked: Option<String>,
    pub next_model: Option<String>,
    pub knowledge: Vec<String>,
    pub journal: Vec<String>,
    pub learn: Vec<String>,
    pub verify_pass: bool,
    pub verify_fail: Vec<String>,
    pub verdict: Option<ReviewVerdict>,
    pub winner: Option<usize>,
}

impl Sigils {
    pub fn is_success(&self) -> bool {
        (self.done || self.verify_pass) && self.blocked.is_none() && !self.failed
    }

    pub fn is_blocked(&self) -> bool {
        self.blocked.is_some()
    }

    pub fn failed_or_blocked(&self) -> bool {
        self.failed || self.blocked.is_some() || !self.verify_fail.is_empty()
    }

    pub fn patterns(&self) -> Vec<String> {
        self.learn
            .iter()
            .filter_map(|line| {
                let trimmed = line.trim();
                let upper = trimmed.to_ascii_uppercase();
                if upper.starts_with("PATTERN:") {
                    Some(trimmed[8..].trim().to_string())
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn lessons(&self) -> Vec<String> {
        self.learn
            .iter()
            .filter_map(|line| {
                let trimmed = line.trim();
                let upper = trimmed.to_ascii_uppercase();
                if !upper.starts_with("PATTERN:") && !trimmed.is_empty() {
                    Some(trimmed.to_string())
                } else {
                    None
                }
            })
            .collect()
    }
}

fn between_all(text: &str, open: &str, close: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find(open) {
        let after = &rest[start + open.len()..];
        match after.find(close) {
            Some(end) => {
                out.push(after[..end].trim().to_string());
                rest = &after[end + close.len()..];
            }
            None => break,
        }
    }
    out
}

fn first_between(text: &str, open: &str, close: &str) -> Option<String> {
    between_all(text, open, close).into_iter().next()
}

pub fn parse(output: &str) -> Sigils {
    let mut sigils = Sigils::default();

    let done_tags = between_all(output, "<task-done>", "</task-done>");
    if !done_tags.is_empty() {
        sigils.done = true;
        sigils.done_summary = Some(done_tags[0].clone());
    }
    let failed_tags = between_all(output, "<task-failed>", "</task-failed>");
    if !failed_tags.is_empty() {
        sigils.failed = true;
        sigils.failed_reason = Some(failed_tags[0].clone());
    }
    if output.contains("<promise>") {
        sigils.promise = true;
    }
    if let Some(value) = first_between(output, "<blocked>", "</blocked>") {
        sigils.blocked = Some(value);
    }
    if let Some(value) = first_between(output, "<next-model>", "</next-model>") {
        sigils.next_model = Some(value);
    }
    sigils.knowledge = between_all(output, "<knowledge>", "</knowledge>");
    sigils.journal = between_all(output, "<journal>", "</journal>");
    if output.contains("<verify-pass/>") || output.contains("<verify-pass>") {
        sigils.verify_pass = true;
    }
    sigils.verify_fail = between_all(output, "<verify-fail>", "</verify-fail>");

    for raw in output.lines() {
        let line = raw.trim();
        if let Some(rest) = line.strip_prefix("MAJSTACK_DONE:") {
            sigils.done = true;
            let summary = rest.trim();
            if !summary.is_empty() {
                sigils.done_summary = Some(summary.to_string());
            }
        } else if let Some(rest) = line.strip_prefix("MAJSTACK_BLOCKED:") {
            sigils.blocked = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("MAJSTACK_LEARN:") {
            let lesson = rest.trim();
            if !lesson.is_empty() {
                sigils.learn.push(lesson.to_string());
            }
        } else if let Some(rest) = line.strip_prefix("VERDICT:") {
            let value = rest.trim().to_ascii_uppercase();
            if value.starts_with("APPROVE") {
                sigils.verdict = Some(ReviewVerdict::Approve);
            } else if value.starts_with("REJECT") {
                sigils.verdict = Some(ReviewVerdict::Reject);
            } else if value.starts_with("NEEDS") {
                sigils.verdict = Some(ReviewVerdict::NeedsChanges);
            }
        } else if let Some(rest) = line.strip_prefix("WINNER:") {
            if let Ok(number) = rest.trim().parse::<usize>() {
                sigils.winner = Some(number);
            }
        }
    }

    sigils
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_completion_sigils() {
        let parsed = parse("<task-done>t-1</task-done>\nMAJSTACK_DONE: made it work");
        assert!(parsed.done);
        assert_eq!(parsed.done_summary.as_deref(), Some("made it work"));
        let only_tag = parse("<task-done>t-1</task-done>");
        assert_eq!(only_tag.done_summary.as_deref(), Some("t-1"));
    }

    #[test]
    fn parses_blocked_and_next_model() {
        let parsed = parse("<blocked>missing creds</blocked>\n<next-model>opus</next-model>");
        assert_eq!(parsed.blocked.as_deref(), Some("missing creds"));
        assert_eq!(parsed.next_model.as_deref(), Some("opus"));
    }

    #[test]
    fn parses_verdict_and_winner() {
        let parsed = parse("VERDICT: REJECT: bad tests\nWINNER: 2");
        assert_eq!(parsed.verdict, Some(ReviewVerdict::Reject));
        assert_eq!(parsed.winner, Some(2));
    }

    #[test]
    fn parses_learnings_and_patterns() {
        let parsed =
            parse("MAJSTACK_LEARN: PATTERN: use argon2\nMAJSTACK_LEARN: tests live in tests/");
        assert_eq!(parsed.patterns(), vec!["use argon2".to_string()]);
        assert_eq!(parsed.lessons(), vec!["tests live in tests/".to_string()]);
    }

    #[test]
    fn parses_verification_and_knowledge() {
        let parsed =
            parse("<verify-fail>step 2 failed</verify-fail><knowledge>db is postgres</knowledge>");
        assert_eq!(parsed.verify_fail, vec!["step 2 failed".to_string()]);
        assert_eq!(parsed.knowledge, vec!["db is postgres".to_string()]);
    }
}
