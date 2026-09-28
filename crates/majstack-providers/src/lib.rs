use majstack_config::{AgentSpec, MajstackConfig};
use majstack_core::{MajstackError, Result, Sigils};
use majstack_execution::{run_process, ProcessSpec};
use majstack_telemetry::Usage;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderRequest {
    pub prompt: String,
    pub cwd: PathBuf,
    pub model: Option<String>,
    pub timeout_seconds: u64,
    pub env: BTreeMap<String, String>,
}

impl ProviderRequest {
    pub fn new(prompt: impl Into<String>, cwd: impl Into<PathBuf>) -> Self {
        ProviderRequest {
            prompt: prompt.into(),
            cwd: cwd.into(),
            model: None,
            timeout_seconds: 0,
            env: BTreeMap::new(),
        }
    }

    pub fn model(mut self, model: Option<String>) -> Self {
        self.model = model;
        self
    }

    pub fn timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = seconds;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderResponse {
    pub provider: String,
    pub model: Option<String>,
    pub output: String,
    pub exit_code: i32,
    pub duration_ms: u64,
    pub timed_out: bool,
    pub needs_human: bool,
    pub sigils: Sigils,
    pub usage: Usage,
}

impl ProviderResponse {
    pub fn success(&self) -> bool {
        self.exit_code == 0 && !self.timed_out && !self.needs_human
    }
}

pub trait Provider: Send + Sync {
    fn name(&self) -> &str;
    fn available(&self) -> bool;
    fn run(&self, request: &ProviderRequest) -> Result<ProviderResponse>;
}

pub struct CliProvider {
    name: String,
    spec: AgentSpec,
}

impl CliProvider {
    pub fn new(name: impl Into<String>, spec: AgentSpec) -> Self {
        CliProvider {
            name: name.into(),
            spec,
        }
    }

    fn build_args(&self, prompt: &str, model: Option<&str>) -> Vec<String> {
        let mut args: Vec<String> = self.spec.cmd.iter().skip(1).cloned().collect();
        let prompt_slot = args.iter().position(|arg| arg.contains("{prompt}"));
        if let Some(model) = model {
            if !self.spec.model_args.is_empty() {
                let model_args: Vec<String> = self
                    .spec
                    .model_args
                    .iter()
                    .map(|arg| arg.replace("{model}", model))
                    .collect();
                let index = prompt_slot.unwrap_or(args.len());
                for (offset, value) in model_args.into_iter().enumerate() {
                    args.insert(index + offset, value);
                }
            }
        }
        args.iter()
            .map(|arg| arg.replace("{prompt}", prompt))
            .collect()
    }

    fn program(&self) -> Result<String> {
        self.spec
            .cmd
            .first()
            .cloned()
            .ok_or_else(|| MajstackError::Provider(format!("agent '{}' has no cmd", self.name)))
    }
}

impl Provider for CliProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn available(&self) -> bool {
        self.program()
            .map(|program| majstack_execution::command_exists(&program))
            .unwrap_or(false)
    }

    fn run(&self, request: &ProviderRequest) -> Result<ProviderResponse> {
        if self.spec.mode == "handoff" {
            return Err(MajstackError::Provider(format!(
                "agent '{}' is a handoff provider and cannot run headless",
                self.name
            )));
        }
        let program = self.program()?;
        let has_prompt_slot = self.spec.cmd.iter().any(|arg| arg.contains("{prompt}"));
        let args = self.build_args(&request.prompt, request.model.as_deref());
        let mut spec =
            ProcessSpec::new(program, args, &request.cwd).timeout(request.timeout_seconds);
        for (key, value) in &self.spec.env {
            spec = spec.env(key, value);
        }
        for (key, value) in &request.env {
            spec = spec.env(key, value);
        }
        if !has_prompt_slot {
            spec = spec.stdin(request.prompt.clone());
        }
        let output = run_process(&spec)?;
        let combined = output.combined();
        let output_tokens = majstack_telemetry::estimate_tokens(&combined) as u64;
        Ok(ProviderResponse {
            provider: self.name.clone(),
            model: request.model.clone(),
            sigils: majstack_core::sigils::parse(&combined),
            output: combined,
            exit_code: output.code,
            duration_ms: output.duration_ms,
            timed_out: output.timed_out,
            needs_human: false,
            usage: Usage {
                input_tokens: majstack_telemetry::estimate_tokens(&request.prompt) as u64,
                output_tokens,
                cost_usd: 0.0,
            },
        })
    }
}

