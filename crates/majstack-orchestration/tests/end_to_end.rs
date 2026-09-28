use majstack_core::RunState;
use majstack_orchestration::{Engine, RunOptions};
use majstack_providers::{MockProvider, ProviderRequest};
use std::path::PathBuf;
use std::sync::Arc;

fn unique_dir() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("majstack-e2e-{}-{nanos}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn init_repo(root: &PathBuf) {
    let _ = majstack_git::git(root, &["init", "-q", "-b", "main"]);
    let _ = majstack_git::git(root, &["config", "user.email", "t@t"]);
    let _ = majstack_git::git(root, &["config", "user.name", "t"]);
    std::fs::write(root.join("README.md"), "hi").unwrap();
    let _ = majstack_git::git(root, &["add", "-A"]);
    let _ = majstack_git::git(root, &["commit", "-qm", "init"]);
}

fn verify_cmd() -> String {
    if cfg!(windows) {
        "(if exist a.txt (exit 0) else (if exist b.txt (exit 0) else (exit 1)))".to_string()
    } else {
        "test -f a.txt || test -f b.txt".to_string()
    }
}

fn mock_provider() -> Arc<MockProvider> {
    let responder = Arc::new(|request: &ProviderRequest| -> String {
        let prompt = &request.prompt;
        if prompt.contains("# PLANNING") {
            return "<tasks>[{\"id\":\"1\",\"title\":\"make a.txt\",\"depends\":[]},{\"id\":\"2\",\"title\":\"make b.txt\",\"depends\":[\"1\"]}]</tasks>".to_string();
        }
        if prompt.contains("review the task diff") {
            return "VERDICT: APPROVE".to_string();
        }
        if prompt.contains("make a.txt") {
            let _ = std::fs::write(request.cwd.join("a.txt"), "x");
        }
        if prompt.contains("make b.txt") {
            let _ = std::fs::write(request.cwd.join("b.txt"), "x");
        }
        "MAJSTACK_LEARN: PATTERN: artifacts are plain text files\nMAJSTACK_DONE: done".to_string()
    });
    Arc::new(MockProvider::new("mock", responder))
}

#[test]
fn runs_goal_end_to_end() {
    let root = unique_dir();
    init_repo(&root);
    let engine = Engine::open_with(&root, Some(("mock".to_string(), mock_provider()))).unwrap();
    let outcome = engine
        .run_goal(
            "add feature a and b",
            RunOptions {
                agent: Some("mock".to_string()),
                verify: Some(verify_cmd()),
                max_iterations: Some(20),
                ..RunOptions::default()
            },
        )
        .unwrap();
    assert_eq!(outcome.state, RunState::Completed, "outcome: {outcome:?}");
    assert_eq!(outcome.completed, 2);
    assert!(root.join("a.txt").exists());
    assert!(root.join("b.txt").exists());
    assert_eq!(engine.store().count("tasks").unwrap(), 2);
    assert_eq!(
        engine
            .store()
            .list_tasks(&outcome.run_id)
            .unwrap()
            .iter()
            .filter(|task| task.status == majstack_core::TaskStatus::Completed)
            .count(),
        2
    );
    assert!(!engine
        .store()
        .search_knowledge("artifacts", 5)
        .unwrap()
        .is_empty());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn swarm_runs_independent_tasks_in_parallel() {
    let root = unique_dir();
    init_repo(&root);
    let engine = Engine::open_with(&root, Some(("mock".to_string(), mock_provider()))).unwrap();
    let run = engine
        .store()
        .create_run(None, Some("feature"), Some("mock"), None, None)
        .unwrap();
    for title in ["make a.txt", "make b.txt"] {
        engine
            .store()
            .insert_task(&majstack_state::NewTask {
                run_id: run.id.clone(),
                title: title.to_string(),
                verify_required: true,
                ..Default::default()
            })
            .unwrap();
    }
    let outcome = engine
        .swarm(
            &run.id,
            "make files",
            2,
            &RunOptions {
                agent: Some("mock".to_string()),
                verify: Some(verify_cmd()),
                ..RunOptions::default()
            },
        )
        .unwrap();
    assert_eq!(
        outcome.completed, 2,
        "swarm outcome completed={}",
        outcome.completed
    );
    assert!(root.join("a.txt").exists());
    assert!(root.join("b.txt").exists());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn blocked_when_dependency_never_satisfied() {
    let root = unique_dir();
    init_repo(&root);
    let responder = Arc::new(|request: &ProviderRequest| -> String {
        if request.prompt.contains("# PLANNING") {
            return "<tasks>[{\"id\":\"1\",\"title\":\"make a.txt\",\"depends\":[\"does-not-exist\"]}]</tasks>".to_string();
        }
        "MAJSTACK_DONE: nope".to_string()
    });
    let engine = Engine::open_with(
        &root,
        Some((
            "mock".to_string(),
            Arc::new(MockProvider::new("mock", responder)),
        )),
    )
    .unwrap();
    let outcome = engine.run_goal(
        "do the thing",
        RunOptions {
            agent: Some("mock".to_string()),
            max_iterations: Some(5),
            ..RunOptions::default()
        },
    );
    assert!(outcome.is_err());
    let _ = std::fs::remove_dir_all(&root);
}
