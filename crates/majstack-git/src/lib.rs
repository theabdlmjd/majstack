use majstack_core::Result;
use majstack_execution::{run_process, ProcessOutput, ProcessSpec};
use std::path::{Path, PathBuf};

pub fn git(root: impl AsRef<Path>, args: &[&str]) -> Result<ProcessOutput> {
    let spec = ProcessSpec::new(
        "git",
        args.iter().map(|arg| arg.to_string()).collect(),
        root.as_ref(),
    )
    .timeout(300);
    run_process(&spec)
}

pub fn is_repo(root: impl AsRef<Path>) -> bool {
    git(root, &["rev-parse", "--is-inside-work-tree"])
        .map(|output| output.success())
        .unwrap_or(false)
}

pub fn has_commits(root: impl AsRef<Path>) -> bool {
    git(root, &["rev-parse", "HEAD"])
        .map(|output| output.success())
        .unwrap_or(false)
}

pub fn head(root: impl AsRef<Path>) -> Option<String> {
    git(root, &["rev-parse", "HEAD"])
        .ok()
        .filter(|output| output.success())
        .map(|output| output.stdout.trim().to_string())
}

pub fn branch(root: impl AsRef<Path>) -> Option<String> {
    git(root, &["rev-parse", "--abbrev-ref", "HEAD"])
        .ok()
        .filter(|output| output.success())
        .map(|output| output.stdout.trim().to_string())
}

pub fn changed_paths(root: impl AsRef<Path>) -> Vec<String> {
    let output = match git(root, &["status", "--porcelain", "-uall"]) {
        Ok(output) if output.success() => output,
        _ => return Vec::new(),
    };
    output
        .stdout
        .lines()
        .filter_map(|line| {
            if line.len() < 4 {
                return None;
            }
            let path = &line[3..];
            Some(
                path.rsplit(" -> ")
                    .next()
                    .unwrap_or(path)
                    .trim_matches('"')
                    .to_string(),
            )
        })
        .collect()
}

pub fn is_dirty(root: impl AsRef<Path>) -> bool {
    !changed_paths(root).is_empty()
}

pub fn checkout_branch(root: impl AsRef<Path>, name: &str) -> Result<bool> {
    if branch(&root).as_deref() == Some(name) {
        return Ok(true);
    }
    if git(&root, &["rev-parse", "--verify", name])
        .map(|output| output.success())
        .unwrap_or(false)
    {
        Ok(git(&root, &["checkout", name])?.success())
    } else {
        Ok(git(&root, &["checkout", "-b", name])?.success())
    }
}

pub fn commit_all(root: impl AsRef<Path>, message: &str) -> Result<Option<String>> {
    if !is_repo(&root) {
        return Ok(None);
    }
    git(&root, &["add", "-A"])?;
    let commit = git(&root, &["commit", "-q", "-m", message])?;
    if commit.success() {
        Ok(head(&root))
    } else {
        Ok(None)
    }
}

pub fn exclude_scratch(root: impl AsRef<Path>) -> Result<()> {
    let output = git(&root, &["rev-parse", "--git-path", "info/exclude"])?;
    if !output.success() {
        return Ok(());
    }
    let file = root.as_ref().join(output.stdout.trim());
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let current = std::fs::read_to_string(&file).unwrap_or_default();
    let entries = [
        ".majstack/worktrees/",
        ".majstack/logs/",
        ".majstack/browser-state.json",
        ".majstack/majstack.db",
    ];
    let missing: Vec<&str> = entries
        .iter()
        .filter(|entry| !current.contains(**entry))
        .copied()
        .collect();
    if !missing.is_empty() {
        let mut updated = current;
        if !updated.ends_with('\n') && !updated.is_empty() {
            updated.push('\n');
        }
        updated.push_str(&missing.join("\n"));
        updated.push('\n');
        std::fs::write(&file, updated)?;
    }
    Ok(())
}

pub fn worktree_add(
    root: impl AsRef<Path>,
    path: impl AsRef<Path>,
    branch_name: &str,
) -> Result<bool> {
    let spec = ProcessSpec::new(
        "git",
        vec![
            "worktree".into(),
            "add".into(),
            "-q".into(),
            "-B".into(),
            branch_name.into(),
            path.as_ref().to_string_lossy().to_string(),
        ],
        root.as_ref(),
    )
    .timeout(300);
    Ok(run_process(&spec)?.success())
}

pub fn worktree_remove(root: impl AsRef<Path>, path: impl AsRef<Path>) -> Result<()> {
    let spec = ProcessSpec::new(
        "git",
        vec![
            "worktree".into(),
            "remove".into(),
            "--force".into(),
            path.as_ref().to_string_lossy().to_string(),
        ],
        root.as_ref(),
    )
    .timeout(300);
    let _ = run_process(&spec);
    Ok(())
}

pub fn worktree_prune(root: impl AsRef<Path>) -> Result<()> {
    let _ = git(root, &["worktree", "prune"]);
    Ok(())
}

pub fn merge(root: impl AsRef<Path>, branch_name: &str) -> Result<bool> {
    let output = git(
        &root,
        &[
            "merge",
            "--no-ff",
            "-q",
            "-m",
            &format!("merge {branch_name}"),
            branch_name,
        ],
    )?;
    if !output.success() {
        let _ = git(&root, &["merge", "--abort"]);
    }
    Ok(output.success())
}

pub fn delete_branch(root: impl AsRef<Path>, branch_name: &str) -> Result<()> {
    let _ = git(root, &["branch", "-D", branch_name]);
    Ok(())
}

pub fn diff_stat(root: impl AsRef<Path>, base: &str) -> Result<String> {
    Ok(git(&root, &["diff", "--stat", &format!("{base}...HEAD")])?.stdout)
}

pub fn log_oneline(root: impl AsRef<Path>, limit: usize) -> Vec<String> {
    match git(&root, &["log", "--oneline", "-n", &limit.to_string()]) {
        Ok(output) if output.success() => output.stdout.lines().map(str::to_string).collect(),
        _ => Vec::new(),
    }
}

pub fn default_branch(root: impl AsRef<Path>) -> String {
    for candidate in ["main", "master"] {
        if git(&root, &["rev-parse", "--verify", candidate])
            .map(|output| output.success())
            .unwrap_or(false)
        {
            return candidate.to_string();
        }
    }
    "main".to_string()
}

pub fn root_path(path: impl AsRef<Path>) -> PathBuf {
    path.as_ref().to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_repo(dir: &Path) {
        let _ = run_process(&ProcessSpec::new(
            "git",
            vec!["init".into(), "-q".into(), "-b".into(), "main".into()],
            dir,
        ));
        let _ = run_process(&ProcessSpec::new(
            "git",
            vec!["config".into(), "user.email".into(), "t@t".into()],
            dir,
        ));
        let _ = run_process(&ProcessSpec::new(
            "git",
            vec!["config".into(), "user.name".into(), "t".into()],
            dir,
        ));
    }

    #[test]
    fn commit_and_branch() {
        let dir = std::env::temp_dir().join(format!("majstack-git-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        init_repo(&dir);
        std::fs::write(dir.join("a.txt"), "hi").unwrap();
        let sha = commit_all(&dir, "init").unwrap();
        assert!(sha.is_some());
        assert!(is_repo(&dir));
        assert!(has_commits(&dir));
        assert!(checkout_branch(&dir, "feature/x").unwrap());
        assert_eq!(branch(&dir).as_deref(), Some("feature/x"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
