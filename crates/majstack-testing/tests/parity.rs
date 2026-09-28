use serde_json::Value;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

#[test]
fn required_docs_exist() {
    let root = repo_root();
    for relative in [
        "README.md",
        "NOTICE.md",
        "LICENSE",
        "SECURITY.md",
        "docs/PLAN.md",
    ] {
        assert!(
            root.join(relative).exists(),
            "missing required doc {relative}"
        );
    }
}

#[test]
fn workspace_has_expected_crates() {
    let crates = repo_root().join("crates");
    let count = std::fs::read_dir(&crates)
        .expect("crates directory")
        .filter(|entry| {
            entry
                .as_ref()
                .map(|entry| entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false))
                .unwrap_or(false)
        })
        .count();
    assert!(count >= 20, "expected at least 20 crates, found {count}");
}

#[test]
fn inventory_is_structurally_valid_when_present() {
    let path = repo_root()
        .join("docs")
        .join("source-research")
        .join("capability-inventory.json");
    if !path.exists() {
        return;
    }
    let text = std::fs::read_to_string(&path).expect("read inventory");
    let document: Value = serde_json::from_str(&text).expect("inventory is valid json");
    let entries = document
        .get("entries")
        .and_then(Value::as_array)
        .expect("inventory has entries");
    for entry in entries {
        assert!(
            entry.get("implementation_status").is_some(),
            "entry missing implementation_status"
        );
        assert!(entry.get("mapped_to").is_some(), "entry missing mapped_to");
    }
}
