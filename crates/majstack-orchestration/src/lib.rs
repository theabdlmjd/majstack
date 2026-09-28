use majstack_config::{MajstackConfig, MajstackPaths};
use majstack_core::slug::slug;
use majstack_core::{
    EventKind, FailureClass, MajstackError, PermissionLevel, Result, RunState, Sigils, TaskStatus,
    WorkType,
};
use majstack_events::EventBus;
use majstack_evidence::{check_evidence, fingerprint, Freshness};
use majstack_graph::Dag;
use majstack_memory::{append_line, remember_lesson, remember_pattern, search, ContextBuilder};
use majstack_policy::Policy;
use majstack_providers::{
    choose_model, ModelStrategy, Provider, ProviderRegistry, ProviderRequest,
};
use majstack_state::{NewTask, Store, TaskRecord};
use majstack_verification::{detect_checks, parse_verdict, run_all, VerificationResult};
use majstack_workflow::{classify_work_type, workflow_for, WorkflowRegistry};
use std::sync::Arc;

mod parallel;

const CONTEXT_BUDGET: usize = 24_000;
const PROTOCOL: &str = "# OUTPUT PROTOCOL\nWork directly in the repository. When finished, end with a line `MAJSTACK_DONE: <one-sentence summary>`. If blocked, end with `MAJSTACK_BLOCKED: <reason>`. For each reusable lesson add `MAJSTACK_LEARN: <lesson>` (prefix `PATTERN:` for conventions).";

#[derive(Debug, Clone, Default)]
pub struct RunOptions {
    pub agent: Option<String>,
    pub verify: Option<String>,
    pub max_iterations: Option<u64>,
    pub max_attempts: Option<u64>,
    pub model: Option<String>,
    pub strategy: Option<String>,
    pub workflow: Option<String>,
    pub branch: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RunOutcome {
    pub run_id: String,
    pub state: RunState,
    pub completed: usize,
    pub failed: usize,
    pub iterations: u64,
}

pub struct Engine {
    paths: MajstackPaths,
    config: MajstackConfig,
    store: Arc<Store>,
    bus: EventBus,
    providers: ProviderRegistry,
    policy: Policy,
}

impl Engine {
    pub fn open(root: impl AsRef<std::path::Path>) -> Result<Self> {
        Self::open_with(root, None)
    }

    pub fn open_with(
        root: impl AsRef<std::path::Path>,
        provider_override: Option<(String, Arc<dyn Provider>)>,
    ) -> Result<Self> {
        let paths = MajstackPaths::discover(root.as_ref());
        paths.ensure()?;
        let config = MajstackConfig::load(&paths)?;
        config.validate()?;
        let store = Arc::new(Store::open(paths.db_file())?);
        store.recover_interrupted()?;
        let bus = EventBus::new(store.clone());
        let mut providers =
            ProviderRegistry::from_config(&config, paths.dir.join("current-prompt.md"))?;
        if let Some((name, provider)) = provider_override {
            providers.insert(name, provider);
        }
        let level = config
            .permissions
            .default
            .unwrap_or(PermissionLevel::Standard);
        let policy = Policy::new(level)
            .with_careful(true)
            .with_freeze(config.permissions.freeze.clone())
            .with_dangerous(config.permissions.allow_dangerous);
        Ok(Engine {
            paths,
            config,
            store,
            bus,
            providers,
            policy,
        })
    }

    pub fn store(&self) -> &Arc<Store> {
        &self.store
    }

    pub fn bus(&self) -> &EventBus {
        &self.bus
    }

    pub fn paths(&self) -> &MajstackPaths {
        &self.paths
    }

    pub fn config(&self) -> &MajstackConfig {
        &self.config
    }

    pub fn policy(&self) -> &Policy {
        &self.policy
    }

    pub fn providers(&self) -> &ProviderRegistry {
        &self.providers
    }

    pub fn route(&self, request: &str) -> WorkType {
        classify_work_type(request)
    }

    fn select_work_type(&self, request: &str) -> WorkType {
        if self.config.typed.enabled {
            if let Ok(client) = majstack_typed::build_client(&self.config.typed) {
                if let Ok((work_type, confidence)) = majstack_typed::classify_work_type(
                    client.as_ref(),
                    request,
                    self.config.typed.confidence_threshold,
                ) {
                    if work_type != WorkType::Unknown
                        && confidence >= self.config.typed.confidence_threshold
                    {
                        return work_type;
                    }
                }
            }
        }
        self.route(request)
    }

