use super::{Engine, RunOptions};
use majstack_core::{ids, EventKind, Result, TaskStatus};
use majstack_providers::{Provider, ProviderRequest};
use majstack_state::{NewTask, TaskRecord};
use std::path::PathBuf;
use std::sync::Arc;

pub struct SwarmOutcome {
    pub completed: usize,
    pub failed: usize,
    pub rounds: usize,
}

impl Engine {
    fn panel_providers(&self, run_id: &str) -> Result<Vec<(String, Arc<dyn Provider>)>> {
        let names = if self.config.panel.agents.is_empty() {
            vec![self.config.run.agent.clone()]
        } else {
            self.config.panel.agents.clone()
        };
        let mut out = Vec::new();
        for name in names {
            let provider = self.providers.get(&name)?;
            let _ = run_id;
            out.push((name, provider));
        }
        Ok(out)
    }

    pub fn panel(&self, run_id: &str, target: &str) -> Result<Vec<(String, String)>> {
        let providers = self.panel_providers(run_id)?;
        let body = majstack_skills::load_skill(&self.paths.skills_dir(), "review")
            .map(|skill| skill.body)
            .unwrap_or_else(|| {
                "Review the change independently and report concrete findings.".to_string()
            });
        let lenses = majstack_verification::REVIEW_LENSES;
        let timeout = self.config.run.timeout_seconds;
        let root = self.paths.root.clone();
        let mut jobs: Vec<(String, Arc<dyn Provider>, String)> = Vec::new();
        for (index, (name, provider)) in providers.into_iter().enumerate() {
            let lens = lenses[index % lenses.len()];
            let prompt = self.build_prompt(run_id, target, None, &body, None, "review")
                + &format!("\n\n# LENS\n{lens}\n\nReport findings. End with `VERDICT: APPROVE` or `VERDICT: REJECT: <reasons>`.");
            jobs.push((name, provider, prompt));
        }
        let results = std::thread::scope(|scope| {
            let handles: Vec<_> = jobs
                .into_iter()
                .map(|(name, provider, prompt)| {
                    let root = root.clone();
                    scope.spawn(move || {
                        let response =
                            provider.run(&ProviderRequest::new(prompt, root).timeout(timeout));
                        (
                            name,
                            response.map(|response| response.output).unwrap_or_default(),
                        )
                    })
                })
                .collect();
            handles
                .into_iter()
                .filter_map(|handle| handle.join().ok())
                .collect::<Vec<_>>()
        });
        for (name, output) in &results {
            self.store.insert_finding(&majstack_state::FindingRecord {
                id: String::new(),
                run_id: Some(run_id.to_string()),
                task_id: None,
                reviewer: name.clone(),
                severity: "info".to_string(),
                category: Some("panel".to_string()),
                file: None,
                line: None,
                description: Some(output.chars().take(500).collect()),
                evidence: None,
                recommendation: None,
                blocking: false,
                created_at: majstack_core::clock::now_millis(),
            })?;
        }
        Ok(results)
    }

