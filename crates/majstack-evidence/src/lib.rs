#![allow(clippy::too_many_arguments)]

use majstack_core::{clock, ids, Result};
use majstack_execution::{run_process, ProcessSpec};
use majstack_state::Store;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Freshness {
    Fresh,
    Stale,
    Failed,
    Missing,
}

pub fn fingerprint(root: &Path) -> String {
    if !majstack_git::is_repo(root) {
        return "no-git".to_string();
    }
    let index = std::env::temp_dir().join(format!(
        "majstack-index-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let index_file = index.to_string_lossy().to_string();
    let add = ProcessSpec::new(
        "git",
        vec![
            "add".into(),
            "-A".into(),
            "--".into(),
            ".".into(),
            ":(exclude).majstack".into(),
        ],
        root,
    )
    .env("GIT_INDEX_FILE", &index_file)
    .timeout(120);
    let _ = run_process(&add);
    let write = ProcessSpec::new("git", vec!["write-tree".into()], root)
        .env("GIT_INDEX_FILE", &index_file)
        .timeout(120);
    let tree = run_process(&write)
        .ok()
        .filter(|output| output.success())
        .map(|output| output.stdout.trim().to_string());
    let _ = std::fs::remove_file(&index);
    tree.unwrap_or_else(|| "unknown".to_string())
}

pub fn check_evidence(
    store: &Store,
    current_fingerprint: &str,
    labels: &[&str],
) -> Result<BTreeMap<String, Freshness>> {
    let mut out = BTreeMap::new();
    for label in labels {
        let freshness = match store.latest_evidence(label)? {
            None => Freshness::Missing,
            Some(record) => {
                if record.code.unwrap_or(1) != 0 {
                    Freshness::Failed
                } else if record.fingerprint.as_deref() != Some(current_fingerprint) {
                    Freshness::Stale
                } else {
                    Freshness::Fresh
                }
            }
        };
        out.insert(label.to_string(), freshness);
    }
    Ok(out)
}

pub fn record_evidence(
    store: &Store,
    run_id: Option<&str>,
    task_id: Option<&str>,
    label: &str,
    command: &str,
    code: i64,
    fingerprint: &str,
    log_path: Option<&str>,
) -> Result<String> {
    store.insert_evidence(
        run_id,
        task_id,
        label,
        Some(command),
        Some(code),
        Some(fingerprint),
        log_path,
    )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EgressRecord {
    pub ts: i64,
    pub sink: String,
    pub detail: String,
    pub prev: String,
    pub hash: String,
}

fn hash_record(record: &EgressRecord) -> String {
    let mut hasher = Sha256::new();
    hasher.update(record.prev.as_bytes());
    hasher.update(b"|");
    hasher.update(record.sink.as_bytes());
    hasher.update(b"|");
    hasher.update(record.detail.as_bytes());
    hasher.update(b"|");
    hasher.update(record.ts.to_string().as_bytes());
    format!("{:x}", hasher.finalize())
}

pub struct EgressLedger {
    path: PathBuf,
}

impl EgressLedger {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        EgressLedger { path: path.into() }
    }

    pub fn records(&self) -> Vec<EgressRecord> {
        let text = std::fs::read_to_string(&self.path).unwrap_or_default();
        text.lines()
            .filter_map(|line| serde_json::from_str::<EgressRecord>(line).ok())
            .collect()
    }

    pub fn append(&self, sink: &str, detail: &str) -> Result<EgressRecord> {
        let previous = self
            .records()
            .last()
            .map(|record| record.hash.clone())
            .unwrap_or_else(|| "0".repeat(64));
        let mut record = EgressRecord {
            ts: clock::now_ts(),
            sink: sink.to_string(),
            detail: detail.chars().take(300).collect(),
            prev: previous,
            hash: String::new(),
        };
        record.hash = hash_record(&record);
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut line = serde_json::to_string(&record)?;
        line.push('\n');
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        file.write_all(line.as_bytes())?;
        Ok(record)
    }

    pub fn verify(&self) -> (bool, usize) {
        let mut previous = "0".repeat(64);
        let mut count = 0;
        for record in self.records() {
            if record.prev != previous || record.hash != hash_record(&record) {
                return (false, count);
            }
            previous = record.hash;
            count += 1;
        }
        (true, count)
    }
}

pub fn decision_id() -> String {
    ids::new_id("decision")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ledger_detects_tamper() {
        let dir = std::env::temp_dir().join(format!("majstack-ledger-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("egress.jsonl");
        let ledger = EgressLedger::new(&path);
        ledger.append("a", "one").unwrap();
        ledger.append("b", "two").unwrap();
        assert_eq!(ledger.verify(), (true, 2));
        let text = std::fs::read_to_string(&path).unwrap();
        let tampered = text.replacen("one", "ONE", 1);
        std::fs::write(&path, tampered).unwrap();
        assert!(!ledger.verify().0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn evidence_freshness_transitions() {
        let store = Store::memory().unwrap();
        let fp = "abc";
        assert_eq!(
            check_evidence(&store, fp, &["verify"]).unwrap()["verify"],
            Freshness::Missing
        );
        record_evidence(&store, None, None, "verify", "true", 0, fp, None).unwrap();
        assert_eq!(
            check_evidence(&store, fp, &["verify"]).unwrap()["verify"],
            Freshness::Fresh
        );
        assert_eq!(
            check_evidence(&store, "other", &["verify"]).unwrap()["verify"],
            Freshness::Stale
        );
        record_evidence(&store, None, None, "verify", "false", 1, fp, None).unwrap();
        assert_eq!(
            check_evidence(&store, fp, &["verify"]).unwrap()["verify"],
            Freshness::Failed
        );
    }
}
