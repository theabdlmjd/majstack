#![allow(clippy::too_many_arguments)]

mod schema;

pub use schema::SCHEMA;

use majstack_core::clock;
use majstack_core::ids;
use majstack_core::{Event, EventKind, MajstackError, Result, RunState, TaskStatus};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

fn db(error: impl std::fmt::Display) -> MajstackError {
    MajstackError::Database(error.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectRecord {
    pub id: String,
    pub root: String,
    pub name: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRecord {
    pub id: String,
    pub project_id: Option<String>,
    pub goal_id: Option<String>,
    pub state: RunState,
    pub workflow: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub strategy: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub finished_at: Option<i64>,
    pub meta: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecord {
    pub id: String,
    pub run_id: String,
    pub parent_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub objective: Option<String>,
    pub acceptance_criteria: Option<String>,
    pub status: TaskStatus,
    pub priority: i64,
    pub risk: Option<String>,
    pub complexity: Option<String>,
    pub capability: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub workspace: Option<String>,
    pub attempts: i64,
    pub max_attempts: i64,
    pub verify_required: bool,
    pub evidence_required: bool,
    pub ui: bool,
    pub claimed_by: Option<String>,
    pub notes: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
    pub meta: Option<String>,
}

impl TaskRecord {
    pub fn dependencies_of(store: &Store, task_id: &str) -> Result<Vec<String>> {
        store.dependencies_of(task_id)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NewTask {
    pub id: Option<String>,
    pub run_id: String,
    pub parent_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub objective: Option<String>,
    pub acceptance_criteria: Option<String>,
    pub priority: i64,
    pub risk: Option<String>,
    pub complexity: Option<String>,
    pub capability: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub max_attempts: i64,
    pub verify_required: bool,
    pub evidence_required: bool,
    pub ui: bool,
    pub depends_on: Vec<String>,
    pub meta: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttemptRecord {
    pub id: String,
    pub task_id: String,
    pub run_id: Option<String>,
    pub attempt_no: i64,
    pub agent_id: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub status: String,
    pub output_log: Option<String>,
    pub error: Option<String>,
    pub started_at: i64,
    pub finished_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRecord {
    pub id: String,
    pub run_id: Option<String>,
    pub task_id: Option<String>,
    pub decision: String,
    pub alternatives: Option<String>,
    pub evidence: Option<String>,
    pub agent: Option<String>,
    pub result: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceRecord {
    pub id: String,
    pub run_id: Option<String>,
    pub task_id: Option<String>,
    pub label: String,
    pub command: Option<String>,
    pub code: Option<i64>,
    pub fingerprint: Option<String>,
    pub log_path: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingRecord {
    pub id: String,
    pub run_id: Option<String>,
    pub task_id: Option<String>,
    pub reviewer: String,
    pub severity: String,
    pub category: Option<String>,
    pub file: Option<String>,
    pub line: Option<i64>,
    pub description: Option<String>,
    pub evidence: Option<String>,
    pub recommendation: Option<String>,
    pub blocking: bool,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureRecord {
    pub id: String,
    pub run_id: Option<String>,
    pub name: String,
    pub slug: String,
    pub spec_path: Option<String>,
    pub plan_path: Option<String>,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalRecord {
    pub id: String,
    pub run_id: String,
    pub task_id: Option<String>,
    pub iteration: i64,
    pub outcome: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub duration_ms: i64,
    pub cost_usd: f64,
    pub files_changed: i64,
    pub notes: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecoveryReport {
    pub tasks_reset: usize,
    pub runs_suspended: usize,
}

pub struct Store {
    conn: Mutex<Connection>,
}

fn row_to_run(row: &rusqlite::Row<'_>) -> rusqlite::Result<RunRecord> {
    let state: String = row.get("state")?;
    Ok(RunRecord {
        id: row.get("id")?,
        project_id: row.get("project_id")?,
        goal_id: row.get("goal_id")?,
        state: RunState::parse(&state).unwrap_or(RunState::Requested),
        workflow: row.get("workflow")?,
        provider: row.get("provider")?,
        model: row.get("model")?,
        strategy: row.get("strategy")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        finished_at: row.get("finished_at")?,
        meta: row.get("meta")?,
    })
}

fn row_to_task(row: &rusqlite::Row<'_>) -> rusqlite::Result<TaskRecord> {
    let status: String = row.get("status")?;
    Ok(TaskRecord {
        id: row.get("id")?,
        run_id: row.get("run_id")?,
        parent_id: row.get("parent_id")?,
        title: row.get("title")?,
        description: row.get("description")?,
        objective: row.get("objective")?,
        acceptance_criteria: row.get("acceptance_criteria")?,
        status: TaskStatus::parse(&status).unwrap_or(TaskStatus::Pending),
        priority: row.get("priority")?,
        risk: row.get("risk")?,
        complexity: row.get("complexity")?,
        capability: row.get("capability")?,
        provider: row.get("provider")?,
        model: row.get("model")?,
        workspace: row.get("workspace")?,
        attempts: row.get("attempts")?,
        max_attempts: row.get("max_attempts")?,
        verify_required: row.get::<_, i64>("verify_required")? != 0,
        evidence_required: row.get::<_, i64>("evidence_required")? != 0,
        ui: row.get::<_, i64>("ui")? != 0,
        claimed_by: row.get("claimed_by")?,
        notes: row.get("notes")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        started_at: row.get("started_at")?,
        finished_at: row.get("finished_at")?,
        meta: row.get("meta")?,
    })
}

impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path).map_err(db)?;
        let store = Store {
            conn: Mutex::new(conn),
        };
        store.migrate()?;
        Ok(store)
    }

    pub fn memory() -> Result<Self> {
        let conn = Connection::open_in_memory().map_err(db)?;
        let store = Store {
            conn: Mutex::new(conn),
        };
        store.migrate()?;
        Ok(store)
    }

    fn lock(&self) -> MutexGuard<'_, Connection> {
        self.conn
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn migrate(&self) -> Result<()> {
        let conn = self.lock();
        conn.execute_batch(SCHEMA).map_err(db)?;
        Ok(())
    }

    pub fn count(&self, table: &str) -> Result<i64> {
        let conn = self.lock();
        let sql = format!("SELECT count(*) FROM {table}");
        conn.query_row(&sql, [], |row| row.get(0)).map_err(db)
    }

    pub fn create_project(&self, root: &str, name: &str) -> Result<ProjectRecord> {
        let record = ProjectRecord {
            id: ids::new_id("proj"),
            root: root.to_string(),
            name: name.to_string(),
            created_at: clock::now_millis(),
        };
        let conn = self.lock();
        conn.execute(
            "INSERT INTO projects (id, root, name, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![record.id, record.root, record.name, record.created_at],
        )
        .map_err(db)?;
        Ok(record)
    }

    pub fn create_run(
        &self,
        project_id: Option<&str>,
        workflow: Option<&str>,
        provider: Option<&str>,
        model: Option<&str>,
        strategy: Option<&str>,
    ) -> Result<RunRecord> {
        let now = clock::now_millis();
        let record = RunRecord {
            id: ids::new_id("run"),
            project_id: project_id.map(str::to_string),
            goal_id: None,
            state: RunState::Requested,
            workflow: workflow.map(str::to_string),
            provider: provider.map(str::to_string),
            model: model.map(str::to_string),
            strategy: strategy.map(str::to_string),
            created_at: now,
            updated_at: now,
            finished_at: None,
            meta: None,
        };
        let conn = self.lock();
        conn.execute(
            "INSERT INTO runs (id, project_id, goal_id, state, workflow, provider, model, strategy, created_at, updated_at, finished_at, meta)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                record.id,
                record.project_id,
                record.goal_id,
                record.state.as_str(),
                record.workflow,
                record.provider,
                record.model,
                record.strategy,
                record.created_at,
                record.updated_at,
                record.finished_at,
                record.meta
            ],
        )
        .map_err(db)?;
        Ok(record)
    }

    pub fn get_run(&self, id: &str) -> Result<Option<RunRecord>> {
        let conn = self.lock();
        conn.query_row("SELECT * FROM runs WHERE id = ?1", params![id], row_to_run)
            .optional()
            .map_err(db)
    }

    pub fn list_runs(&self) -> Result<Vec<RunRecord>> {
        let conn = self.lock();
        let mut stmt = conn
            .prepare("SELECT * FROM runs ORDER BY created_at DESC")
            .map_err(db)?;
        let rows = stmt.query_map([], row_to_run).map_err(db)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(db)
    }

    pub fn update_run_state(&self, id: &str, state: RunState) -> Result<()> {
        let conn = self.lock();
        let finished = if state.is_terminal() {
            Some(clock::now_millis())
        } else {
            None
        };
        conn.execute(
            "UPDATE runs SET state = ?1, updated_at = ?2, finished_at = COALESCE(?3, finished_at) WHERE id = ?4",
            params![state.as_str(), clock::now_millis(), finished, id],
        )
        .map_err(db)?;
        Ok(())
    }

    pub fn update_run_meta(&self, id: &str, key: &str, value: &str) -> Result<()> {
        let conn = self.lock();
        let existing: Option<String> = conn
            .query_row("SELECT meta FROM runs WHERE id = ?1", params![id], |row| {
                row.get(0)
            })
            .optional()
            .map_err(db)?
            .flatten();
        let mut map: serde_json::Map<String, serde_json::Value> = existing
            .as_deref()
            .and_then(|text| serde_json::from_str(text).ok())
            .unwrap_or_default();
        map.insert(
            key.to_string(),
            serde_json::Value::String(value.to_string()),
        );
        let encoded = serde_json::Value::Object(map).to_string();
        conn.execute(
            "UPDATE runs SET meta = ?1, updated_at = ?2 WHERE id = ?3",
            params![encoded, clock::now_millis(), id],
        )
        .map_err(db)?;
        Ok(())
    }

    pub fn create_goal(
        &self,
        project_id: Option<&str>,
        run_id: Option<&str>,
        text: &str,
        work_type: Option<&str>,
    ) -> Result<String> {
        let id = ids::new_id("goal");
        let conn = self.lock();
        conn.execute(
            "INSERT INTO goals (id, project_id, run_id, text, work_type, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, project_id, run_id, text, work_type, clock::now_millis()],
        )
        .map_err(db)?;
        Ok(id)
    }

    pub fn save_artifact(
        &self,
        run_id: Option<&str>,
        goal_id: Option<&str>,
        kind: &str,
        content: &str,
    ) -> Result<String> {
        let id = ids::new_id(kind);
        let now = clock::now_millis();
        let conn = self.lock();
        match kind {
            "specification" => {
                conn.execute(
                    "INSERT INTO specifications (id, goal_id, content, status, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![id, goal_id, content, "draft", now],
                )
                .map_err(db)?;
            }
            "plan" => {
                conn.execute(
                    "INSERT INTO plans (id, goal_id, content, status, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![id, goal_id, content, "draft", now],
                )
                .map_err(db)?;
            }
            _ => {
                conn.execute(
                    "INSERT INTO artifacts (id, run_id, task_id, kind, path, meta, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![id, run_id, Option::<String>::None, kind, content, Option::<String>::None, now],
                )
                .map_err(db)?;
            }
        }
        Ok(id)
    }

    pub fn insert_task(&self, task: &NewTask) -> Result<TaskRecord> {
        let now = clock::now_millis();
        let id = task.id.clone().unwrap_or_else(|| ids::new_id("task"));
        let conn = self.lock();
        conn.execute(
            "INSERT INTO tasks (id, run_id, parent_id, title, description, objective, acceptance_criteria, status, priority, risk, complexity, capability, provider, model, workspace, attempts, max_attempts, verify_required, evidence_required, ui, claimed_by, notes, created_at, updated_at, started_at, finished_at, meta)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, 0, ?16, ?17, ?18, ?19, NULL, NULL, ?20, ?21, NULL, NULL, ?22)",
            params![
                id,
                task.run_id,
                task.parent_id,
                task.title,
                task.description,
                task.objective,
                task.acceptance_criteria,
                TaskStatus::Pending.as_str(),
                task.priority,
                task.risk,
                task.complexity,
                task.capability,
                task.provider,
                task.model,
                Option::<String>::None,
                task.max_attempts,
                task.verify_required as i64,
                task.evidence_required as i64,
                task.ui as i64,
                now,
                now,
                task.meta
            ],
        )
        .map_err(db)?;
        drop(conn);
        for dependency in &task.depends_on {
            self.add_dependency(&id, dependency)?;
        }
        self.get_task(&id)?
            .ok_or_else(|| MajstackError::State(format!("task {id} missing after insert")))
    }

    pub fn add_dependency(&self, task_id: &str, depends_on: &str) -> Result<()> {
        if task_id == depends_on {
            return Err(MajstackError::Invalid(
                "a task cannot depend on itself".into(),
            ));
        }
        let conn = self.lock();
        conn.execute(
            "INSERT OR IGNORE INTO task_dependencies (task_id, depends_on) VALUES (?1, ?2)",
            params![task_id, depends_on],
        )
        .map_err(db)?;
        Ok(())
    }

    pub fn dependencies_of(&self, task_id: &str) -> Result<Vec<String>> {
        let conn = self.lock();
        let mut stmt = conn
            .prepare("SELECT depends_on FROM task_dependencies WHERE task_id = ?1")
            .map_err(db)?;
        let rows = stmt
            .query_map(params![task_id], |row| row.get(0))
            .map_err(db)?;
        rows.collect::<rusqlite::Result<Vec<String>>>().map_err(db)
    }

    pub fn all_dependencies(&self, run_id: &str) -> Result<Vec<(String, String)>> {
        let conn = self.lock();
        let mut stmt = conn
            .prepare(
                "SELECT d.task_id, d.depends_on FROM task_dependencies d JOIN tasks t ON t.id = d.task_id WHERE t.run_id = ?1",
            )
            .map_err(db)?;
        let rows = stmt
            .query_map(params![run_id], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(db)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(db)
    }

    pub fn get_task(&self, id: &str) -> Result<Option<TaskRecord>> {
        let conn = self.lock();
        conn.query_row(
            "SELECT * FROM tasks WHERE id = ?1",
            params![id],
            row_to_task,
        )
        .optional()
        .map_err(db)
    }

    pub fn list_tasks(&self, run_id: &str) -> Result<Vec<TaskRecord>> {
        let conn = self.lock();
        let mut stmt = conn
            .prepare("SELECT * FROM tasks WHERE run_id = ?1 ORDER BY priority ASC, created_at ASC")
            .map_err(db)?;
        let rows = stmt.query_map(params![run_id], row_to_task).map_err(db)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(db)
    }

    pub fn update_task_status(
        &self,
        id: &str,
        status: TaskStatus,
        notes: Option<&str>,
    ) -> Result<()> {
        let conn = self.lock();
        let now = clock::now_millis();
        let started = if matches!(status, TaskStatus::Running | TaskStatus::Claimed) {
            Some(now)
        } else {
            None
        };
        let finished = if status.is_terminal() {
            Some(now)
        } else {
            None
        };
        conn.execute(
            "UPDATE tasks SET status = ?1, notes = COALESCE(?2, notes), updated_at = ?3, started_at = COALESCE(started_at, ?4), finished_at = COALESCE(?5, finished_at), claimed_by = CASE WHEN ?1 IN ('claimed','running') THEN claimed_by ELSE NULL END WHERE id = ?6",
            params![status.as_str(), notes, now, started, finished, id],
        )
        .map_err(db)?;
        Ok(())
    }

    pub fn claim_task(&self, id: &str, agent: &str) -> Result<bool> {
        let conn = self.lock();
        let changed = conn
            .execute(
                "UPDATE tasks SET status = 'claimed', claimed_by = ?1, started_at = ?2, updated_at = ?2 WHERE id = ?3 AND status IN ('pending','ready','blocked')",
                params![agent, clock::now_millis(), id],
            )
            .map_err(db)?;
        Ok(changed == 1)
    }

    pub fn release_task(&self, id: &str, status: TaskStatus) -> Result<()> {
        let conn = self.lock();
        conn.execute(
            "UPDATE tasks SET status = ?1, claimed_by = NULL, updated_at = ?2 WHERE id = ?3",
            params![status.as_str(), clock::now_millis(), id],
        )
        .map_err(db)?;
        Ok(())
    }

    pub fn set_task_attempts(&self, id: &str, attempts: i64) -> Result<()> {
        let conn = self.lock();
        conn.execute(
            "UPDATE tasks SET attempts = ?1, updated_at = ?2 WHERE id = ?3",
            params![attempts, clock::now_millis(), id],
        )
        .map_err(db)?;
        Ok(())
    }

    pub fn set_task_workspace(&self, id: &str, workspace: &str) -> Result<()> {
        let conn = self.lock();
        conn.execute(
            "UPDATE tasks SET workspace = ?1, updated_at = ?2 WHERE id = ?3",
            params![workspace, clock::now_millis(), id],
        )
        .map_err(db)?;
        Ok(())
    }

    pub fn insert_attempt(
        &self,
        task_id: &str,
        run_id: Option<&str>,
        attempt_no: i64,
        agent_id: Option<&str>,
        provider: Option<&str>,
        model: Option<&str>,
        status: &str,
        output_log: Option<&str>,
        error: Option<&str>,
    ) -> Result<String> {
        let id = ids::new_id("attempt");
        let now = clock::now_millis();
        let finished = if status != "running" { Some(now) } else { None };
        let conn = self.lock();
        conn.execute(
            "INSERT INTO task_attempts (id, task_id, run_id, attempt_no, agent_id, provider, model, status, output_log, error, started_at, finished_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![id, task_id, run_id, attempt_no, agent_id, provider, model, status, output_log, error, now, finished],
        )
        .map_err(db)?;
        Ok(id)
    }

    pub fn append_event(&self, event: &Event) -> Result<()> {
        let payload = serde_json::to_string(&event.payload)?;
        let conn = self.lock();
        conn.execute(
            "INSERT OR REPLACE INTO events (id, run_id, kind, ts, payload) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![event.id, event.run_id, event.kind.as_str(), event.ts, payload],
        )
        .map_err(db)?;
        Ok(())
    }

    pub fn list_events(&self, run_id: &str, limit: i64) -> Result<Vec<Event>> {
        let conn = self.lock();
        let mut stmt = conn
            .prepare("SELECT id, run_id, kind, ts, payload FROM events WHERE run_id = ?1 ORDER BY ts ASC LIMIT ?2")
            .map_err(db)?;
        let rows = stmt
            .query_map(params![run_id, limit], |row| {
                let kind: String = row.get(2)?;
                let payload: Option<String> = row.get(4)?;
                Ok(Event {
                    id: row.get(0)?,
                    run_id: row.get(1)?,
                    ts: row.get(3)?,
                    kind: EventKind::parse(&kind).unwrap_or(EventKind::Message),
                    payload: payload
                        .and_then(|text| serde_json::from_str(&text).ok())
                        .unwrap_or(serde_json::Value::Null),
                })
            })
            .map_err(db)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(db)
    }

    pub fn insert_decision(
        &self,
        run_id: Option<&str>,
        task_id: Option<&str>,
        decision: &str,
        alternatives: Option<&str>,
        evidence: Option<&str>,
        agent: Option<&str>,
        result: Option<&str>,
    ) -> Result<String> {
        let id = ids::new_id("decision");
        let conn = self.lock();
        conn.execute(
            "INSERT INTO decisions (id, run_id, task_id, decision, alternatives, evidence, agent, result, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![id, run_id, task_id, decision, alternatives, evidence, agent, result, clock::now_millis()],
        )
        .map_err(db)?;
        Ok(id)
    }

    pub fn list_decisions(&self, run_id: &str) -> Result<Vec<DecisionRecord>> {
        let conn = self.lock();
        let mut stmt = conn
            .prepare("SELECT id, run_id, task_id, decision, alternatives, evidence, agent, result, created_at FROM decisions WHERE run_id = ?1 ORDER BY created_at ASC")
            .map_err(db)?;
        let rows = stmt
            .query_map(params![run_id], |row| {
                Ok(DecisionRecord {
                    id: row.get(0)?,
                    run_id: row.get(1)?,
                    task_id: row.get(2)?,
                    decision: row.get(3)?,
                    alternatives: row.get(4)?,
                    evidence: row.get(5)?,
                    agent: row.get(6)?,
                    result: row.get(7)?,
                    created_at: row.get(8)?,
                })
            })
            .map_err(db)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(db)
    }

    pub fn list_all_decisions(&self, limit: i64) -> Result<Vec<DecisionRecord>> {
        let conn = self.lock();
        let mut stmt = conn
            .prepare("SELECT id, run_id, task_id, decision, alternatives, evidence, agent, result, created_at FROM decisions ORDER BY created_at DESC LIMIT ?1")
            .map_err(db)?;
        let rows = stmt
            .query_map(params![limit], |row| {
                Ok(DecisionRecord {
                    id: row.get(0)?,
                    run_id: row.get(1)?,
                    task_id: row.get(2)?,
                    decision: row.get(3)?,
                    alternatives: row.get(4)?,
                    evidence: row.get(5)?,
                    agent: row.get(6)?,
                    result: row.get(7)?,
                    created_at: row.get(8)?,
                })
            })
            .map_err(db)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(db)
    }

    pub fn insert_evidence(
        &self,
        run_id: Option<&str>,
        task_id: Option<&str>,
        label: &str,
        command: Option<&str>,
        code: Option<i64>,
        fingerprint: Option<&str>,
        log_path: Option<&str>,
    ) -> Result<String> {
        let id = ids::new_id("evidence");
        let conn = self.lock();
        conn.execute(
            "INSERT INTO evidence (id, run_id, task_id, label, command, code, fingerprint, log_path, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![id, run_id, task_id, label, command, code, fingerprint, log_path, clock::now_millis()],
        )
        .map_err(db)?;
        Ok(id)
    }

    pub fn latest_evidence(&self, label: &str) -> Result<Option<EvidenceRecord>> {
        let conn = self.lock();
        conn.query_row(
            "SELECT id, run_id, task_id, label, command, code, fingerprint, log_path, created_at FROM evidence WHERE label = ?1 ORDER BY created_at DESC, rowid DESC LIMIT 1",
            params![label],
            |row| {
                Ok(EvidenceRecord {
                    id: row.get(0)?,
                    run_id: row.get(1)?,
                    task_id: row.get(2)?,
                    label: row.get(3)?,
                    command: row.get(4)?,
                    code: row.get(5)?,
                    fingerprint: row.get(6)?,
                    log_path: row.get(7)?,
                    created_at: row.get(8)?,
                })
            },
        )
        .optional()
        .map_err(db)
    }

    pub fn insert_finding(&self, finding: &FindingRecord) -> Result<String> {
        let id = if finding.id.is_empty() {
            ids::new_id("finding")
        } else {
            finding.id.clone()
        };
        let conn = self.lock();
        conn.execute(
            "INSERT INTO review_findings (id, run_id, task_id, reviewer, severity, category, file, line, description, evidence, recommendation, blocking, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                id,
                finding.run_id,
                finding.task_id,
                finding.reviewer,
                finding.severity,
                finding.category,
                finding.file,
                finding.line,
                finding.description,
                finding.evidence,
                finding.recommendation,
                finding.blocking as i64,
                finding.created_at
            ],
        )
        .map_err(db)?;
        Ok(id)
    }

    pub fn list_findings(&self, run_id: &str) -> Result<Vec<FindingRecord>> {
        let conn = self.lock();
        let mut stmt = conn
            .prepare("SELECT id, run_id, task_id, reviewer, severity, category, file, line, description, evidence, recommendation, blocking, created_at FROM review_findings WHERE run_id = ?1 ORDER BY created_at ASC")
            .map_err(db)?;
        let rows = stmt
            .query_map(params![run_id], |row| {
                Ok(FindingRecord {
                    id: row.get(0)?,
                    run_id: row.get(1)?,
                    task_id: row.get(2)?,
                    reviewer: row.get(3)?,
                    severity: row.get(4)?,
                    category: row.get(5)?,
                    file: row.get(6)?,
                    line: row.get(7)?,
                    description: row.get(8)?,
                    evidence: row.get(9)?,
                    recommendation: row.get(10)?,
                    blocking: row.get::<_, i64>(11)? != 0,
                    created_at: row.get(12)?,
                })
            })
            .map_err(db)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(db)
    }

    pub fn insert_failure(
        &self,
        run_id: Option<&str>,
        task_id: Option<&str>,
        class: &str,
        message: &str,
        evidence: Option<&str>,
    ) -> Result<String> {
        let id = ids::new_id("failure");
        let conn = self.lock();
        conn.execute(
            "INSERT INTO failures (id, run_id, task_id, class, message, evidence, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, run_id, task_id, class, message, evidence, clock::now_millis()],
        )
        .map_err(db)?;
        Ok(id)
    }

    pub fn insert_tool_call(
        &self,
        run_id: Option<&str>,
        task_id: Option<&str>,
        tool: &str,
        args: Option<&str>,
        result: Option<&str>,
        permission: Option<&str>,
        status: &str,
        duration_ms: Option<i64>,
    ) -> Result<String> {
        let id = ids::new_id("tool");
        let conn = self.lock();
        conn.execute(
            "INSERT INTO tool_calls (id, run_id, task_id, tool, args, result, permission, status, duration_ms, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![id, run_id, task_id, tool, args, result, permission, status, duration_ms, clock::now_millis()],
        )
        .map_err(db)?;
        Ok(id)
    }

    pub fn insert_commit(
        &self,
        run_id: Option<&str>,
        task_id: Option<&str>,
        sha: &str,
        message: &str,
    ) -> Result<String> {
        let id = ids::new_id("commit");
        let conn = self.lock();
        conn.execute(
            "INSERT INTO commits (id, run_id, task_id, sha, message, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, run_id, task_id, sha, message, clock::now_millis()],
        )
        .map_err(db)?;
        Ok(id)
    }

    pub fn insert_worktree(
        &self,
        run_id: Option<&str>,
        task_id: Option<&str>,
        path: &str,
        branch: Option<&str>,
        status: &str,
    ) -> Result<String> {
        let id = ids::new_id("wt");
        let conn = self.lock();
        conn.execute(
            "INSERT INTO worktrees (id, run_id, task_id, path, branch, status, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, run_id, task_id, path, branch, status, clock::now_millis()],
        )
        .map_err(db)?;
        Ok(id)
    }

    pub fn insert_recovery_action(
        &self,
        run_id: Option<&str>,
        task_id: Option<&str>,
        action: &str,
        reason: Option<&str>,
        applied: bool,
    ) -> Result<String> {
        let id = ids::new_id("recovery");
        let conn = self.lock();
        conn.execute(
            "INSERT INTO recovery_actions (id, run_id, task_id, action, reason, applied, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, run_id, task_id, action, reason, applied as i64, clock::now_millis()],
        )
        .map_err(db)?;
        Ok(id)
    }

    pub fn insert_discovery(
        &self,
        run_id: Option<&str>,
        kind: &str,
        text: &str,
        tags: &[String],
    ) -> Result<String> {
        let id = ids::new_id("disc");
        let tags_json = serde_json::to_string(tags)?;
        let conn = self.lock();
        conn.execute(
            "INSERT INTO discoveries (id, run_id, kind, text, tags, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, run_id, kind, text, tags_json, clock::now_millis()],
        )
        .map_err(db)?;
        conn.execute(
            "INSERT INTO knowledge_fts (id, text) VALUES (?1, ?2)",
            params![id, text],
        )
        .map_err(db)?;
        Ok(id)
    }

    pub fn search_knowledge(&self, query: &str, limit: i64) -> Result<Vec<String>> {
        let conn = self.lock();
        let mut stmt = conn
            .prepare("SELECT text FROM knowledge_fts WHERE knowledge_fts MATCH ?1 LIMIT ?2")
            .map_err(db)?;
        let rows = stmt
            .query_map(params![query, limit], |row| row.get(0))
            .map_err(db)?;
        rows.collect::<rusqlite::Result<Vec<String>>>().map_err(db)
    }

    pub fn insert_skill_invocation(
        &self,
        run_id: Option<&str>,
        task_id: Option<&str>,
        skill: &str,
        agent: Option<&str>,
        status: &str,
        output_log: Option<&str>,
    ) -> Result<String> {
        let id = ids::new_id("skill");
        let now = clock::now_millis();
        let finished = if status != "running" { Some(now) } else { None };
        let conn = self.lock();
        conn.execute(
            "INSERT INTO skill_invocations (id, run_id, task_id, skill, agent, status, output_log, started_at, finished_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![id, run_id, task_id, skill, agent, status, output_log, now, finished],
        )
        .map_err(db)?;
        Ok(id)
    }

    pub fn insert_provider_run(
        &self,
        run_id: Option<&str>,
        provider: &str,
        model: Option<&str>,
        status: &str,
        usage_json: &str,
    ) -> Result<String> {
        let id = ids::new_id("provider");
        let now = clock::now_millis();
        let finished = if status != "running" { Some(now) } else { None };
        let conn = self.lock();
        conn.execute(
            "INSERT INTO provider_runs (id, run_id, provider, model, status, usage, started_at, finished_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![id, run_id, provider, model, status, usage_json, now, finished],
        )
        .map_err(db)?;
        Ok(id)
    }

    pub fn insert_verification_result(
        &self,
        run_id: &str,
        task_id: &str,
        name: &str,
        status: &str,
        command: &str,
        output: &str,
        duration_ms: u64,
    ) -> Result<String> {
        let id = ids::new_id("vresult");
        let conn = self.lock();
        conn.execute(
            "INSERT INTO verification_results (id, run_id, task_id, name, status, command, output, duration_ms, artifacts, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, ?9)",
            params![id, run_id, task_id, name, status, command, output, duration_ms as i64, clock::now_millis()],
        )
        .map_err(db)?;
        Ok(id)
    }

    pub fn insert_journal(
        &self,
        run_id: &str,
        task_id: Option<&str>,
        iteration: i64,
        outcome: &str,
        provider: Option<&str>,
        model: Option<&str>,
        duration_ms: i64,
        cost_usd: f64,
        files_changed: i64,
        notes: &str,
    ) -> Result<String> {
        let id = ids::new_id("journal");
        let conn = self.lock();
        conn.execute(
            "INSERT INTO run_journal (id, run_id, task_id, iteration, outcome, provider, model, duration_ms, cost_usd, files_changed, notes, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![id, run_id, task_id, iteration, outcome, provider, model, duration_ms, cost_usd, files_changed, notes, clock::now_millis()],
        )
        .map_err(db)?;
        conn.execute(
            "INSERT INTO journal_fts (id, text) VALUES (?1, ?2)",
            params![id, format!("{outcome} {notes}")],
        )
        .map_err(db)?;
        Ok(id)
    }

    pub fn recent_journal(&self, run_id: &str, limit: i64) -> Result<Vec<JournalRecord>> {
        let conn = self.lock();
        let mut stmt = conn
            .prepare("SELECT id, run_id, task_id, iteration, outcome, provider, model, duration_ms, cost_usd, files_changed, notes, created_at FROM run_journal WHERE run_id = ?1 ORDER BY created_at DESC LIMIT ?2")
            .map_err(db)?;
        let rows = stmt
            .query_map(params![run_id, limit], |row| {
                Ok(JournalRecord {
                    id: row.get(0)?,
                    run_id: row.get(1)?,
                    task_id: row.get(2)?,
                    iteration: row.get(3)?,
                    outcome: row.get(4)?,
                    provider: row.get(5)?,
                    model: row.get(6)?,
                    duration_ms: row.get(7)?,
                    cost_usd: row.get(8)?,
                    files_changed: row.get(9)?,
                    notes: row.get(10)?,
                    created_at: row.get(11)?,
                })
            })
            .map_err(db)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(db)
    }

    pub fn search_journal(&self, query: &str, limit: i64) -> Result<Vec<String>> {
        let conn = self.lock();
        let mut stmt = conn
            .prepare("SELECT text FROM journal_fts WHERE journal_fts MATCH ?1 LIMIT ?2")
            .map_err(db)?;
        let rows = stmt
            .query_map(params![query, limit], |row| row.get(0))
            .map_err(db)?;
        rows.collect::<rusqlite::Result<Vec<String>>>().map_err(db)
    }

    pub fn create_feature(
        &self,
        run_id: Option<&str>,
        name: &str,
        slug: &str,
        spec_path: Option<&str>,
        plan_path: Option<&str>,
    ) -> Result<FeatureRecord> {
        let now = clock::now_millis();
        let record = FeatureRecord {
            id: ids::new_id("feature"),
            run_id: run_id.map(str::to_string),
            name: name.to_string(),
            slug: slug.to_string(),
            spec_path: spec_path.map(str::to_string),
            plan_path: plan_path.map(str::to_string),
            status: "draft".to_string(),
            created_at: now,
            updated_at: now,
        };
        let conn = self.lock();
        conn.execute(
            "INSERT INTO features (id, run_id, name, slug, spec_path, plan_path, status, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![record.id, record.run_id, record.name, record.slug, record.spec_path, record.plan_path, record.status, record.created_at, record.updated_at],
        )
        .map_err(db)?;
        Ok(record)
    }

    fn feature_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<FeatureRecord> {
        Ok(FeatureRecord {
            id: row.get(0)?,
            run_id: row.get(1)?,
            name: row.get(2)?,
            slug: row.get(3)?,
            spec_path: row.get(4)?,
            plan_path: row.get(5)?,
            status: row.get(6)?,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
        })
    }

    pub fn get_feature(&self, id: &str) -> Result<Option<FeatureRecord>> {
        let conn = self.lock();
        conn.query_row(
            "SELECT id, run_id, name, slug, spec_path, plan_path, status, created_at, updated_at FROM features WHERE id = ?1",
            params![id],
            Self::feature_from_row,
        )
        .optional()
        .map_err(db)
    }

    pub fn get_feature_by_run(&self, run_id: &str) -> Result<Option<FeatureRecord>> {
        let conn = self.lock();
        conn.query_row(
            "SELECT id, run_id, name, slug, spec_path, plan_path, status, created_at, updated_at FROM features WHERE run_id = ?1 ORDER BY created_at DESC LIMIT 1",
            params![run_id],
            Self::feature_from_row,
        )
        .optional()
        .map_err(db)
    }

    pub fn list_features(&self) -> Result<Vec<FeatureRecord>> {
        let conn = self.lock();
        let mut stmt = conn
            .prepare("SELECT id, run_id, name, slug, spec_path, plan_path, status, created_at, updated_at FROM features ORDER BY created_at DESC")
            .map_err(db)?;
        let rows = stmt.query_map([], Self::feature_from_row).map_err(db)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(db)
    }

    pub fn update_feature_status(&self, id: &str, status: &str) -> Result<()> {
        let conn = self.lock();
        conn.execute(
            "UPDATE features SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![status, clock::now_millis(), id],
        )
        .map_err(db)?;
        Ok(())
    }

    pub fn insert_principle_invocation(
        &self,
        run_id: Option<&str>,
        task_id: Option<&str>,
        principle: &str,
        stage: &str,
    ) -> Result<String> {
        let id = ids::new_id("principle");
        let conn = self.lock();
        conn.execute(
            "INSERT INTO principle_invocations (id, run_id, task_id, principle, stage, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, run_id, task_id, principle, stage, clock::now_millis()],
        )
        .map_err(db)?;
        Ok(id)
    }

    pub fn list_principle_invocations(
        &self,
        run_id: &str,
        limit: i64,
    ) -> Result<Vec<(String, String)>> {
        let conn = self.lock();
        let mut stmt = conn
            .prepare("SELECT principle, stage FROM principle_invocations WHERE run_id = ?1 ORDER BY created_at ASC LIMIT ?2")
            .map_err(db)?;
        let rows = stmt
            .query_map(params![run_id, limit], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(db)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(db)
    }

    pub fn get_goal_text(&self, run_id: &str) -> Result<Option<String>> {
        let conn = self.lock();
        conn.query_row(
            "SELECT text FROM goals WHERE run_id = ?1 ORDER BY created_at DESC LIMIT 1",
            params![run_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(db)
    }

    pub fn recover_interrupted(&self) -> Result<RecoveryReport> {
        let conn = self.lock();
        let tasks_reset = conn
            .execute(
                "UPDATE tasks SET status = 'pending', claimed_by = NULL, updated_at = ?1 WHERE status IN ('claimed','running','verifying','reviewing','repairing')",
                params![clock::now_millis()],
            )
            .map_err(db)?;
        let runs_suspended = conn
            .execute(
                "UPDATE runs SET state = 'paused', updated_at = ?1 WHERE state NOT IN ('completed','failed','cancelled','paused')",
                params![clock::now_millis()],
            )
            .map_err(db)?;
        Ok(RecoveryReport {
            tasks_reset,
            runs_suspended,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_task(run: &str, title: &str, depends_on: Vec<String>) -> NewTask {
        NewTask {
            run_id: run.to_string(),
            title: title.to_string(),
            verify_required: true,
            depends_on,
            ..NewTask::default()
        }
    }

    #[test]
    fn stores_and_reads_tasks() {
        let store = Store::memory().unwrap();
        let run = store.create_run(None, None, None, None, None).unwrap();
        let first = store
            .insert_task(&new_task(&run.id, "one", vec![]))
            .unwrap();
        let second = store
            .insert_task(&new_task(&run.id, "two", vec![first.id.clone()]))
            .unwrap();
        assert_eq!(store.list_tasks(&run.id).unwrap().len(), 2);
        assert_eq!(
            store.dependencies_of(&second.id).unwrap(),
            vec![first.id.clone()]
        );
    }

    #[test]
    fn claims_are_atomic() {
        let store = Store::memory().unwrap();
        let run = store.create_run(None, None, None, None, None).unwrap();
        let task = store
            .insert_task(&new_task(&run.id, "one", vec![]))
            .unwrap();
        assert!(store.claim_task(&task.id, "agent-1").unwrap());
        assert!(!store.claim_task(&task.id, "agent-2").unwrap());
    }

    #[test]
    fn recovers_interrupted_state() {
        let store = Store::memory().unwrap();
        let run = store.create_run(None, None, None, None, None).unwrap();
        let task = store
            .insert_task(&new_task(&run.id, "one", vec![]))
            .unwrap();
        store.claim_task(&task.id, "agent-1").unwrap();
        store
            .update_run_state(&run.id, RunState::Executing)
            .unwrap();
        let report = store.recover_interrupted().unwrap();
        assert_eq!(report.tasks_reset, 1);
        assert_eq!(report.runs_suspended, 1);
        assert_eq!(
            store.get_task(&task.id).unwrap().unwrap().status,
            TaskStatus::Pending
        );
        assert_eq!(
            store.get_run(&run.id).unwrap().unwrap().state,
            RunState::Paused
        );
    }

    #[test]
    fn events_roundtrip() {
        let store = Store::memory().unwrap();
        let run = store.create_run(None, None, None, None, None).unwrap();
        let event =
            Event::new(EventKind::RunCreated, serde_json::json!({"k": 1})).with_run(&run.id);
        store.append_event(&event).unwrap();
        let events = store.list_events(&run.id, 10).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, EventKind::RunCreated);
    }

    #[test]
    fn knowledge_search() {
        let store = Store::memory().unwrap();
        store
            .insert_discovery(None, "knowledge", "the build uses vite", &[])
            .unwrap();
        let hits = store.search_knowledge("vite", 5).unwrap();
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn schema_covers_all_entities() {
        let store = Store::memory().unwrap();
        let tables = [
            "projects",
            "runs",
            "goals",
            "specifications",
            "plans",
            "tasks",
            "task_dependencies",
            "task_attempts",
            "agents",
            "agent_runs",
            "workflow_runs",
            "workflow_steps",
            "skills",
            "skill_invocations",
            "decisions",
            "discoveries",
            "artifacts",
            "evidence",
            "verification_runs",
            "verification_results",
            "review_findings",
            "failures",
            "recovery_actions",
            "commits",
            "branches",
            "worktrees",
            "provider_runs",
            "model_runs",
            "tool_calls",
            "browser_sessions",
            "browser_artifacts",
            "human_interventions",
            "events",
        ];
        for table in tables {
            assert_eq!(store.count(table).unwrap(), 0, "missing table {table}");
        }
    }
}