    pub fn swarm(
        &self,
        run_id: &str,
        goal: &str,
        workers: usize,
        options: &RunOptions,
    ) -> Result<SwarmOutcome> {
        let provider = self
            .providers
            .get(options.agent.as_deref().unwrap_or(&self.config.run.agent))?;
        let timeout = self.config.run.timeout_seconds;
        let workers = workers.max(1);
        let mut completed = 0usize;
        let mut failed = 0usize;
        let mut rounds = 0usize;

        loop {
            if self.paths.stop_file().exists() {
                break;
            }
            let dag = self.dag(run_id)?;
            if dag.all_resolved() {
                break;
            }
            let batch: Vec<TaskRecord> = dag.ready_batch(workers).into_iter().cloned().collect();
            if batch.is_empty() {
                break;
            }
            rounds += 1;
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_millis())
                .unwrap_or(0);
            let mut claimed: Vec<TaskRecord> = Vec::new();
            for task in batch {
                let agent = ids::agent_id();
                if self.store.claim_task(&task.id, &agent)? {
                    self.store
                        .update_task_status(&task.id, TaskStatus::Running, None)?;
                    self.bus.emit(
                        EventKind::TaskClaimed,
                        serde_json::json!({ "task": task.id, "swarm": true }),
                    )?;
                    claimed.push(task);
                }
            }
            if claimed.is_empty() {
                break;
            }
            let results = std::thread::scope(|scope| {
                let handles: Vec<_> = claimed
                    .iter()
                    .map(|task| {
                        let provider = provider.clone();
                        let options = options.clone();
                        let task = task.clone();
                        scope.spawn(move || {
                            self.run_swarm_task(
                                run_id, goal, provider, timeout, &task, stamp, &options,
                            )
                        })
                    })
                    .collect();
                handles
                    .into_iter()
                    .filter_map(|handle| handle.join().ok())
                    .collect::<Vec<TaskResult>>()
            });

            for result in results {
                if result.ok && majstack_git::merge(&self.paths.root, &result.branch)? {
                    self.store.update_task_status(
                        &result.task.id,
                        TaskStatus::Completed,
                        Some(""),
                    )?;
                    self.store.insert_commit(
                        Some(run_id),
                        Some(&result.task.id),
                        "",
                        &format!("swarm {}", result.task.title),
                    )?;
                    self.bus.emit(
                        EventKind::TaskCompleted,
                        serde_json::json!({ "task": result.task.id, "swarm": true }),
                    )?;
                    completed += 1;
                    self.apply_rollups(run_id)?;
                } else {
                    self.store.update_task_status(
                        &result.task.id,
                        TaskStatus::Failed,
                        Some("swarm verification or merge failed"),
                    )?;
                    self.bus.emit(
                        EventKind::TaskFailed,
                        serde_json::json!({ "task": result.task.id, "swarm": true }),
                    )?;
                    failed += 1;
                }
                self.store
                    .release_task(&result.task.id, TaskStatus::Failed)?;
                let _ = majstack_git::worktree_remove(&self.paths.root, &result.worktree);
                let _ = majstack_git::delete_branch(&self.paths.root, &result.branch);
            }
            if self.paths.stop_file().exists() {
                break;
            }
        }
        let _ = majstack_git::worktree_prune(&self.paths.root);
        Ok(SwarmOutcome {
            completed,
            failed,
            rounds,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn run_swarm_task(
        &self,
        run_id: &str,
        goal: &str,
        provider: Arc<dyn Provider>,
        timeout: u64,
        task: &TaskRecord,
        stamp: u128,
        options: &RunOptions,
    ) -> TaskResult {
        let worktree = self
            .paths
            .worktrees_dir()
            .join(format!("swarm-{stamp}-{}", task.id));
        let branch = format!("majstack/swarm-{stamp}-{}", task.id);
        if !majstack_git::worktree_add(&self.paths.root, &worktree, &branch).unwrap_or(false) {
            return TaskResult::failure(task.clone(), branch, worktree);
        }
        let body = majstack_skills::load_skill(&self.paths.skills_dir(), "build")
            .map(|skill| skill.body)
            .unwrap_or_else(|| "Implement the task with the smallest correct change.".to_string());
        let prompt = self.build_prompt(run_id, goal, Some(task), &body, None, "execute");
        let response =
            provider.run(&ProviderRequest::new(prompt, worktree.clone()).timeout(timeout));
        if response.is_err() {
            return TaskResult::failure(task.clone(), branch, worktree);
        }
        let _ =
            majstack_git::commit_all(&worktree, &format!("feat: [{}] - {}", task.id, task.title));
        let checks = majstack_verification::detect_checks(&worktree, options.verify.as_deref());
        let results = majstack_verification::run_all(&worktree, &checks, timeout);
        TaskResult {
            task: task.clone(),
            branch,
            worktree,
            ok: majstack_verification::all_passed(&results),
        }
    }

    pub fn arena(
        &self,
        goal: &str,
        contestants: usize,
        options: &RunOptions,
    ) -> Result<Option<usize>> {
        let provider = self
            .providers
            .get(options.agent.as_deref().unwrap_or(&self.config.run.agent))?;
        let timeout = self.config.run.timeout_seconds;
        let run = self
            .store
            .create_run(None, Some("arena"), Some(provider.name()), None, None)?;
        let arena_run_id = run.id.clone();
        self.bus.set_run(Some(arena_run_id.clone()));
        self.bus.emit(
            EventKind::RunCreated,
            serde_json::json!({ "goal": goal, "arena": true }),
        )?;
        let work_type = self.route(goal);
        let goal_id =
            self.store
                .create_goal(None, Some(&run.id), goal, Some(work_type.as_str()))?;
        let _ = self
            .store
            .save_artifact(Some(&run.id), Some(&goal_id), "specification", goal)?;
        let _ = self.store.insert_task(&NewTask {
            run_id: run.id.clone(),
            title: goal.to_string(),
            ..NewTask::default()
        })?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or(0);
        let approaches = [
            "the simplest possible design",
            "the most robust design",
            "the design that changes the least existing code",
            "a data-structure-first design",
            "a design optimized for testability",
        ];
        let body = majstack_skills::load_skill(&self.paths.skills_dir(), "build")
            .map(|skill| skill.body)
            .unwrap_or_else(|| "Implement the goal with the smallest correct change.".to_string());

        let results = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..contestants)
                .map(|index| {
                    let provider = provider.clone();
                    let body = body.clone();
                    let goal = goal.to_string();
                    let options = options.clone();
                    let arena_run_id = arena_run_id.clone();
                    scope.spawn(move || {
                        let worktree = self
                            .paths
                            .worktrees_dir()
                            .join(format!("arena-{stamp}-{index}"));
                        let branch = format!("majstack/arena-{stamp}-{index}");
                        if !majstack_git::worktree_add(&self.paths.root, &worktree, &branch)
                            .unwrap_or(false)
                        {
                            return Contestant {
                                index,
                                branch,
                                worktree,
                                ok: false,
                            };
                        }
                        let approach = approaches[index % approaches.len()];
                        let prompt =
                            self.build_prompt(&arena_run_id, &goal, None, &body, None, "execute")
                                + &format!(
                                "\n\nYou are contestant #{index}. Take this approach: {approach}."
                            );
                        let response = provider
                            .run(&ProviderRequest::new(prompt, worktree.clone()).timeout(timeout));
                        let _ = majstack_git::commit_all(
                            &worktree,
                            &format!("arena contestant {index}"),
                        );
                        let checks = majstack_verification::detect_checks(
                            &worktree,
                            options.verify.as_deref(),
                        );
                        let verification =
                            majstack_verification::run_all(&worktree, &checks, timeout);
                        Contestant {
                            index,
                            branch,
                            worktree,
                            ok: response.is_ok()
                                && majstack_verification::all_passed(&verification),
                        }
                    })
                })
                .collect();
            handles
                .into_iter()
                .filter_map(|handle| handle.join().ok())
                .collect::<Vec<Contestant>>()
        });

