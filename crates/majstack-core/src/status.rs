use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunState {
    Requested,
    Understanding,
    Investigating,
    Specifying,
    ReviewingSpec,
    Planning,
    ReviewingPlan,
    Decomposing,
    Ready,
    Executing,
    Verifying,
    Reviewing,
    Repairing,
    Reverifying,
    Completed,
    Paused,
    Blocked,
    Failed,
    Cancelled,
    NeedsHumanInput,
}

str_enum!(RunState {
    Requested => "requested",
    Understanding => "understanding",
    Investigating => "investigating",
    Specifying => "specifying",
    ReviewingSpec => "reviewing_spec",
    Planning => "planning",
    ReviewingPlan => "reviewing_plan",
    Decomposing => "decomposing",
    Ready => "ready",
    Executing => "executing",
    Verifying => "verifying",
    Reviewing => "reviewing",
    Repairing => "repairing",
    Reverifying => "reverifying",
    Completed => "completed",
    Paused => "paused",
    Blocked => "blocked",
    Failed => "failed",
    Cancelled => "cancelled",
    NeedsHumanInput => "needs_human_input",
});

impl RunState {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            RunState::Completed | RunState::Failed | RunState::Cancelled
        )
    }

    pub fn is_suspended(&self) -> bool {
        matches!(
            self,
            RunState::Paused | RunState::Blocked | RunState::NeedsHumanInput
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Blocked,
    Ready,
    Claimed,
    Running,
    Verifying,
    Reviewing,
    Repairing,
    Completed,
    Failed,
    Cancelled,
}

str_enum!(TaskStatus {
    Pending => "pending",
    Blocked => "blocked",
    Ready => "ready",
    Claimed => "claimed",
    Running => "running",
    Verifying => "verifying",
    Reviewing => "reviewing",
    Repairing => "repairing",
    Completed => "completed",
    Failed => "failed",
    Cancelled => "cancelled",
});

impl TaskStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled
        )
    }

    pub fn is_active(&self) -> bool {
        matches!(
            self,
            TaskStatus::Claimed
                | TaskStatus::Running
                | TaskStatus::Verifying
                | TaskStatus::Reviewing
                | TaskStatus::Repairing
        )
    }

    pub fn is_success(&self) -> bool {
        matches!(self, TaskStatus::Completed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkType {
    Feature,
    BugFix,
    Investigation,
    Refactoring,
    Performance,
    Prototype,
    Architecture,
    Design,
    VisualParity,
    RuntimeForensics,
    TraceForensics,
    Security,
    Documentation,
    Release,
    Qa,
    Migration,
    DependencyUpgrade,
    IncidentRepair,
    RepositoryUnderstanding,
    TechnicalWriting,
    SkillAuthoring,
    Evaluation,
    Cleanup,
    Unknown,
}

str_enum!(WorkType {
    Feature => "feature",
    BugFix => "bug_fix",
    Investigation => "investigation",
    Refactoring => "refactoring",
    Performance => "performance",
    Prototype => "prototype",
    Architecture => "architecture",
    Design => "design",
    VisualParity => "visual_parity",
    RuntimeForensics => "runtime_forensics",
    TraceForensics => "trace_forensics",
    Security => "security",
    Documentation => "documentation",
    Release => "release",
    Qa => "qa",
    Migration => "migration",
    DependencyUpgrade => "dependency_upgrade",
    IncidentRepair => "incident_repair",
    RepositoryUnderstanding => "repository_understanding",
    TechnicalWriting => "technical_writing",
    SkillAuthoring => "skill_authoring",
    Evaluation => "evaluation",
    Cleanup => "cleanup",
    Unknown => "unknown",
});

impl WorkType {
    pub fn is_investigation_first(&self) -> bool {
        matches!(
            self,
            WorkType::BugFix
                | WorkType::Investigation
                | WorkType::Performance
                | WorkType::RuntimeForensics
                | WorkType::TraceForensics
                | WorkType::IncidentRepair
                | WorkType::Unknown
                | WorkType::RepositoryUnderstanding
        )
    }

    pub fn requires_browser(&self) -> bool {
        matches!(
            self,
            WorkType::VisualParity | WorkType::Qa | WorkType::Design
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureClass {
    Tool,
    Environment,
    Model,
    Implementation,
    Test,
    Design,
    Architecture,
    Dependency,
    Permission,
    Timeout,
    ResourceExhaustion,
    Unknown,
}

str_enum!(FailureClass {
    Tool => "tool",
    Environment => "environment",
    Model => "model",
    Implementation => "implementation",
    Test => "test",
    Design => "design",
    Architecture => "architecture",
    Dependency => "dependency",
    Permission => "permission",
    Timeout => "timeout",
    ResourceExhaustion => "resource_exhaustion",
    Unknown => "unknown",
});

impl FailureClass {
    pub fn retryable_without_change(&self) -> bool {
        matches!(
            self,
            FailureClass::Tool | FailureClass::Environment | FailureClass::Timeout
        )
    }

    pub fn requires_replan(&self) -> bool {
        matches!(self, FailureClass::Architecture | FailureClass::Design)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionLevel {
    ReadOnly,
    Safe,
    Standard,
    Autonomous,
    Dangerous,
}

str_enum!(PermissionLevel {
    ReadOnly => "read_only",
    Safe => "safe",
    Standard => "standard",
    Autonomous => "autonomous",
    Dangerous => "dangerous",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Minor,
    Major,
    Critical,
    Blocker,
}

str_enum!(Severity {
    Info => "info",
    Minor => "minor",
    Major => "major",
    Critical => "critical",
    Blocker => "blocker",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    Skill,
    Workflow,
    Playbook,
    Principle,
    Agent,
    Subagent,
    Script,
    Hook,
    Command,
    BrowserCommand,
    Automation,
    Provider,
    Host,
    Template,
    Configuration,
    Test,
    Utility,
    Capability,
}

str_enum!(ArtifactType {
    Skill => "skill",
    Workflow => "workflow",
    Playbook => "playbook",
    Principle => "principle",
    Agent => "agent",
    Subagent => "subagent",
    Script => "script",
    Hook => "hook",
    Command => "command",
    BrowserCommand => "browser_command",
    Automation => "automation",
    Provider => "provider",
    Host => "host",
    Template => "template",
    Configuration => "configuration",
    Test => "test",
    Utility => "utility",
    Capability => "capability",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    Passed,
    Failed,
    Error,
    Skipped,
}

str_enum!(VerificationStatus {
    Passed => "passed",
    Failed => "failed",
    Error => "error",
    Skipped => "skipped",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewVerdict {
    Approve,
    Reject,
    NeedsChanges,
}

str_enum!(ReviewVerdict {
    Approve => "approve",
    Reject => "reject",
    NeedsChanges => "needs_changes",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Cli,
    Api,
    Handoff,
    Mock,
}

str_enum!(ProviderKind {
    Cli => "cli",
    Api => "api",
    Handoff => "handoff",
    Mock => "mock",
});
