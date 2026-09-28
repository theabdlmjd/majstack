pub const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS projects (
  id TEXT PRIMARY KEY,
  root TEXT NOT NULL,
  name TEXT NOT NULL,
  created_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS runs (
  id TEXT PRIMARY KEY,
  project_id TEXT,
  goal_id TEXT,
  state TEXT NOT NULL,
  workflow TEXT,
  provider TEXT,
  model TEXT,
  strategy TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  finished_at INTEGER,
  meta TEXT
);
CREATE TABLE IF NOT EXISTS goals (
  id TEXT PRIMARY KEY,
  project_id TEXT,
  run_id TEXT,
  text TEXT NOT NULL,
  work_type TEXT,
  created_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS specifications (
  id TEXT PRIMARY KEY,
  goal_id TEXT NOT NULL,
  content TEXT NOT NULL,
  status TEXT NOT NULL,
  created_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS plans (
  id TEXT PRIMARY KEY,
  goal_id TEXT NOT NULL,
  content TEXT NOT NULL,
  status TEXT NOT NULL,
  created_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS tasks (
  id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL,
  parent_id TEXT,
  title TEXT NOT NULL,
  description TEXT,
  objective TEXT,
  acceptance_criteria TEXT,
  status TEXT NOT NULL,
  priority INTEGER NOT NULL DEFAULT 0,
  risk TEXT,
  complexity TEXT,
  capability TEXT,
  provider TEXT,
  model TEXT,
  workspace TEXT,
  attempts INTEGER NOT NULL DEFAULT 0,
  max_attempts INTEGER NOT NULL DEFAULT 0,
  verify_required INTEGER NOT NULL DEFAULT 1,
  evidence_required INTEGER NOT NULL DEFAULT 0,
  ui INTEGER NOT NULL DEFAULT 0,
  claimed_by TEXT,
  notes TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  started_at INTEGER,
  finished_at INTEGER,
  meta TEXT
);
CREATE INDEX IF NOT EXISTS idx_tasks_run ON tasks(run_id);
CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
CREATE INDEX IF NOT EXISTS idx_tasks_parent ON tasks(parent_id);
CREATE TABLE IF NOT EXISTS task_dependencies (
  task_id TEXT NOT NULL,
  depends_on TEXT NOT NULL,
  PRIMARY KEY (task_id, depends_on)
);
CREATE TABLE IF NOT EXISTS task_attempts (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL,
  run_id TEXT,
  attempt_no INTEGER NOT NULL,
  agent_id TEXT,
  provider TEXT,
  model TEXT,
  status TEXT NOT NULL,
  output_log TEXT,
  error TEXT,
  started_at INTEGER NOT NULL,
  finished_at INTEGER
);
CREATE INDEX IF NOT EXISTS idx_attempts_task ON task_attempts(task_id);
CREATE TABLE IF NOT EXISTS agents (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  role TEXT NOT NULL,
  provider TEXT,
  model TEXT,
  created_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS agent_runs (
  id TEXT PRIMARY KEY,
  run_id TEXT,
  task_id TEXT,
  agent_id TEXT,
  provider_run_id TEXT,
  status TEXT NOT NULL,
  output_log TEXT,
  started_at INTEGER NOT NULL,
  finished_at INTEGER
);
CREATE TABLE IF NOT EXISTS workflow_runs (
  id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL,
  workflow TEXT NOT NULL,
  status TEXT NOT NULL,
  started_at INTEGER NOT NULL,
  finished_at INTEGER
);
CREATE TABLE IF NOT EXISTS workflow_steps (
  id TEXT PRIMARY KEY,
  workflow_run_id TEXT NOT NULL,
  name TEXT NOT NULL,
  status TEXT NOT NULL,
  output TEXT,
  started_at INTEGER NOT NULL,
  finished_at INTEGER
);
CREATE TABLE IF NOT EXISTS skills (
  name TEXT PRIMARY KEY,
  group_name TEXT,
  summary TEXT,
  path TEXT,
  meta TEXT
);
CREATE TABLE IF NOT EXISTS skill_invocations (
  id TEXT PRIMARY KEY,
  run_id TEXT,
  task_id TEXT,
  skill TEXT NOT NULL,
  agent TEXT,
  status TEXT NOT NULL,
  output_log TEXT,
  started_at INTEGER NOT NULL,
  finished_at INTEGER
);
CREATE TABLE IF NOT EXISTS decisions (
  id TEXT PRIMARY KEY,
  run_id TEXT,
  task_id TEXT,
  decision TEXT NOT NULL,
  alternatives TEXT,
  evidence TEXT,
  agent TEXT,
  result TEXT,
  created_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS discoveries (
  id TEXT PRIMARY KEY,
  run_id TEXT,
  kind TEXT NOT NULL,
  text TEXT NOT NULL,
  tags TEXT,
  created_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS artifacts (
  id TEXT PRIMARY KEY,
  run_id TEXT,
  task_id TEXT,
  kind TEXT NOT NULL,
  path TEXT NOT NULL,
  meta TEXT,
  created_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS evidence (
  id TEXT PRIMARY KEY,
  run_id TEXT,
  task_id TEXT,
  label TEXT NOT NULL,
  command TEXT,
  code INTEGER,
  fingerprint TEXT,
  log_path TEXT,
  created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_evidence_label ON evidence(label);
CREATE TABLE IF NOT EXISTS verification_runs (
  id TEXT PRIMARY KEY,
  run_id TEXT,
  task_id TEXT,
  status TEXT NOT NULL,
  started_at INTEGER NOT NULL,
  finished_at INTEGER
);
CREATE TABLE IF NOT EXISTS verification_results (
  id TEXT PRIMARY KEY,
  run_id TEXT,
  task_id TEXT,
  name TEXT NOT NULL,
  status TEXT NOT NULL,
  command TEXT,
  output TEXT,
  duration_ms INTEGER,
  artifacts TEXT,
  created_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS review_findings (
  id TEXT PRIMARY KEY,
  run_id TEXT,
  task_id TEXT,
  reviewer TEXT NOT NULL,
  severity TEXT NOT NULL,
  category TEXT,
  file TEXT,
  line INTEGER,
  description TEXT,
  evidence TEXT,
  recommendation TEXT,
  blocking INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS failures (
  id TEXT PRIMARY KEY,
  run_id TEXT,
  task_id TEXT,
  class TEXT NOT NULL,
  message TEXT,
  evidence TEXT,
  created_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS recovery_actions (
  id TEXT PRIMARY KEY,
  run_id TEXT,
  task_id TEXT,
  action TEXT NOT NULL,
  reason TEXT,
  applied INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS commits (
  id TEXT PRIMARY KEY,
  run_id TEXT,
  task_id TEXT,
  sha TEXT NOT NULL,
  message TEXT,
  created_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS branches (
  id TEXT PRIMARY KEY,
  run_id TEXT,
  name TEXT NOT NULL,
  base TEXT,
  created_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS worktrees (
  id TEXT PRIMARY KEY,
  run_id TEXT,
  task_id TEXT,
  path TEXT NOT NULL,
  branch TEXT,
  status TEXT NOT NULL,
  created_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS provider_runs (
  id TEXT PRIMARY KEY,
  run_id TEXT,
  provider TEXT NOT NULL,
  model TEXT,
  status TEXT NOT NULL,
  usage TEXT,
  started_at INTEGER NOT NULL,
  finished_at INTEGER
);
CREATE TABLE IF NOT EXISTS model_runs (
  id TEXT PRIMARY KEY,
  provider_run_id TEXT,
  provider TEXT NOT NULL,
  model TEXT,
  input_tokens INTEGER NOT NULL DEFAULT 0,
  output_tokens INTEGER NOT NULL DEFAULT 0,
  cost_usd REAL NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS tool_calls (
  id TEXT PRIMARY KEY,
  run_id TEXT,
  task_id TEXT,
  tool TEXT NOT NULL,
  args TEXT,
  result TEXT,
  permission TEXT,
  status TEXT NOT NULL,
  duration_ms INTEGER,
  created_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS browser_sessions (
  id TEXT PRIMARY KEY,
  run_id TEXT,
  url TEXT,
  engine TEXT,
  status TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  finished_at INTEGER
);
CREATE TABLE IF NOT EXISTS browser_artifacts (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  kind TEXT NOT NULL,
  path TEXT NOT NULL,
  meta TEXT,
  created_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS human_interventions (
  id TEXT PRIMARY KEY,
  run_id TEXT,
  task_id TEXT,
  kind TEXT NOT NULL,
  request TEXT,
  response TEXT,
  created_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS events (
  id TEXT PRIMARY KEY,
  run_id TEXT,
  kind TEXT NOT NULL,
  ts INTEGER NOT NULL,
  payload TEXT
);
CREATE INDEX IF NOT EXISTS idx_events_run ON events(run_id);
CREATE INDEX IF NOT EXISTS idx_events_kind ON events(kind);
CREATE VIRTUAL TABLE IF NOT EXISTS knowledge_fts USING fts5(id UNINDEXED, text);
CREATE TABLE IF NOT EXISTS features (
  id TEXT PRIMARY KEY,
  run_id TEXT,
  name TEXT NOT NULL,
  slug TEXT NOT NULL,
  spec_path TEXT,
  plan_path TEXT,
  status TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS run_journal (
  id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL,
  task_id TEXT,
  iteration INTEGER NOT NULL DEFAULT 0,
  outcome TEXT,
  provider TEXT,
  model TEXT,
  duration_ms INTEGER NOT NULL DEFAULT 0,
  cost_usd REAL NOT NULL DEFAULT 0,
  files_changed INTEGER NOT NULL DEFAULT 0,
  notes TEXT,
  created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_journal_run ON run_journal(run_id);
CREATE VIRTUAL TABLE IF NOT EXISTS journal_fts USING fts5(id UNINDEXED, text);
CREATE TABLE IF NOT EXISTS principle_invocations (
  id TEXT PRIMARY KEY,
  run_id TEXT,
  task_id TEXT,
  principle TEXT NOT NULL,
  stage TEXT NOT NULL,
  created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_principle_run ON principle_invocations(run_id);
"#;
