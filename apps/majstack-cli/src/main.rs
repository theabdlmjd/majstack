use clap::{Parser, Subcommand};
use majstack_config::{MajstackConfig, MajstackPaths};
use majstack_core::{MajstackError, Result, RunState, TaskStatus};
use majstack_execution::doctor;
use majstack_github::{CheckState, GitHub, PrSpec};
use majstack_orchestration::{Engine, RunOptions};
use majstack_workflow::{classify_work_type, workflow_for, WorkflowRegistry};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Parser)]
#[command(
    name = "majstack",
    version,
    about = "Autonomous software engineer: give it a goal, get verified changes."
)]
struct Cli {
    #[arg(long, global = true, default_value = ".")]
    path: PathBuf,
    #[arg(long, global = true, help = "Machine readable JSON output")]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Init,
    Run {
        #[arg(required = true, num_args = 1..)]
        goal: Vec<String>,
        #[arg(long)]
        agent: Option<String>,
        #[arg(long)]
        verify: Option<String>,
        #[arg(long = "max-iterations")]
        max_iterations: Option<u64>,
        #[arg(long = "max-attempts")]
        max_attempts: Option<u64>,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        strategy: Option<String>,
        #[arg(long)]
        workflow: Option<String>,
        #[arg(long)]
        branch: Option<String>,
    },
    Status,
    Tasks {
        #[arg(long)]
        run: Option<String>,
    },
    Route {
        #[arg(required = true, num_args = 1..)]
        request: Vec<String>,
    },
    Skills,
    Workflows,
    Providers,
    Doctor,
    Evidence {
        #[arg(long, num_args = 1.., default_values_t = vec!["verify".to_string()])]
        labels: Vec<String>,
    },
    Logs {
        run: Option<String>,
        #[arg(long, default_value_t = 50)]
        limit: i64,
    },
    Graph {
        #[arg(long)]
        run: Option<String>,
    },
    Pause,
    Resume {
        #[arg(long)]
        run: Option<String>,
    },
    Decide {
        #[arg(required = true, num_args = 1..)]
        decision: Vec<String>,
        #[arg(long)]
        why: Option<String>,
        #[arg(long)]
        run: Option<String>,
    },
    Decisions {
        #[arg(long)]
        run: Option<String>,
    },
    Features,
    Journal {
        #[arg(long)]
        run: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: i64,
    },
    Comments {
        #[arg(long = "dir")]
        dir: Option<String>,
        #[arg(long, default_value_t = 200)]
        limit: usize,
    },
    Agents,
    Eval {
        file: String,
        #[arg(long)]
        threshold: Option<f64>,
    },
    Swarm {
        #[arg(long, default_value_t = 3)]
        workers: usize,
        #[arg(long)]
        run: Option<String>,
        #[arg(long)]
        agent: Option<String>,
        #[arg(long)]
        verify: Option<String>,
    },
    Arena {
        #[arg(required = true, num_args = 1..)]
        goal: Vec<String>,
        #[arg(short = 'n', default_value_t = 3)]
        contestants: usize,
        #[arg(long)]
        agent: Option<String>,
        #[arg(long)]
        verify: Option<String>,
    },
    Panel {
        #[arg(required = true, num_args = 1..)]
        target: Vec<String>,
        #[arg(long)]
        run: Option<String>,
    },
    Pr {
        op: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        body: Option<String>,
        #[arg(long)]
        base: Option<String>,
    },
    Browse {
        url: String,
        #[arg(long)]
        shot: Option<String>,
        #[arg(long)]
        headed: bool,
    },
    Tools,
    InstallSkills {
        #[arg(long, default_value = "claude")]
        host: String,
        #[arg(long)]
        global: bool,
    },
    Ask {
        #[arg(required = true, num_args = 1..)]
        state: Vec<String>,
        #[arg(long)]
        instructions: Option<String>,
        #[arg(long, help = "Comma-separated options for a choice question")]
        choice: Option<String>,
        #[arg(long, help = "Comma-separated ordered levels for a score question")]
        score: Option<String>,
        #[arg(long, help = "Ask a yes/no question")]
        noul: Option<String>,
        #[arg(long)]
        model: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();
    if let Err(error) = dispatch(&cli) {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn print_json(value: &serde_json::Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string())
    );
}

fn dispatch(cli: &Cli) -> Result<()> {
    let paths = MajstackPaths::discover(&cli.path);
    match &cli.command {
        Command::Init => {
            paths.ensure()?;
            let (agent, detected) = write_default_config(&paths)?;
            let installed = majstack_skills::install_assets(&paths.dir)?;
            if majstack_git::is_repo(&paths.root) {
                let _ = majstack_git::exclude_scratch(&paths.root);
            }
            if cli.json {
                print_json(&serde_json::json!({
                    "ok": true,
                    "dir": paths.dir,
                    "assets": installed,
                    "agent": agent,
                    "agent_detected": detected,
                }));
            } else {
                println!(
                    "initialized {} ({} bundled skills/playbooks/principles installed)",
                    paths.dir.display(),
                    installed
                );
                if detected {
                    println!("agent: {agent} (detected on PATH)");
                } else {
                    println!("agent: {agent} (default; no coding-agent CLI detected)");
                    println!("  install one of claude, codex, or opencode, then edit [run] agent in .majstack/config.toml");
                }
            }
            Ok(())
        }
        Command::Route { request } => {
            let text = request.join(" ");
            let work_type = classify_work_type(&text);
            let workflow = workflow_for(work_type);
            if cli.json {
                print_json(
                    &serde_json::json!({ "work_type": work_type.as_str(), "workflow": workflow }),
                );
            } else {
                println!("work type: {}", work_type.as_str());
                println!("workflow:  {workflow}");
            }
            Ok(())
        }
        Command::Skills => {
            let skills = majstack_skills::list_skills(&paths.skills_dir());
            if cli.json {
                print_json(&serde_json::to_value(&skills)?);
            } else if skills.is_empty() {
                println!("no skills found in {}", paths.skills_dir().display());
            } else {
                for skill in skills {
                    println!("{:20} {:10} {}", skill.name, skill.group, skill.summary);
                }
            }
            Ok(())
        }
        Command::Workflows => {
            let registry = WorkflowRegistry::load(paths.playbooks_dir());
            let names = registry.names();
            if cli.json {
                print_json(&serde_json::json!({ "workflows": names }));
            } else if names.is_empty() {
                println!("no workflows found in {}", paths.playbooks_dir().display());
            } else {
                for name in names {
                    println!("{name}");
                }
            }
            Ok(())
        }
        Command::Providers => {
            let engine = Engine::open(&cli.path)?;
            let availability = engine.providers().availability();
            if cli.json {
                print_json(&serde_json::to_value(&availability)?);
            } else {
                for (name, ready) in availability {
                    println!("{name:12} ready={}", if ready { "yes" } else { "no" });
                }
            }
            Ok(())
        }
        Command::Doctor => {
            let report = doctor(&[]);
            if cli.json {
                print_json(&serde_json::to_value(&report)?);
            } else {
                for check in &report.checks {
                    println!("{} {}", if check.ok { "PASS" } else { "FAIL" }, check.name);
                }
            }
            Ok(())
        }
        Command::Evidence { labels } => {
            let engine = Engine::open(&cli.path)?;
            let label_refs: Vec<&str> = labels.iter().map(String::as_str).collect();
            let report = engine.readiness(&label_refs)?;
            if cli.json {
                let map: std::collections::BTreeMap<String, String> = report
                    .iter()
                    .map(|(label, freshness)| (label.clone(), format!("{freshness:?}")))
                    .collect();
                print_json(&serde_json::to_value(&map)?);
            } else {
                for (label, freshness) in report {
                    println!("{label:12} {freshness:?}");
                }
            }
            Ok(())
        }
        Command::Status => {
            let engine = Engine::open(&cli.path)?;
            let runs = engine.store().list_runs()?;
            if cli.json {
                print_json(&serde_json::to_value(&runs)?);
                return Ok(());
            }
            if runs.is_empty() {
                println!("no runs yet");
                return Ok(());
            }
            for run in runs {
                println!(
                    "[{:^11}] {} workflow={} provider={}",
                    run.state.as_str(),
                    run.id,
                    run.workflow.unwrap_or_default(),
                    run.provider.unwrap_or_default()
                );
            }
            Ok(())
        }
        Command::Tasks { run } => {
            let engine = Engine::open(&cli.path)?;
            let run_id = match run {
                Some(run) => run.clone(),
                None => engine
                    .store()
                    .list_runs()?
                    .first()
                    .map(|run| run.id.clone())
                    .ok_or_else(|| MajstackError::NotFound("no runs".into()))?,
            };
            let tasks = engine.store().list_tasks(&run_id)?;
            if cli.json {
                print_json(&serde_json::to_value(&tasks)?);
                return Ok(());
            }
            for task in tasks {
                let marker = match task.status {
                    TaskStatus::Completed => "x",
                    TaskStatus::Failed => "!",
                    TaskStatus::Running => ">",
                    _ => " ",
                };
                println!(
                    "[{marker}] {:<14} p{} {}",
                    task.id, task.priority, task.title
                );
            }
            Ok(())
        }
        Command::Logs { run, limit } => {
            let engine = Engine::open(&cli.path)?;
            let run_id = match run {
                Some(run) => run.clone(),
                None => engine
                    .store()
                    .list_runs()?
                    .first()
                    .map(|run| run.id.clone())
                    .ok_or_else(|| MajstackError::NotFound("no runs".into()))?,
            };
            let events = engine.store().list_events(&run_id, *limit)?;
            for event in events {
                println!(
                    "{:>13} {:20} {}",
                    event.ts,
                    event.kind.as_str(),
                    event.payload
                );
            }
            Ok(())
        }
        Command::Graph { run } => {
            let engine = Engine::open(&cli.path)?;
            let run_id = match run {
                Some(run) => run.clone(),
                None => engine
                    .store()
                    .list_runs()?
                    .first()
                    .map(|run| run.id.clone())
                    .ok_or_else(|| MajstackError::NotFound("no runs".into()))?,
            };
            let tasks = engine.store().list_tasks(&run_id)?;
            let edges = engine.store().all_dependencies(&run_id)?;
            let mut lines = vec!["flowchart TD".to_string()];
            for task in &tasks {
                let title = task.title.replace('"', "'");
                lines.push(format!("  T{}[\"#{}\"]", task.id, title));
            }
            for (task, depends_on) in edges {
                lines.push(format!("  T{depends_on} --> T{task}"));
            }
            println!("{}", lines.join("\n"));
            Ok(())
        }
        Command::Pause => {
            paths.ensure()?;
            std::fs::write(paths.stop_file(), "pause")?;
            println!("pause requested");
            Ok(())
        }
        Command::Resume { run } => {
            let engine = Engine::open(&cli.path)?;
            let outcome = engine.resume(run.as_deref())?;
            if cli.json {
                print_json(&serde_json::json!({
                    "run_id": outcome.run_id,
                    "state": outcome.state.as_str(),
                    "completed": outcome.completed,
                    "failed": outcome.failed,
                }));
            } else {
                println!(
                    "[majstack] resumed {}: {} ({} completed, {} failed)",
                    outcome.run_id,
                    outcome.state.as_str(),
                    outcome.completed,
                    outcome.failed
                );
            }
            Ok(())
        }
        Command::Decide { decision, why, run } => {
            let engine = Engine::open(&cli.path)?;
            let run_id = match run {
                Some(run) => Some(run.clone()),
                None => engine
                    .store()
                    .list_runs()?
                    .first()
                    .map(|run| run.id.clone()),
            };
            engine.store().insert_decision(
                run_id.as_deref(),
                None,
                &decision.join(" "),
                None,
                why.as_deref(),
                None,
                None,
            )?;
            println!("logged decision");
            Ok(())
        }
        Command::Decisions { run } => {
            let engine = Engine::open(&cli.path)?;
            let decisions = match run {
                Some(run) => engine.store().list_decisions(run)?,
                None => engine.store().list_all_decisions(200)?,
            };
            if cli.json {
                print_json(&serde_json::to_value(&decisions)?);
                return Ok(());
            }
            for decision in decisions {
                println!("- {}", decision.decision);
                if let Some(evidence) = decision.evidence {
                    println!("    evidence: {evidence}");
                }
                if let Some(result) = decision.result {
                    println!("    result: {result}");
                }
            }
            Ok(())
        }
        Command::Features => {
            let engine = Engine::open(&cli.path)?;
            let features = engine.store().list_features()?;
            if cli.json {
                print_json(&serde_json::to_value(&features)?);
                return Ok(());
            }
            for feature in features {
                println!(
                    "[{:^8}] {} ({})",
                    feature.status, feature.name, feature.slug
                );
            }
            Ok(())
        }
        Command::Journal { run, limit } => {
            let engine = Engine::open(&cli.path)?;
            let run_id = match run {
                Some(run) => run.clone(),
                None => latest_run(&engine)?,
            };
            let entries = engine.store().recent_journal(&run_id, *limit)?;
            if cli.json {
                print_json(&serde_json::to_value(&entries)?);
                return Ok(());
            }
            for entry in entries {
                println!(
                    "#{:<3} {:10} {:8} {:>7}ms {}",
                    entry.iteration,
                    entry.outcome,
                    entry.model.unwrap_or_default(),
                    entry.duration_ms,
                    entry.notes.unwrap_or_default()
                );
            }
            Ok(())
        }
        Command::Comments { dir, limit } => {
            let root = match dir {
                Some(dir) => PathBuf::from(dir),
                None => cli.path.clone(),
            };
            let findings = majstack_agents::comment_sicko::scan(&root, *limit);
            if cli.json {
                print_json(&serde_json::to_value(&findings)?);
                return Ok(());
            }
            for finding in &findings {
                println!("{}:{} {}", finding.file, finding.line, finding.text);
            }
            println!("{} comment(s) flagged", findings.len());
            Ok(())
        }
        Command::Agents => {
            for role in majstack_agents::ROLES {
                println!(
                    "{:24} {} ({})",
                    role.name,
                    role.purpose,
                    if role.read_only { "read-only" } else { "write" }
                );
            }
            Ok(())
        }
        Command::Eval { file, threshold } => {
            let engine = Engine::open(&cli.path)?;
            let text = std::fs::read_to_string(file)?;
            let suite = majstack_typed::load_suite(&text)?;
            let mut typed = engine.config().typed.clone();
            typed.enabled = true;
            let client = majstack_typed::build_client(&typed)?;
            let cutoff = threshold.unwrap_or(typed.confidence_threshold);
            let report = majstack_typed::run_eval(client.as_ref(), &suite, cutoff)?;
            if cli.json {
                print_json(&serde_json::to_value(&report)?);
            } else {
                println!("suite: {}", report.name);
                println!(
                    "accuracy: {}/{} ({:.0}%)",
                    report.correct,
                    report.total,
                    report.accuracy() * 100.0
                );
                println!("mean confidence: {:.2}", report.mean_confidence);
                println!(
                    "accurate & confident (>= {cutoff:.2}): {}",
                    report.accurate_at_threshold
                );
                println!("below threshold: {}", report.below_threshold);
                for failure in &report.failures {
                    println!("  {failure}");
                }
            }
            Ok(())
        }
        Command::Swarm {
            workers,
            run,
            agent,
            verify,
        } => {
            let engine = Engine::open(&cli.path)?;
            let run_id = match run {
                Some(run) => run.clone(),
                None => latest_run(&engine)?,
            };
            let outcome = engine.swarm(
                &run_id,
                "",
                *workers,
                &RunOptions {
                    agent: agent.clone(),
                    verify: verify.clone(),
                    ..RunOptions::default()
                },
            )?;
            if cli.json {
                print_json(&serde_json::json!({
                    "completed": outcome.completed,
                    "failed": outcome.failed,
                    "rounds": outcome.rounds,
                }));
            } else {
                println!(
                    "swarm: {} completed, {} failed in {} rounds",
                    outcome.completed, outcome.failed, outcome.rounds
                );
            }
            Ok(())
        }
        Command::Arena {
            goal,
            contestants,
            agent,
            verify,
        } => {
            let engine = Engine::open(&cli.path)?;
            let winner = engine.arena(
                &goal.join(" "),
                *contestants,
                &RunOptions {
                    agent: agent.clone(),
                    verify: verify.clone(),
                    ..RunOptions::default()
                },
            )?;
            if cli.json {
                print_json(&serde_json::json!({ "winner": winner }));
            } else {
                match winner {
                    Some(index) => println!("arena winner: contestant {index}"),
                    None => println!("arena: no contestant passed verification"),
                }
            }
            Ok(())
        }
        Command::Panel { target, run } => {
            let engine = Engine::open(&cli.path)?;
            let run_id = match run {
                Some(run) => run.clone(),
                None => latest_run(&engine)?,
            };
            let results = engine.panel(&run_id, &target.join(" "))?;
            for (name, output) in results {
                println!(
                    "### {name}\n{}",
                    output.chars().take(1500).collect::<String>()
                );
            }
            Ok(())
        }
        Command::Pr {
            op,
            title,
            body,
            base,
        } => {
            let engine = Engine::open(&cli.path)?;
            let config = engine.config();
            let base = base
                .clone()
                .unwrap_or_else(|| config.github.base_branch.clone());
            let gh = GitHub::new(&paths.root, base, config.github.draft);
            match op.as_str() {
                "open" => {
                    let spec = PrSpec {
                        title: title
                            .clone()
                            .unwrap_or_else(|| "Automated change".to_string()),
                        body: body.clone().unwrap_or_default(),
                        base: gh.config_base().to_string(),
                        draft: gh.config_draft(),
                    };
                    let result = gh.create_pr(&spec)?;
                    println!("opened PR #{} {}", result.number, result.url);
                }
                "checks" => {
                    let summary = gh.checks()?;
                    if cli.json {
                        print_json(&serde_json::to_value(&summary)?);
                    } else {
                        println!("state: {:?}", summary.state);
                        println!("failing: {:?}", summary.failing);
                        println!("pending: {:?}", summary.pending);
                    }
                }
                "merge" => {
                    let merged = gh.merge_pr(true)?;
                    println!("merged: {merged}");
                }
                "babysit" => {
                    babysit(&cli.path, &engine, &gh, config.run.babysit_rounds)?;
                }
                other => {
                    return Err(MajstackError::Invalid(format!("unknown pr op '{other}'")));
                }
            }
            Ok(())
        }
        Command::Browse { url, shot, headed } => {
            let engine = Engine::open(&cli.path)?;
            let shots = engine.paths().dir.join("shots");
            let mut session = majstack_browser::BrowserSession::launch(!*headed, shots)?;
            session.navigate(url)?;
            let title = session.title().unwrap_or_default();
            let text = session.text().unwrap_or_default();
            let errors = session.console_errors.clone();
            let screenshot = match shot {
                Some(name) => Some(session.screenshot(name)?),
                None => None,
            };
            session.close();
            if cli.json {
                print_json(&serde_json::json!({
                    "title": title,
                    "console_errors": errors,
                    "screenshot": screenshot,
                    "text_preview": text.chars().take(2000).collect::<String>(),
                }));
            } else {
                println!("title: {title}");
                if !errors.is_empty() {
                    println!("console errors: {errors:?}");
                }
                println!("{}", text.chars().take(1500).collect::<String>());
            }
            Ok(())
        }
        Command::Tools => {
            let registry = majstack_tools::ToolRegistry::builtin();
            for name in registry.names() {
                if let Some(tool) = registry.get(&name) {
                    let spec = tool.spec();
                    println!(
                        "{:14} {:12} {}",
                        name,
                        spec.permission.as_str(),
                        spec.description
                    );
                }
            }
            Ok(())
        }
        Command::InstallSkills { host, global } => {
            let layout = majstack_skills::host_layout(host).ok_or_else(|| {
                MajstackError::Invalid(format!(
                    "unknown host '{host}'. Known: {}",
                    majstack_skills::HOSTS.join(", ")
                ))
            })?;
            let base = if *global {
                let home = std::env::var("USERPROFILE")
                    .or_else(|_| std::env::var("HOME"))
                    .map_err(|_| MajstackError::Config("cannot determine home directory".into()))?;
                PathBuf::from(home).join(layout.global_skills)
            } else {
                paths.root.join(layout.project_skills)
            };
            let count = majstack_skills::install_skills(&base, "majstack-")?;
            if cli.json {
                print_json(&serde_json::json!({ "host": host, "dir": base, "skills": count }));
            } else {
                println!("installed {count} skills for {host} -> {}", base.display());
            }
            Ok(())
        }
        Command::Ask {
            state,
            instructions,
            choice,
            score,
            noul,
            model,
        } => {
            let engine = Engine::open(&cli.path)?;
            let mut typed = engine.config().typed.clone();
            typed.enabled = true;
            if let Some(model) = model {
                typed.model = model.clone();
            }
            let client = majstack_typed::build_client(&typed)?;
            let question = if let Some(options) = choice {
                let criteria = options
                    .split(',')
                    .map(|option| (option.trim().to_string(), serde_json::Value::Null))
                    .collect::<BTreeMap<_, _>>();
                majstack_typed::Question::choice(
                    instructions
                        .clone()
                        .unwrap_or_else(|| "Choose the best option.".to_string()),
                    criteria,
                )
            } else if let Some(levels) = score {
                majstack_typed::Question::score(
                    instructions
                        .clone()
                        .unwrap_or_else(|| "Rate the state.".to_string()),
                    levels
                        .split(',')
                        .map(|level| level.trim().to_string())
                        .collect(),
                )
            } else if let Some(question) = noul {
                majstack_typed::Question::noul(question.clone())
            } else {
                return Err(MajstackError::Invalid(
                    "provide --choice, --score, or --noul".into(),
                ));
            };
            let mut questions = BTreeMap::new();
            questions.insert("answer".to_string(), question);
            let response = majstack_typed::SystemOne::ask(
                client.as_ref(),
                serde_json::json!({ "state": state.join(" ") }),
                questions,
            )?;
            if cli.json {
                print_json(&serde_json::to_value(&response)?);
            } else {
                println!("{}", serde_json::to_string_pretty(&response.answers)?);
            }
            Ok(())
        }
        Command::Run {
            goal,
            agent,
            verify,
            max_iterations,
            max_attempts,
            model,
            strategy,
            workflow,
            branch,
        } => {
            let engine = Engine::open(&cli.path)?;
            let text = goal.join(" ");
            let options = RunOptions {
                agent: agent.clone(),
                verify: verify.clone(),
                max_iterations: *max_iterations,
                max_attempts: *max_attempts,
                model: model.clone(),
                strategy: strategy.clone(),
                workflow: workflow.clone(),
                branch: branch.clone(),
            };
            let outcome = engine.run_goal(&text, options)?;
            if cli.json {
                print_json(&serde_json::json!({
                    "run_id": outcome.run_id,
                    "state": outcome.state.as_str(),
                    "completed": outcome.completed,
                    "failed": outcome.failed,
                    "iterations": outcome.iterations,
                }));
            } else {
                println!(
                    "[majstack] {}: {} completed, {} failed, {} iterations (run {})",
                    outcome.state.as_str(),
                    outcome.completed,
                    outcome.failed,
                    outcome.iterations,
                    outcome.run_id
                );
            }
            let code = match outcome.state {
                RunState::Completed => 0,
                RunState::Blocked => 2,
                _ => 1,
            };
            std::process::exit(code);
        }
    }
}

fn latest_run(engine: &Engine) -> Result<String> {
    engine
        .store()
        .list_runs()?
        .first()
        .map(|run| run.id.clone())
        .ok_or_else(|| MajstackError::NotFound("no runs".into()))
}

fn babysit(root: &Path, engine: &Engine, gh: &GitHub, rounds: u64) -> Result<()> {
    let mut round = 0u64;
    loop {
        if engine.paths().stop_file().exists() {
            println!("stop requested");
            return Ok(());
        }
        let summary = gh.checks()?;
        match summary.state {
            CheckState::Passing => {
                println!("checks green");
                return Ok(());
            }
            CheckState::Pending => {
                println!("checks pending...");
                std::thread::sleep(Duration::from_secs(10));
            }
            _ => {
                let comments = gh.pr_comments().unwrap_or_default();
                let goal = format!(
                    "Fix the failing CI checks: {}\n\nReview comments:\n{}",
                    summary.failing.join(", "),
                    comments
                );
                let _ = engine.run_goal(&goal, RunOptions::default());
                let _ = majstack_git::git(root, &["push"]);
                round += 1;
                if rounds > 0 && round >= rounds {
                    println!("babysit round limit reached");
                    return Ok(());
                }
                std::thread::sleep(Duration::from_secs(10));
            }
        }
    }
}

fn detect_agent(config: &MajstackConfig) -> (String, bool) {
    for name in ["claude", "codex", "opencode"] {
        if let Some(spec) = config.agents.get(name) {
            if let Some(program) = spec.cmd.first() {
                if majstack_execution::command_exists(program) {
                    return (name.to_string(), true);
                }
            }
        }
    }
    (config.run.agent.clone(), false)
}

fn write_default_config(paths: &MajstackPaths) -> Result<(String, bool)> {
    let file = paths.config_file();
    let mut config = MajstackConfig::with_builtin_agents();
    let (agent, detected) = detect_agent(&config);
    config.run.agent = agent.clone();
    if !file.exists() {
        std::fs::write(&file, toml_string(&config))?;
    }
    Ok((agent, detected))
}

fn toml_string(config: &MajstackConfig) -> String {
    let mut out = String::new();
    out.push_str("[run]\n");
    out.push_str(&format!("agent = \"{}\"\n", config.run.agent));
    out.push_str("verify = \"\"\n");
    out.push_str("max_iterations = 0\n");
    out.push_str("max_attempts_per_task = 0\n");
    out.push_str("timeout_seconds = 0\n");
    out.push_str(&format!("commit = {}\n", config.run.commit));
    out.push_str(&format!("review = {}\n", config.run.review));
    out.push_str(&format!("branching = {}\n", config.run.branching));
    out.push_str("\n[permissions]\n");
    out.push_str("allow_dangerous = false\n");
    out.push_str("\n[typed]\n");
    out.push_str("enabled = false\n");
    out.push_str("provider = \"typesafe\"\n");
    out.push_str("base_url = \"https://api.typesafe.ai/v1/systemone\"\n");
    out.push_str("model = \"jev-latest\"\n");
    out.push_str("confidence_threshold = 0.6\n");
    out.push_str("\n[logging]\n");
    out.push_str("level = \"info\"\n");
    for (name, spec) in &config.agents {
        out.push_str(&format!("\n[agents.{name}]\n"));
        out.push_str(&format!("mode = \"{}\"\n", spec.mode));
        if !spec.cmd.is_empty() {
            let cmd: Vec<String> = spec.cmd.iter().map(|part| format!("\"{part}\"")).collect();
            out.push_str(&format!("cmd = [{}]\n", cmd.join(", ")));
        }
        if !spec.model_args.is_empty() {
            let args: Vec<String> = spec
                .model_args
                .iter()
                .map(|part| format!("\"{part}\""))
                .collect();
            out.push_str(&format!("model_args = [{}]\n", args.join(", ")));
        }
        out.push_str(&format!(
            "instructions_file = \"{}\"\n",
            spec.instructions_file
        ));
    }
    out
}
