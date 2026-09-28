use crate::clock;
use crate::ids;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    RunCreated,
    RunStateChanged,
    RunCompleted,
    RunFailed,
    RunPaused,
    RunResumed,
    RunCancelled,
    WorkflowSelected,
    WorkflowStarted,
    WorkflowStepStarted,
    WorkflowStepCompleted,
    TaskCreated,
    TaskReady,
    TaskClaimed,
    TaskStarted,
    TaskCompleted,
    TaskFailed,
    TaskRetrying,
    TaskBlocked,
    TaskUnblocked,
    AgentStarted,
    AgentOutput,
    AgentCompleted,
    AgentFailed,
    ToolStarted,
    ToolCompleted,
    ToolFailed,
    VerificationStarted,
    VerificationCompleted,
    ReviewStarted,
    ReviewFinding,
    ReviewCompleted,
    RepairStarted,
    RepairCompleted,
    DecisionCreated,
    DiscoveryRecorded,
    EvidenceRecorded,
    CommitCreated,
    BranchCreated,
    WorktreeCreated,
    WorktreeRemoved,
    ProviderRunStarted,
    ProviderRunCompleted,
    ModelRunStarted,
    ModelRunCompleted,
    BrowserSessionStarted,
    BrowserArtifactCreated,
    HumanInterventionRequested,
    HumanInterventionRecorded,
    FailureClassified,
    RecoveryActionPlanned,
    RecoveryActionApplied,
    StrategyAdapted,
    SkillInvoked,
    PolicyDenied,
    Message,
}

str_enum!(EventKind {
    RunCreated => "run.created",
    RunStateChanged => "run.state_changed",
    RunCompleted => "run.completed",
    RunFailed => "run.failed",
    RunPaused => "run.paused",
    RunResumed => "run.resumed",
    RunCancelled => "run.cancelled",
    WorkflowSelected => "workflow.selected",
    WorkflowStarted => "workflow.started",
    WorkflowStepStarted => "workflow.step_started",
    WorkflowStepCompleted => "workflow.step_completed",
    TaskCreated => "task.created",
    TaskReady => "task.ready",
    TaskClaimed => "task.claimed",
    TaskStarted => "task.started",
    TaskCompleted => "task.completed",
    TaskFailed => "task.failed",
    TaskRetrying => "task.retrying",
    TaskBlocked => "task.blocked",
    TaskUnblocked => "task.unblocked",
    AgentStarted => "agent.started",
    AgentOutput => "agent.output",
    AgentCompleted => "agent.completed",
    AgentFailed => "agent.failed",
    ToolStarted => "tool.started",
    ToolCompleted => "tool.completed",
    ToolFailed => "tool.failed",
    VerificationStarted => "verification.started",
    VerificationCompleted => "verification.completed",
    ReviewStarted => "review.started",
    ReviewFinding => "review.finding",
    ReviewCompleted => "review.completed",
    RepairStarted => "repair.started",
    RepairCompleted => "repair.completed",
    DecisionCreated => "decision.created",
    DiscoveryRecorded => "discovery.recorded",
    EvidenceRecorded => "evidence.recorded",
    CommitCreated => "commit.created",
    BranchCreated => "branch.created",
    WorktreeCreated => "worktree.created",
    WorktreeRemoved => "worktree.removed",
    ProviderRunStarted => "provider.run_started",
    ProviderRunCompleted => "provider.run_completed",
    ModelRunStarted => "model.run_started",
    ModelRunCompleted => "model.run_completed",
    BrowserSessionStarted => "browser.session_started",
    BrowserArtifactCreated => "browser.artifact_created",
    HumanInterventionRequested => "human.intervention_requested",
    HumanInterventionRecorded => "human.intervention_recorded",
    FailureClassified => "failure.classified",
    RecoveryActionPlanned => "recovery.planned",
    RecoveryActionApplied => "recovery.applied",
    StrategyAdapted => "strategy.adapted",
    SkillInvoked => "skill.invoked",
    PolicyDenied => "policy.denied",
    Message => "message",
});

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub run_id: Option<String>,
    pub ts: i64,
    pub kind: EventKind,
    pub payload: serde_json::Value,
}

impl Event {
    pub fn new(kind: EventKind, payload: serde_json::Value) -> Self {
        Event {
            id: ids::event_id(),
            run_id: None,
            ts: clock::now_millis(),
            kind,
            payload,
        }
    }

    pub fn message(text: impl Into<String>) -> Self {
        Event::new(
            EventKind::Message,
            serde_json::json!({ "text": text.into() }),
        )
    }

    pub fn with_run(mut self, run_id: impl Into<String>) -> Self {
        self.run_id = Some(run_id.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_roundtrip() {
        let event = Event::message("hello").with_run("run-1");
        let encoded = serde_json::to_string(&event).unwrap();
        let decoded: Event = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.kind, EventKind::Message);
        assert_eq!(decoded.run_id.as_deref(), Some("run-1"));
    }

    #[test]
    fn event_kind_strings() {
        assert_eq!(EventKind::RunCompleted.as_str(), "run.completed");
        assert_eq!(
            EventKind::parse("task.claimed"),
            Some(EventKind::TaskClaimed)
        );
        assert_eq!(EventKind::parse("nope"), None);
    }
}