pub struct HandoffProvider {
    name: String,
    spec: AgentSpec,
    prompt_file: PathBuf,
}

impl HandoffProvider {
    pub fn new(name: impl Into<String>, spec: AgentSpec, prompt_file: PathBuf) -> Self {
        HandoffProvider {
            name: name.into(),
            spec,
            prompt_file,
        }
    }
}

impl Provider for HandoffProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn available(&self) -> bool {
        true
    }

    fn run(&self, request: &ProviderRequest) -> Result<ProviderResponse> {
        if let Some(parent) = self.prompt_file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.prompt_file, &request.prompt)?;
        Ok(ProviderResponse {
            provider: self.name.clone(),
            model: request.model.clone(),
            output: format!(
                "handoff prompt written to {} (host reads {})",
                self.prompt_file.display(),
                self.spec.instructions_file
            ),
            exit_code: 0,
            duration_ms: 0,
            timed_out: false,
            needs_human: true,
            sigils: Sigils::default(),
            usage: Usage::default(),
        })
    }
}

pub struct MockProvider {
    name: String,
    responder: Arc<dyn Fn(&ProviderRequest) -> String + Send + Sync>,
}

impl MockProvider {
    pub fn new(
        name: impl Into<String>,
        responder: Arc<dyn Fn(&ProviderRequest) -> String + Send + Sync>,
    ) -> Self {
        MockProvider {
            name: name.into(),
            responder,
        }
    }
}

impl Provider for MockProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn available(&self) -> bool {
        true
    }

    fn run(&self, request: &ProviderRequest) -> Result<ProviderResponse> {
        let output = (self.responder)(request);
        let output_tokens = majstack_telemetry::estimate_tokens(&output) as u64;
        let input_tokens = majstack_telemetry::estimate_tokens(&request.prompt) as u64;
        Ok(ProviderResponse {
            provider: self.name.clone(),
            model: request.model.clone(),
            sigils: majstack_core::sigils::parse(&output),
            output,
            exit_code: 0,
            duration_ms: 0,
            timed_out: false,
            needs_human: false,
            usage: Usage {
                input_tokens,
                output_tokens,
                cost_usd: 0.0,
            },
        })
    }
}

pub struct ProviderRegistry {
    providers: BTreeMap<String, Arc<dyn Provider>>,
    default: String,
}

impl ProviderRegistry {
    pub fn from_config(config: &MajstackConfig, prompt_file: PathBuf) -> Result<Self> {
        let mut providers: BTreeMap<String, Arc<dyn Provider>> = BTreeMap::new();
        for (name, spec) in &config.agents {
            let provider: Arc<dyn Provider> = if spec.mode == "handoff" {
                Arc::new(HandoffProvider::new(
                    name.clone(),
                    spec.clone(),
                    prompt_file.clone(),
                ))
            } else {
                Arc::new(CliProvider::new(name.clone(), spec.clone()))
            };
            providers.insert(name.clone(), provider);
        }
        if providers.is_empty() {
            return Err(MajstackError::Config("no providers configured".into()));
        }
        Ok(ProviderRegistry {
            providers,
            default: config.run.agent.clone(),
        })
    }

    pub fn get(&self, name: &str) -> Result<Arc<dyn Provider>> {
        self.providers
            .get(name)
            .cloned()
            .ok_or_else(|| MajstackError::Provider(format!("unknown provider '{name}'")))
    }

    pub fn insert(&mut self, name: impl Into<String>, provider: Arc<dyn Provider>) {
        self.providers.insert(name.into(), provider);
    }

