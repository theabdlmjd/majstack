use majstack_core::{MajstackError, PermissionLevel, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const DIR_NAME: &str = ".majstack";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MajstackPaths {
    pub root: PathBuf,
    pub dir: PathBuf,
}

impl MajstackPaths {
    pub fn new(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();
        let dir = root.join(DIR_NAME);
        MajstackPaths { root, dir }
    }

    pub fn discover(start: impl AsRef<Path>) -> Self {
        let start = start.as_ref();
        let mut current = if start.is_absolute() {
            start.to_path_buf()
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(start)
        };
        if current.is_file() {
            current.pop();
        }
        loop {
            if current.join(DIR_NAME).is_dir() {
                return MajstackPaths::new(&current);
            }
            if !current.pop() {
                break;
            }
        }
        MajstackPaths::new(start)
    }

    pub fn db_file(&self) -> PathBuf {
        self.dir.join("majstack.db")
    }
    pub fn config_file(&self) -> PathBuf {
        self.dir.join("config.toml")
    }
    pub fn skills_dir(&self) -> PathBuf {
        self.dir.join("skills")
    }
    pub fn playbooks_dir(&self) -> PathBuf {
        self.dir.join("playbooks")
    }
    pub fn principles_dir(&self) -> PathBuf {
        self.dir.join("principles")
    }
    pub fn logs_dir(&self) -> PathBuf {
        self.dir.join("logs")
    }
    pub fn worktrees_dir(&self) -> PathBuf {
        self.dir.join("worktrees")
    }
    pub fn archive_dir(&self) -> PathBuf {
        self.dir.join("archive")
    }
    pub fn stop_file(&self) -> PathBuf {
        self.dir.join("STOP")
    }
    pub fn ensure(&self) -> Result<()> {
        for dir in [
            self.dir.clone(),
            self.logs_dir(),
            self.worktrees_dir(),
            self.archive_dir(),
        ] {
            std::fs::create_dir_all(&dir)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RunConfig {
    pub agent: String,
    pub verify: String,
    pub max_iterations: u64,
    pub max_attempts_per_task: u64,
    pub timeout_seconds: u64,
    pub commit: bool,
    pub review: bool,
    pub branching: bool,
    pub swarm_workers: usize,
    pub babysit_rounds: u64,
    pub pause_between_seconds: u64,
    pub concurrency: usize,
}

impl Default for RunConfig {
    fn default() -> Self {
        RunConfig {
            agent: "claude".into(),
            verify: String::new(),
            max_iterations: 0,
            max_attempts_per_task: 0,
            timeout_seconds: 0,
            commit: true,
            review: true,
            branching: true,
            swarm_workers: 3,
            babysit_rounds: 0,
            pause_between_seconds: 0,
            concurrency: 4,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PipelineConfig {
    pub plan: Vec<String>,
    pub per_task: Vec<String>,
    pub finish: Vec<String>,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        PipelineConfig {
            plan: vec!["plan".into(), "architect".into()],
            per_task: vec!["build".into(), "review".into()],
            finish: vec!["security-audit".into()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BrowserConfig {
    pub ui_check: bool,
    pub engine: String,
    pub headless: bool,
    pub timeout_seconds: u64,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        BrowserConfig {
            ui_check: true,
            engine: "chrome".into(),
            headless: true,
            timeout_seconds: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ReleaseConfig {
    pub gate: Vec<String>,
    pub deploy_cmd: String,
    pub canary_url: String,
    pub canary_seconds: u64,
}

impl Default for ReleaseConfig {
    fn default() -> Self {
        ReleaseConfig {
            gate: vec!["verify".into(), "review".into()],
            deploy_cmd: String::new(),
            canary_url: String::new(),
            canary_seconds: 60,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PanelConfig {
    pub agents: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PermissionsConfig {
    pub default: Option<PermissionLevel>,
    pub allow_dangerous: bool,
    pub freeze: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RoutingRule {
    pub agent: Option<String>,
    pub model: Option<String>,
    pub provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentSpec {
    pub mode: String,
    pub cmd: Vec<String>,
    pub model_args: Vec<String>,
    pub instructions_file: String,
    pub kind: majstack_core::ProviderKind,
    pub env: BTreeMap<String, String>,
}

impl Default for AgentSpec {
    fn default() -> Self {
        AgentSpec {
            mode: "exec".into(),
            cmd: Vec::new(),
            model_args: Vec::new(),
            instructions_file: "AGENTS.md".into(),
            kind: majstack_core::ProviderKind::Cli,
            env: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GitHubConfig {
    pub enabled: bool,
    pub remote: String,
    pub base_branch: String,
    pub draft: bool,
}

impl Default for GitHubConfig {
    fn default() -> Self {
        GitHubConfig {
            enabled: false,
            remote: "origin".into(),
            base_branch: "main".into(),
            draft: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    pub level: String,
    pub json: bool,
    pub redact: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        LoggingConfig {
            level: "info".into(),
            json: false,
            redact: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TypedConfig {
    pub enabled: bool,
    pub provider: String,
    pub base_url: String,
    pub model: String,
    pub api_key_env: String,
    pub confidence_threshold: f64,
}

impl Default for TypedConfig {
    fn default() -> Self {
        TypedConfig {
            enabled: false,
            provider: "typesafe".into(),
            base_url: "https://api.typesafe.ai/v1/systemone".into(),
            model: "jev-latest".into(),
            api_key_env: "TYPESAFE_API_KEY".into(),
            confidence_threshold: 0.6,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RolesConfig {
    pub code: Option<String>,
    pub judgment: Option<String>,
    pub review: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MajstackConfig {
    pub run: RunConfig,
    pub pipeline: PipelineConfig,
    pub browser: BrowserConfig,
    pub release: ReleaseConfig,
    pub panel: PanelConfig,
    pub permissions: PermissionsConfig,
    pub github: GitHubConfig,
    pub logging: LoggingConfig,
    pub typed: TypedConfig,
    pub roles: RolesConfig,
    pub routing: BTreeMap<String, RoutingRule>,
    pub agents: BTreeMap<String, AgentSpec>,
}

impl MajstackConfig {
    pub fn load(paths: &MajstackPaths) -> Result<Self> {
        let file = paths.config_file();
        if !file.exists() {
            return Ok(Self::with_builtin_agents());
        }
        let text = std::fs::read_to_string(&file)?;
        let config: MajstackConfig = toml::from_str(&text)
            .map_err(|e| MajstackError::Config(format!("{}: {e}", file.display())))?;
        Ok(config.complete())
    }

    fn complete(mut self) -> Self {
        let builtin = Self::with_builtin_agents();
        if self.agents.is_empty() {
            self.agents = builtin.agents;
        }
        self
    }

    pub fn with_builtin_agents() -> Self {
        let mut agents = BTreeMap::new();
        agents.insert(
            "claude".to_string(),
            AgentSpec {
                mode: "exec".into(),
                cmd: vec!["claude".into(), "-p".into(), "{prompt}".into()],
                model_args: vec!["--model".into(), "{model}".into()],
                instructions_file: "CLAUDE.md".into(),
                ..AgentSpec::default()
            },
        );
        agents.insert(
            "codex".to_string(),
            AgentSpec {
                mode: "exec".into(),
                cmd: vec!["codex".into(), "exec".into(), "{prompt}".into()],
                model_args: vec!["--model".into(), "{model}".into()],
                instructions_file: "AGENTS.md".into(),
                ..AgentSpec::default()
            },
        );
        agents.insert(
            "opencode".to_string(),
            AgentSpec {
                mode: "exec".into(),
                cmd: vec!["opencode".into(), "run".into(), "{prompt}".into()],
                model_args: vec!["--model".into(), "{model}".into()],
                instructions_file: "AGENTS.md".into(),
                ..AgentSpec::default()
            },
        );
        agents.insert(
            "handoff".to_string(),
            AgentSpec {
                mode: "handoff".into(),
                instructions_file: "AGENTS.md".into(),
                ..AgentSpec::default()
            },
        );
        MajstackConfig {
            run: RunConfig::default(),
            pipeline: PipelineConfig::default(),
            browser: BrowserConfig::default(),
            release: ReleaseConfig::default(),
            panel: PanelConfig::default(),
            permissions: PermissionsConfig::default(),
            github: GitHubConfig::default(),
            logging: LoggingConfig::default(),
            typed: TypedConfig::default(),
            roles: RolesConfig::default(),
            routing: BTreeMap::new(),
            agents,
        }
    }

    pub fn agent(&self, name: &str) -> Result<&AgentSpec> {
        self.agents
            .get(name)
            .ok_or_else(|| MajstackError::Config(format!("unknown agent '{name}'")))
    }

    pub fn validate(&self) -> Result<()> {
        if self.run.agent.is_empty() {
            return Err(MajstackError::Config("run.agent must not be empty".into()));
        }
        if !self.agents.contains_key(&self.run.agent) {
            return Err(MajstackError::Config(format!(
                "run.agent '{}' is not defined in [agents.*]",
                self.run.agent
            )));
        }
        for (name, spec) in &self.agents {
            if spec.mode != "exec" && spec.mode != "handoff" {
                return Err(MajstackError::Config(format!(
                    "agent '{name}' has invalid mode '{}'",
                    spec.mode
                )));
            }
            if spec.mode == "exec" && spec.cmd.is_empty() {
                return Err(MajstackError::Config(format!(
                    "agent '{name}' is exec but has no cmd"
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_have_agents() {
        let config = MajstackConfig::with_builtin_agents();
        config.validate().unwrap();
        assert!(config.agents.contains_key("claude"));
        assert_eq!(config.run.agent, "claude");
    }

    #[test]
    fn paths_layout() {
        let paths = MajstackPaths::new("C:/repo");
        assert!(paths.db_file().ends_with("majstack.db"));
        assert!(paths.skills_dir().ends_with("skills"));
    }

    #[test]
    fn parses_toml() {
        let text = r#"
[run]
agent = "codex"
max_iterations = 5
[agents.codex]
mode = "exec"
cmd = ["codex", "exec", "{prompt}"]
"#;
        let config: MajstackConfig = toml::from_str(text).unwrap();
        config.validate().unwrap();
        assert_eq!(config.run.max_iterations, 5);
    }
}
