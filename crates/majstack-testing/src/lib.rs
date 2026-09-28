use std::path::PathBuf;

pub fn unique_dir(prefix: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

pub fn init_git_repo(root: &std::path::Path) {
    let _ = majstack_git::git(root, &["init", "-q", "-b", "main"]);
    let _ = majstack_git::git(root, &["config", "user.email", "test@example.com"]);
    let _ = majstack_git::git(root, &["config", "user.name", "test"]);
    std::fs::write(root.join("README.md"), "seed").expect("write seed");
    let _ = majstack_git::git(root, &["add", "-A"]);
    let _ = majstack_git::git(root, &["commit", "-qm", "seed"]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_git_repo() {
        let dir = unique_dir("majstack-testing");
        init_git_repo(&dir);
        assert!(majstack_git::is_repo(&dir));
        assert!(majstack_git::has_commits(&dir));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