    fn emit_state(&self, run_id: &str, state: RunState) -> Result<()> {
        self.store.update_run_state(run_id, state)?;
        self.bus.emit(
            EventKind::RunStateChanged,
            serde_json::json!({ "state": state.as_str() }),
        )?;
        Ok(())
    }

    fn dag(&self, run_id: &str) -> Result<Dag> {
        let tasks = self.store.list_tasks(run_id)?;
        let edges = self.store.all_dependencies(run_id)?;
        Dag::build(&tasks, &edges)
    }

    fn build_prompt(
        &self,
        run_id: &str,
        goal: &str,
        task: Option<&TaskRecord>,
        skill_body: &str,
        previous_failure: Option<&str>,
        stage: &str,
    ) -> String {
        let mut builder = ContextBuilder::new(CONTEXT_BUDGET);
        let principles = majstack_skills::principles_for_stage(&self.paths.principles_dir(), stage);
        let rendered_principles = if principles.is_empty() {
            majstack_skills::principles_text(&self.paths.principles_dir())
        } else {
            principles
                .iter()
                .map(|principle| principle.body.clone())
                .collect::<Vec<_>>()
                .join("\n")
        };
        for principle in &principles {
            let _ = self.store.insert_principle_invocation(
                Some(run_id),
                task.map(|task| task.id.as_str()),
                &principle.meta.id,
                stage,
            );
        }
        builder.add("principles (mandatory)", &rendered_principles, true);
        if !self.policy.freeze.is_empty() {
            builder.add(
                "guard",
                &format!(
                    "Only edit files inside: {}. Edits elsewhere are reverted.",
                    self.policy.freeze.join(", ")
                ),
                true,
            );
        }
        if self.policy.careful {
            builder.add(
                "guard",
                "Careful mode is on: never run destructive commands (force push, hard reset, rm -rf, drop table).",
                true,
            );
        }
        let query = format!(
            "{} {}",
            goal,
            task.map(|task| task.title.clone()).unwrap_or_default()
        );
        let recalled = search(&self.store, &query, 8);
        if !recalled.is_empty() {
            builder.add(
                "memory",
                &recalled
                    .iter()
                    .map(|item| format!("- {item}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
                false,
            );
        }
        builder.add("goal", goal, true);
        if let Some(task) = task {
            let mut block = format!(
                "#{}: {}\n{}",
                task.id,
                task.title,
                task.description.clone().unwrap_or_default()
            );
            if let Some(criteria) = &task.acceptance_criteria {
                block.push_str(&format!("\nAcceptance criteria: {criteria}"));
            }
            if let Some(notes) = &task.notes {
                if !notes.is_empty() {
                    block.push_str(&format!("\nNotes: {notes}"));
                }
            }
            builder.add("current task", &block, true);
        }
        if !skill_body.is_empty() {
            builder.add("skill", skill_body, false);
        }
        let progress = majstack_memory::tail(&self.paths.dir.join("progress.md"), 30);
        if !progress.is_empty() {
            builder.add("recent progress", &progress, false);
        }
        if let Some(failure) = previous_failure {
            builder.add("last failure", failure, false);
        }
        builder.add("output protocol", PROTOCOL, true);
        let _ = run_id;
        builder.render()
    }

    fn apply_rollups(&self, run_id: &str) -> Result<()> {
        for _ in 0..8 {
            let dag = self.dag(run_id)?;
            let tasks = self.store.list_tasks(run_id)?;
            let mut changed = false;
            for task in tasks.iter() {
                if task.parent_id.is_none() {
                    continue;
                }
                if let Some(rolled) = dag.rollup(&task.id) {
                    if task.status != rolled {
                        self.store.update_task_status(&task.id, rolled, None)?;
                        let kind = if rolled == TaskStatus::Completed {
                            EventKind::TaskCompleted
                        } else {
                            EventKind::TaskFailed
                        };
                        self.bus
                            .emit(kind, serde_json::json!({ "task": task.id, "rollup": true }))?;
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
        Ok(())
    }

    fn verify_task(
        &self,
        run_id: &str,
        task: &TaskRecord,
        verify_override: Option<&str>,
    ) -> Result<Vec<VerificationResult>> {
        let checks = detect_checks(&self.paths.root, verify_override);
        self.bus.emit(
            EventKind::VerificationStarted,
            serde_json::json!({ "task": task.id }),
        )?;
        let results = run_all(&self.paths.root, &checks, self.config.run.timeout_seconds);
        for result in &results {
            let fp = fingerprint(&self.paths.root);
            self.store.insert_evidence(
                Some(run_id),
                Some(&task.id),
                &result.name,
                Some(&result.command),
                Some(if result.passed() { 0 } else { 1 }),
                Some(&fp),
                None,
            )?;
            self.store.insert_verification_result(
                run_id,
                &task.id,
                &result.name,
                result.status.as_str(),
                &result.command,
                &result.output,
                result.duration_ms,
            )?;
        }
        self.bus.emit(
            EventKind::VerificationCompleted,
            serde_json::json!({ "task": task.id, "passed": majstack_verification::all_passed(&results) }),
        )?;
        Ok(results)
    }

    fn review_task(
        &self,
        run_id: &str,
        task: &TaskRecord,
        provider: Arc<dyn Provider>,
        timeout: u64,
    ) -> Result<bool> {
        if !self.config.run.review {
            return Ok(true);
        }
        self.bus.emit(
            EventKind::ReviewStarted,
            serde_json::json!({ "task": task.id }),
        )?;
        let body = majstack_skills::load_skill(&self.paths.skills_dir(), "review")
            .map(|skill| skill.body)
            .unwrap_or_else(|| {
                "Review the current change for correctness, tests, and quality.".to_string()
            });
        let prompt = self.build_prompt(
            run_id,
            "review the task diff",
            Some(task),
            &body,
            None,
            "review",
        ) + "\n\nEnd with `VERDICT: APPROVE` or `VERDICT: REJECT: <reasons>`.";
        let response = provider
            .run(&ProviderRequest::new(prompt, self.paths.root.clone()).timeout(timeout))?;
        let usage = serde_json::to_string(&response.usage)?;
        self.store.insert_provider_run(
            Some(run_id),
            provider.name(),
            response.model.as_deref(),
            "ok",
            &usage,
        )?;
        let approved =
            parse_verdict(&response.output) == Some(majstack_core::ReviewVerdict::Approve);
        self.bus.emit(
            EventKind::ReviewCompleted,
            serde_json::json!({ "task": task.id, "approved": approved }),
        )?;
        Ok(approved)
    }

    pub fn run_goal(&self, goal: &str, options: RunOptions) -> Result<RunOutcome> {
        self.run_inner(goal, options, None)
    }

    pub fn resume(&self, run_id: Option<&str>) -> Result<RunOutcome> {
        let run = match run_id {
            Some(id) => self
                .store
                .get_run(id)?
                .ok_or_else(|| MajstackError::NotFound(format!("run {id}")))?,
            None => self
                .store
                .list_runs()?
                .into_iter()
                .find(|run| {
                    matches!(
                        run.state,
                        RunState::Paused
                            | RunState::Blocked
                            | RunState::Ready
                            | RunState::Executing
                    )
                })
                .ok_or_else(|| MajstackError::NotFound("no paused run to resume".into()))?,
        };
        let goal = self
            .store
            .get_goal_text(&run.id)?
            .unwrap_or_else(|| "resume".to_string());
        let _ = std::fs::remove_file(self.paths.stop_file());
        self.run_inner(
            &goal,
            RunOptions {
                agent: run.provider.clone(),
                ..RunOptions::default()
            },
            Some(run),
        )
    }

    fn run_inner(
        &self,
        goal: &str,
        options: RunOptions,
        existing: Option<majstack_state::RunRecord>,
    ) -> Result<RunOutcome> {
        if !majstack_git::is_repo(&self.paths.root) {
            return Err(MajstackError::State(
                "the workspace is not a git repository".into(),
            ));
        }
        let _ = majstack_git::exclude_scratch(&self.paths.root);
        let timeout = self.config.run.timeout_seconds;
        let strategy =
            ModelStrategy::parse(options.strategy.as_deref().unwrap_or("cost-optimized"));

        let (run, goal_id, provider_name, work_type) = if let Some(run) = existing {
            let provider_name = run
                .provider
                .clone()
                .unwrap_or_else(|| self.config.run.agent.clone());
            let work_type = self.route(goal);
            let goal_id =
                self.store
                    .create_goal(None, Some(&run.id), goal, Some(work_type.as_str()))?;
            (run, goal_id, provider_name, work_type)
        } else {
            let provider_name = options
                .agent
                .clone()
                .unwrap_or_else(|| self.config.run.agent.clone());
            let work_type = self.select_work_type(goal);
            let workflow = options
                .workflow
                .clone()
                .unwrap_or_else(|| workflow_for(work_type).to_string());
            let run = self.store.create_run(
                None,
                Some(&workflow),
                Some(&provider_name),
                options.model.as_deref(),
                Some(options.strategy.as_deref().unwrap_or("cost-optimized")),
            )?;
            self.bus.set_run(Some(run.id.clone()));
            self.bus.emit(
                EventKind::RunCreated,
                serde_json::json!({ "goal": goal, "work_type": work_type.as_str() }),
            )?;
            let goal_id =
                self.store
                    .create_goal(None, Some(&run.id), goal, Some(work_type.as_str()))?;
            self.store.update_run_meta(&run.id, "goal_id", &goal_id)?;
            self.bus.emit(
                EventKind::WorkflowSelected,
                serde_json::json!({ "workflow": workflow }),
            )?;
            self.store.insert_decision(
                Some(&run.id),
                None,
                &format!(
                    "routed to work type '{}' using workflow '{}'",
                    work_type.as_str(),
                    workflow
                ),
                None,
                None,
                Some(&provider_name),
                Some(work_type.as_str()),
            )?;
            if self.config.run.branching {
                let branch = options
                    .branch
                    .clone()
                    .unwrap_or_else(|| format!("majstack/{}", slug(goal)));
                if majstack_git::has_commits(&self.paths.root)
                    && majstack_git::checkout_branch(&self.paths.root, &branch)?
                {
                    self.bus.emit(
                        EventKind::BranchCreated,
                        serde_json::json!({ "branch": branch }),
                    )?;
                }
            }
            (run, goal_id, provider_name, work_type)
        };
        let _ = &work_type;

        let provider = self.providers.get(&provider_name)?;
        let feature_slug = slug(goal);
        let feature_dir = self.paths.dir.join("features").join(&feature_slug);
        std::fs::create_dir_all(&feature_dir)?;
        let spec_path = feature_dir.join("spec.md");
        let plan_path = feature_dir.join("plan.md");
        let feature = match self.store.get_feature_by_run(&run.id)? {
            Some(feature) => feature,
            None => self.store.create_feature(
                Some(&run.id),
                goal,
                &feature_slug,
                Some(&spec_path.to_string_lossy()),
                Some(&plan_path.to_string_lossy()),
            )?,
        };
        let feature_id = feature.id.clone();

        let has_tasks = !self.store.list_tasks(&run.id)?.is_empty();
        if !has_tasks {
            self.emit_state(&run.id, RunState::Understanding)?;
            if spec_path.exists() {
                self.emit_state(&run.id, RunState::ReviewingSpec)?;
            } else {
                self.emit_state(&run.id, RunState::Specifying)?;
                let body = majstack_skills::load_skill(&self.paths.skills_dir(), "spec")
                    .map(|skill| skill.body)
                    .unwrap_or_else(|| {
                        "Write a concise specification: problem, users, constraints, and testable acceptance criteria.".to_string()
                    });
                let prompt = self.build_prompt(&run.id, goal, None, &body, None, "understand")
                    + "\n\n# SPECIFYING\nWrite the specification as markdown.";
                let response = provider
                    .run(&ProviderRequest::new(prompt, self.paths.root.clone()).timeout(timeout))?;
                self.store.save_artifact(
                    Some(&run.id),
                    Some(&goal_id),
                    "specification",
                    &response.output,
                )?;
                std::fs::write(&spec_path, &response.output)?;
                self.store.insert_journal(
                    &run.id,
                    None,
                    0,
                    "spec",
                    Some(provider.name()),
                    response.model.as_deref(),
                    response.duration_ms as i64,
                    response.usage.cost_usd,
                    0,
                    &format!("specification written ({} chars)", response.output.len()),
                )?;
                self.emit_state(&run.id, RunState::ReviewingSpec)?;
            }
            self.store.update_feature_status(&feature_id, "planned")?;
            self.emit_state(&run.id, RunState::Planning)?;
            let plan_output = if plan_path.exists() {
                std::fs::read_to_string(&plan_path).unwrap_or_default()
            } else {
                let body = majstack_skills::load_skill(&self.paths.skills_dir(), "plan")
                    .map(|skill| skill.body)
                    .unwrap_or_else(|| {
                        "Decompose the goal into small, independently verifiable tasks.".to_string()
                    });
                let instruction = r#"Write tasks as JSON: <tasks>[{"id":"1","title":"...","detail":"acceptance criteria","priority":1,"depends":[],"ui":false}]</tasks>. Each task must fit one iteration and be independently verifiable."#;
                let prompt = self.build_prompt(&run.id, goal, None, &body, None, "plan")
                    + "\n\n# PLANNING\n"
                    + instruction;
                self.store.insert_skill_invocation(
                    Some(&run.id),
                    None,
                    "plan",
                    Some(provider.name()),
                    "running",
                    None,
                )?;
                let response = provider
                    .run(&ProviderRequest::new(prompt, self.paths.root.clone()).timeout(timeout))?;
                let usage = serde_json::to_string(&response.usage)?;
                self.store.insert_provider_run(
                    Some(&run.id),
                    provider.name(),
                    response.model.as_deref(),
                    if response.success() { "ok" } else { "failed" },
                    &usage,
                )?;
                std::fs::write(&plan_path, &response.output)?;
                self.store.insert_journal(
                    &run.id,
                    None,
                    0,
                    "plan",
                    Some(provider.name()),
                    response.model.as_deref(),
                    response.duration_ms as i64,
                    response.usage.cost_usd,
                    0,
                    &format!("plan produced ({} chars)", response.output.len()),
                )?;
                response.output
            };
            let planned = parse_tasks(&plan_output, &run.id);
            if planned.is_empty() {
                self.emit_state(&run.id, RunState::Failed)?;
                self.store.update_feature_status(&feature_id, "failed")?;
                self.bus.emit(
                    EventKind::RunFailed,
                    serde_json::json!({ "reason": "no plan produced" }),
                )?;
                return Ok(RunOutcome {
                    run_id: run.id,
                    state: RunState::Failed,
                    completed: 0,
                    failed: 0,
                    iterations: 0,
                });
            }
            for task in &planned {
                self.store.insert_task(task)?;
                self.bus.emit(
                    EventKind::TaskCreated,
                    serde_json::json!({ "title": task.title }),
                )?;
            }
            self.emit_state(&run.id, RunState::Decomposing)?;
            self.store.update_feature_status(&feature_id, "ready")?;
        } else {
            self.emit_state(&run.id, RunState::Ready)?;
        }
        self.emit_state(&run.id, RunState::Executing)?;
        self.store.update_feature_status(&feature_id, "running")?;

        let max_iterations = options
            .max_iterations
            .unwrap_or(self.config.run.max_iterations);
        let max_attempts = options
            .max_attempts
            .unwrap_or(self.config.run.max_attempts_per_task);
        let mut iterations = 0u64;
        let mut last_failure: Option<String> = None;
        let mut last_hint: Option<String> = None;
        let mut terminal: RunState;

        loop {
            if self.paths.stop_file().exists() {
                self.bus.emit(EventKind::RunPaused, serde_json::json!({}))?;
                terminal = RunState::Paused;
                break;
            }
            if max_iterations > 0 && iterations >= max_iterations {
                terminal = RunState::Ready;
                break;
            }
            let dag = self.dag(&run.id)?;
            if dag.all_resolved() {
                terminal = RunState::Completed;
                break;
            }
            let next = match dag.next() {
                Some(task) => task.clone(),
                None => {
                    terminal = RunState::Blocked;
                    break;
                }
            };
            if max_attempts > 0 && next.attempts >= max_attempts as i64 {
                self.store.release_task(&next.id, TaskStatus::Failed)?;
                last_failure = Some(format!("task {} exceeded retry limit", next.id));
                continue;
            }
            iterations += 1;
            let agent = majstack_core::ids::agent_id();
            if !self.store.claim_task(&next.id, &agent)? {
                continue;
            }
            self.bus.emit(
                EventKind::TaskClaimed,
                serde_json::json!({ "task": next.id, "agent": agent }),
            )?;
            let attempt_no = next.attempts + 1;
            self.store.set_task_attempts(&next.id, attempt_no)?;
            self.store
                .update_task_status(&next.id, TaskStatus::Running, None)?;
            self.emit_state(&run.id, RunState::Executing)?;

            let explicit_model = options.model.as_deref().or(next.model.as_deref()).or(self
                .config
                .roles
                .code
                .as_deref());
            let model = choose_model(
                strategy,
                explicit_model,
                iterations,
                last_failure.as_ref().map(|_| 1).unwrap_or(0),
                last_hint.as_deref(),
            );
            last_hint = None;
            let build_body = majstack_skills::load_skill(&self.paths.skills_dir(), "build")
                .map(|skill| skill.body)
                .unwrap_or_else(|| {
                    "Implement the task with the smallest correct change.".to_string()
                });
            let prompt = self.build_prompt(
                &run.id,
                goal,
                Some(&next),
                &build_body,
                last_failure.as_deref(),
                "execute",
            );
            let response = provider.run(
                &ProviderRequest::new(prompt, self.paths.root.clone())
                    .model(model.clone())
                    .timeout(timeout),
            )?;
            let usage = serde_json::to_string(&response.usage)?;
            self.store.insert_provider_run(
                Some(&run.id),
                provider.name(),
                response.model.as_deref(),
                if response.success() { "ok" } else { "failed" },
                &usage,
            )?;
            self.store.insert_attempt(
                &next.id,
                Some(&run.id),
                attempt_no,
                Some(&agent),
                Some(provider.name()),
                response.model.as_deref(),
                "completed",
                Some(&log_tail(&response.output)),
                None,
            )?;
            for pattern in response.sigils.patterns() {
                let _ = remember_pattern(&self.store, Some(&run.id), &pattern);
            }
            for lesson in response.sigils.lessons() {
                let _ = remember_lesson(&self.store, Some(&run.id), &lesson);
            }
            if let Some(hint) = &response.sigils.next_model {
                last_hint = Some(hint.clone());
            }
            if response.needs_human {
                self.store.update_task_status(
                    &next.id,
                    TaskStatus::Blocked,
                    Some("provider requires human input"),
                )?;
                self.bus.emit(
                    EventKind::HumanInterventionRequested,
                    serde_json::json!({ "task": next.id }),
                )?;
                terminal = RunState::NeedsHumanInput;
                break;
            }

            let mut failure_reason = response
                .sigils
                .blocked
                .clone()
                .or(response.sigils.failed_reason.clone());

            let mut verification_passed = true;
            if failure_reason.is_none() && next.verify_required {
                self.store
                    .update_task_status(&next.id, TaskStatus::Verifying, None)?;
                let results = self.verify_task(&run.id, &next, options.verify.as_deref())?;
                verification_passed = majstack_verification::all_passed(&results);
                if !verification_passed {
                    let failing = results
                        .iter()
                        .filter(|result| !result.passed())
                        .map(|result| format!("{}: {}", result.name, log_tail(&result.output)))
                        .collect::<Vec<_>>()
                        .join("\n");
                    failure_reason = Some(failing);
                }
            }

            if verification_passed && failure_reason.is_none() {
                self.store
                    .update_task_status(&next.id, TaskStatus::Reviewing, None)?;
                let approved = self.review_task(&run.id, &next, provider.clone(), timeout)?;
                if !approved {
                    failure_reason = Some("review rejected the change".to_string());
                }
            }

            if let Some(reason) = failure_reason {
                let class = classify_failure(&reason);
                self.store.insert_failure(
                    Some(&run.id),
                    Some(&next.id),
                    class.as_str(),
                    &reason,
                    None,
                )?;
                self.bus.emit(
                    EventKind::FailureClassified,
                    serde_json::json!({ "task": next.id, "class": class.as_str() }),
                )?;
                let recovery = majstack_recovery::plan(&reason, attempt_no as u64, max_attempts);
                self.store.insert_recovery_action(
                    Some(&run.id),
                    Some(&next.id),
                    &recovery.summary(),
                    Some(class.as_str()),
                    false,
                )?;
                self.bus.emit(
                    EventKind::RecoveryActionPlanned,
                    serde_json::json!({ "task": next.id, "steps": recovery.summary() }),
                )?;
                if recovery.wants_escalation() {
                    self.bus.emit(
                        EventKind::StrategyAdapted,
                        serde_json::json!({ "task": next.id, "class": class.as_str() }),
                    )?;
                }
                self.store.update_task_status(
                    &next.id,
                    TaskStatus::Failed,
                    Some(&log_tail(&reason)),
                )?;
                self.bus.emit(
                    EventKind::TaskFailed,
                    serde_json::json!({ "task": next.id }),
                )?;
                self.store.release_task(&next.id, TaskStatus::Failed)?;
                self.store.insert_journal(
                    &run.id,
                    Some(&next.id),
                    iterations as i64,
                    "failed",
                    Some(provider.name()),
                    None,
                    0,
                    0.0,
                    0,
                    &reason,
                )?;
                self.store.insert_decision(
                    Some(&run.id),
                    Some(&next.id),
                    &format!("recovery plan: {}", recovery.summary()),
                    None,
                    Some(class.as_str()),
                    Some(provider.name()),
                    None,
                )?;
                last_failure = Some(reason);
                if recovery.blocking {
                    terminal = RunState::NeedsHumanInput;
                    break;
                }
                self.bus.emit(
                    EventKind::TaskRetrying,
                    serde_json::json!({ "task": next.id, "attempt": attempt_no }),
                )?;
                continue;
            }

            if self.config.run.commit {
                let message = format!("feat: [{}] - {}", next.id, next.title);
                if let Some(sha) = majstack_git::commit_all(&self.paths.root, &message)? {
                    self.store
                        .insert_commit(Some(&run.id), Some(&next.id), &sha, &message)?;
                    self.bus.emit(
                        EventKind::CommitCreated,
                        serde_json::json!({ "task": next.id, "sha": sha }),
                    )?;
                }
            }
            self.store
                .update_task_status(&next.id, TaskStatus::Completed, Some(""))?;
            append_line(
                &self.paths.dir.join("progress.md"),
                &format!("- [done] #{} {}", next.id, next.title),
            )?;
            self.bus.emit(
                EventKind::TaskCompleted,
                serde_json::json!({ "task": next.id }),
            )?;
            self.apply_rollups(&run.id)?;
            let files_changed = majstack_git::changed_paths(&self.paths.root).len() as i64;
            self.store.insert_journal(
                &run.id,
                Some(&next.id),
                iterations as i64,
                "completed",
                Some(provider.name()),
                response.model.as_deref(),
                response.duration_ms as i64,
                response.usage.cost_usd,
                files_changed,
                &next.title,
            )?;
            last_failure = None;
        }

        if max_iterations > 0 && iterations >= max_iterations && terminal == RunState::Completed {
            terminal = RunState::Ready;
        }
        self.emit_state(&run.id, terminal)?;
        let tasks = self.store.list_tasks(&run.id)?;
        let completed = tasks
            .iter()
            .filter(|task| task.status == TaskStatus::Completed)
            .count();
        let failed = tasks
            .iter()
            .filter(|task| task.status == TaskStatus::Failed)
            .count();
        let feature_status = match terminal {
            RunState::Completed => "done",
            RunState::Failed | RunState::Blocked => "failed",
            _ => "paused",
        };
        self.store
            .update_feature_status(&feature_id, feature_status)?;
        self.store.insert_journal(
            &run.id,
            None,
            iterations as i64,
            "run",
            None,
            None,
            0,
            0.0,
            0,
            &format!(
                "run finished as '{}' ({} completed, {} failed)",
                terminal.as_str(),
                completed,
                failed
            ),
        )?;
        self.store.insert_decision(
            Some(&run.id),
            None,
            &format!(
                "run finished as '{}' ({} completed, {} failed)",
                terminal.as_str(),
                completed,
                failed
            ),
            None,
            None,
            None,
            Some(terminal.as_str()),
        )?;
        match terminal {
            RunState::Completed => {
                self.bus.emit(
                    EventKind::RunCompleted,
                    serde_json::json!({ "completed": completed }),
                )?;
            }
            RunState::Failed | RunState::Blocked => {
                self.bus.emit(
                    EventKind::RunFailed,
                    serde_json::json!({ "failed": failed }),
                )?;
            }
            _ => {}
        }
        self.bus.set_run(None);
        Ok(RunOutcome {
            run_id: run.id,
            state: terminal,
            completed,
            failed,
            iterations,
        })
    }

    pub fn readiness(&self, labels: &[&str]) -> Result<Vec<(String, Freshness)>> {
        let current = fingerprint(&self.paths.root);
        let checked = check_evidence(&self.store, &current, labels)?;
        Ok(checked.into_iter().collect())
    }
}

pub fn classify_failure(message: &str) -> FailureClass {
    majstack_recovery::classify(message)
}

fn log_tail(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.len().saturating_sub(60);
    lines[start..].join("\n")
}

pub fn parse_tasks(output: &str, run_id: &str) -> Vec<NewTask> {
    let candidates: Vec<String> = {
        let mut found = Vec::new();
        if let Some(start) = output.find("<tasks>") {
            if let Some(end) = output[start..].find("</tasks>") {
                found.push(output[start + 7..start + end].to_string());
            }
        }
        for (start_marker, end_marker) in [("```json", "```"), ("```", "```")] {
            let mut rest = output;
            while let Some(start) = rest.find(start_marker) {
                let after = &rest[start + start_marker.len()..];
                if let Some(end) = after.find(end_marker) {
                    found.push(after[..end].to_string());
                    rest = &after[end + end_marker.len()..];
                } else {
                    break;
                }
            }
            if !found.is_empty() {
                break;
            }
        }
        found.push(output.to_string());
        found
    };

    for candidate in candidates {
        let trimmed = candidate.trim();
        let json_text = if trimmed.starts_with('[') || trimmed.starts_with('{') {
            trimmed.to_string()
        } else if let (Some(start), Some(end)) = (trimmed.find('['), trimmed.rfind(']')) {
            trimmed[start..=end].to_string()
        } else {
            continue;
        };
        let value: serde_json::Value = match serde_json::from_str(&json_text) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let items = if value.is_array() {
            value.as_array().cloned().unwrap_or_default()
        } else if let Some(array) = value.get("tasks").and_then(|tasks| tasks.as_array()) {
            array.clone()
        } else {
            continue;
        };
        let mut tasks = Vec::new();
        for (index, item) in items.iter().enumerate() {
            let title = item
                .get("title")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .to_string();
            if title.is_empty() {
                continue;
            }
            let id = item
                .get("id")
                .map(|value| match value {
                    serde_json::Value::String(text) => text.clone(),
                    other => other.to_string(),
                })
                .filter(|text| !text.is_empty());
            let depends: Vec<String> = item
                .get("depends")
                .and_then(|value| value.as_array())
                .map(|array| {
                    array
                        .iter()
                        .map(|entry| match entry {
                            serde_json::Value::String(text) => text.clone(),
                            other => other.to_string(),
                        })
                        .collect()
                })
                .unwrap_or_default();
            tasks.push(NewTask {
                id,
                run_id: run_id.to_string(),
                parent_id: item
                    .get("parent")
                    .and_then(|value| value.as_str())
                    .map(str::to_string),
                title,
                description: item
                    .get("detail")
                    .or_else(|| item.get("description"))
                    .and_then(|value| value.as_str())
                    .map(str::to_string),
                objective: item
                    .get("objective")
                    .and_then(|value| value.as_str())
                    .map(str::to_string),
                acceptance_criteria: item
                    .get("acceptance")
                    .or_else(|| item.get("criteria"))
                    .and_then(|value| value.as_str())
                    .map(str::to_string),
                priority: item
                    .get("priority")
                    .and_then(|value| value.as_i64())
                    .unwrap_or(index as i64 + 1),
                risk: None,
                complexity: None,
                capability: item
                    .get("capability")
                    .and_then(|value| value.as_str())
                    .map(str::to_string),
                provider: None,
                model: None,
                max_attempts: item
                    .get("max_attempts")
                    .and_then(|value| value.as_i64())
                    .unwrap_or(0),
                verify_required: item
                    .get("verify")
                    .and_then(|value| value.as_bool())
                    .unwrap_or(true),
                evidence_required: false,
                ui: item
                    .get("ui")
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false),
                depends_on: depends,
                meta: None,
            });
        }
        if !tasks.is_empty() {
            return tasks;
        }
    }
    Vec::new()
}

pub fn workflow_registry(paths: &MajstackPaths) -> WorkflowRegistry {
    WorkflowRegistry::load(paths.playbooks_dir())
}

pub fn parse_sigils(output: &str) -> Sigils {
    majstack_core::sigils::parse(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tasks_from_tag() {
        let output = "plan:\n<tasks>[{\"id\":\"1\",\"title\":\"make a.txt\",\"depends\":[]},{\"id\":\"2\",\"title\":\"make b.txt\",\"depends\":[\"1\"]}]</tasks>";
        let tasks = parse_tasks(output, "run-1");
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[1].depends_on, vec!["1".to_string()]);
        assert_eq!(tasks[0].run_id, "run-1");
    }

    #[test]
    fn parses_tasks_from_fence() {
        let output = "here\n```json\n{\"tasks\":[{\"title\":\"only\"}]}\n```";
        let tasks = parse_tasks(output, "run-1");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "only");
    }

    #[test]
    fn classifies_failures() {
        assert_eq!(classify_failure("assertion failed"), FailureClass::Test);
        assert_eq!(
            classify_failure("cannot find module x"),
            FailureClass::Dependency
        );
        assert_eq!(
            classify_failure("timed out after 30s"),
            FailureClass::Timeout
        );
    }
}