    pub fn set_default(&mut self, name: impl Into<String>) {
        self.default = name.into();
    }

    pub fn default(&self) -> Result<Arc<dyn Provider>> {
        self.get(&self.default)
    }

    pub fn names(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }

    pub fn availability(&self) -> Vec<(String, bool)> {
        self.providers
            .iter()
            .map(|(name, provider)| (name.clone(), provider.available()))
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelStrategy {
    Fixed,
    CostOptimized,
    Escalate,
    PlanThenExecute,
}

impl ModelStrategy {
    pub fn parse(value: &str) -> Self {
        match value {
            "fixed" => ModelStrategy::Fixed,
            "escalate" => ModelStrategy::Escalate,
            "plan-then-execute" => ModelStrategy::PlanThenExecute,
            _ => ModelStrategy::CostOptimized,
        }
    }
}

pub fn choose_model(
    strategy: ModelStrategy,
    explicit: Option<&str>,
    iteration: u64,
    failures: u64,
    hint: Option<&str>,
) -> Option<String> {
    if let Some(hint) = hint {
        return Some(hint.to_string());
    }
    match strategy {
        ModelStrategy::Fixed => explicit.map(str::to_string),
        ModelStrategy::CostOptimized => {
            if failures > 0 {
                Some("opus".into())
            } else if iteration > 2 {
                Some("haiku".into())
            } else {
                explicit
                    .map(str::to_string)
                    .or_else(|| Some("sonnet".into()))
            }
        }
        ModelStrategy::Escalate => {
            if failures >= 2 {
                Some("opus".into())
            } else if failures == 1 {
                Some("sonnet".into())
            } else {
                Some("haiku".into())
            }
        }
        ModelStrategy::PlanThenExecute => {
            if iteration <= 1 {
                Some("opus".into())
            } else {
                Some("sonnet".into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_args_without_model() {
        let spec = AgentSpec {
            cmd: vec!["claude".into(), "-p".into(), "{prompt}".into()],
            ..AgentSpec::default()
        };
        let provider = CliProvider::new("claude", spec);
        assert_eq!(provider.build_args("hi", None), vec!["-p", "hi"]);
    }

    #[test]
    fn builds_args_with_model() {
        let spec = AgentSpec {
            cmd: vec!["claude".into(), "-p".into(), "{prompt}".into()],
            model_args: vec!["--model".into(), "{model}".into()],
            ..AgentSpec::default()
        };
        let provider = CliProvider::new("claude", spec);
        assert_eq!(
            provider.build_args("hi", Some("opus")),
            vec!["-p", "--model", "opus", "hi"]
        );
    }

    #[test]
    fn strategy_escalates() {
        assert_eq!(
            choose_model(ModelStrategy::Escalate, None, 1, 0, None).as_deref(),
            Some("haiku")
        );
        assert_eq!(
            choose_model(ModelStrategy::Escalate, None, 1, 1, None).as_deref(),
            Some("sonnet")
        );
        assert_eq!(
            choose_model(ModelStrategy::Escalate, None, 1, 2, None).as_deref(),
            Some("opus")
        );
        assert_eq!(
            choose_model(ModelStrategy::Escalate, None, 1, 2, Some("sonnet")).as_deref(),
            Some("sonnet")
        );
    }

    #[test]
    fn plan_then_execute() {
        assert_eq!(
            choose_model(ModelStrategy::PlanThenExecute, None, 1, 0, None).as_deref(),
            Some("opus")
        );
        assert_eq!(
            choose_model(ModelStrategy::PlanThenExecute, None, 2, 0, None).as_deref(),
            Some("sonnet")
        );
    }

    #[test]
    fn registry_from_config() {
        let config = MajstackConfig::with_builtin_agents();
        let registry = ProviderRegistry::from_config(&config, PathBuf::from("prompt.md")).unwrap();
        assert!(registry.names().contains(&"claude".to_string()));
        assert!(registry.get("nope").is_err());
    }
}