        let winner = results
            .iter()
            .find(|contestant| contestant.ok)
            .map(|contestant| contestant.index);
        let mut merged = None;
        for contestant in &results {
            if Some(contestant.index) == winner
                && majstack_git::merge(&self.paths.root, &contestant.branch)?
            {
                merged = Some(contestant.index);
                self.bus.emit(
                    EventKind::TaskCompleted,
                    serde_json::json!({ "arena_winner": contestant.index }),
                )?;
            }
            let _ = majstack_git::worktree_remove(&self.paths.root, &contestant.worktree);
            let _ = majstack_git::delete_branch(&self.paths.root, &contestant.branch);
        }
        let _ = majstack_git::worktree_prune(&self.paths.root);
        self.store.update_run_state(
            &run.id,
            if merged.is_some() {
                majstack_core::RunState::Completed
            } else {
                majstack_core::RunState::Failed
            },
        )?;
        self.bus.set_run(None);
        Ok(merged)
    }
}

struct TaskResult {
    task: TaskRecord,
    branch: String,
    worktree: PathBuf,
    ok: bool,
}

impl TaskResult {
    fn failure(task: TaskRecord, branch: String, worktree: PathBuf) -> Self {
        TaskResult {
            task,
            branch,
            worktree,
            ok: false,
        }
    }
}

struct Contestant {
    index: usize,
    branch: String,
    worktree: PathBuf,
    ok: bool,
}
