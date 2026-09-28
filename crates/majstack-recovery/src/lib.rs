use majstack_core::FailureClass;
use serde::{Deserialize, Serialize};

pub fn classify(message: &str) -> FailureClass {
    let text = message.to_ascii_lowercase();
    let table: &[(FailureClass, &[&str])] = &[
        (
            FailureClass::Timeout,
            &["timed out", "timeout", "deadline exceeded"],
        ),
        (
            FailureClass::Permission,
            &[
                "permission",
                "denied",
                "forbidden",
                "blocked by careful",
                "policy",
            ],
        ),
        (
            FailureClass::Dependency,
            &[
                "cannot find module",
                "no such file",
                "not found",
                "missing dependency",
                "unresolved import",
                "version conflict",
            ],
        ),
        (
            FailureClass::Test,
            &["test failed", "assert", "expected", "panic", "failing test"],
        ),
        (
            FailureClass::Environment,
            &[
                "command not found",
                "not installed",
                "no such command",
                "is not recognized",
            ],
        ),
        (
            FailureClass::Tool,
            &["tool", "spawn", "exec", "failed to run"],
        ),
        (
            FailureClass::Architecture,
            &[
                "architecture",
                "design flaw",
                "wrong abstraction",
                "circular dependency",
            ],
        ),
        (
            FailureClass::Design,
            &["design", "visual", "layout", "contrast"],
        ),
        (
            FailureClass::Model,
            &["model", "provider", "rate limit", "context length"],
        ),
        (
            FailureClass::ResourceExhaustion,
            &["out of memory", "enospc", "resource", "too many open files"],
        ),
    ];
    for (class, keywords) in table {
        if keywords.iter().any(|keyword| text.contains(keyword)) {
            return *class;
        }
    }
    FailureClass::Implementation
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RecoveryStep {
    Retry,
    ChangeModel(String),
    ChangeProvider,
    ReduceScope,
    ReviseTask,
    Replan,
    EscalateHuman,
    Abort,
}

impl RecoveryStep {
    pub fn label(&self) -> String {
        match self {
            RecoveryStep::Retry => "retry".to_string(),
            RecoveryStep::ChangeModel(model) => format!("change_model:{model}"),
            RecoveryStep::ChangeProvider => "change_provider".to_string(),
            RecoveryStep::ReduceScope => "reduce_scope".to_string(),
            RecoveryStep::ReviseTask => "revise_task".to_string(),
            RecoveryStep::Replan => "replan".to_string(),
            RecoveryStep::EscalateHuman => "escalate_human".to_string(),
            RecoveryStep::Abort => "abort".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryPlan {
    pub class: FailureClass,
    pub steps: Vec<RecoveryStep>,
    pub blocking: bool,
}

impl RecoveryPlan {
    pub fn summary(&self) -> String {
        self.steps
            .iter()
            .map(RecoveryStep::label)
            .collect::<Vec<_>>()
            .join(",")
    }

    pub fn needs_replan(&self) -> bool {
        self.steps
            .iter()
            .any(|step| matches!(step, RecoveryStep::Replan))
    }

    pub fn wants_escalation(&self) -> bool {
        self.steps.iter().any(|step| {
            matches!(
                step,
                RecoveryStep::ChangeModel(_)
                    | RecoveryStep::ChangeProvider
                    | RecoveryStep::EscalateHuman
            )
        })
    }
}

pub fn plan(message: &str, attempts: u64, max_attempts: u64) -> RecoveryPlan {
    let class = classify(message);
    let exhausted = max_attempts > 0 && attempts >= max_attempts;
    if exhausted {
        let steps = match class {
            FailureClass::Architecture | FailureClass::Design => {
                vec![RecoveryStep::Replan, RecoveryStep::EscalateHuman]
            }
            FailureClass::Permission => vec![RecoveryStep::EscalateHuman],
            _ => vec![
                RecoveryStep::Replan,
                RecoveryStep::ChangeProvider,
                RecoveryStep::EscalateHuman,
            ],
        };
        return RecoveryPlan {
            class,
            steps,
            blocking: true,
        };
    }
    let steps = match class {
        FailureClass::Tool | FailureClass::Environment | FailureClass::Timeout => {
            vec![RecoveryStep::Retry]
        }
        FailureClass::Model => vec![RecoveryStep::ChangeProvider, RecoveryStep::Retry],
        FailureClass::Dependency => vec![RecoveryStep::ReviseTask, RecoveryStep::Retry],
        FailureClass::Architecture | FailureClass::Design => {
            vec![RecoveryStep::Replan, RecoveryStep::ReduceScope]
        }
        FailureClass::Permission => vec![RecoveryStep::EscalateHuman],
        FailureClass::ResourceExhaustion => vec![RecoveryStep::ReduceScope, RecoveryStep::Retry],
        FailureClass::Implementation | FailureClass::Test => {
            if attempts >= 2 {
                vec![
                    RecoveryStep::ChangeModel("opus".into()),
                    RecoveryStep::ReviseTask,
                    RecoveryStep::Retry,
                ]
            } else {
                vec![RecoveryStep::Retry]
            }
        }
        FailureClass::Unknown => vec![RecoveryStep::Retry],
    };
    let blocking = steps
        .iter()
        .any(|step| matches!(step, RecoveryStep::EscalateHuman));
    RecoveryPlan {
        class,
        steps,
        blocking,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_common_failures() {
        assert_eq!(classify("assertion failed"), FailureClass::Test);
        assert_eq!(classify("cannot find module 'x'"), FailureClass::Dependency);
        assert_eq!(
            classify("the architecture is wrong"),
            FailureClass::Architecture
        );
        assert_eq!(classify("something odd"), FailureClass::Implementation);
    }

    #[test]
    fn retries_tool_failures() {
        let plan = plan("command not found: foo", 1, 5);
        assert!(plan.steps.contains(&RecoveryStep::Retry));
        assert!(!plan.blocking);
    }

    #[test]
    fn escalates_model_after_repeated_implementation_failures() {
        let plan = plan("assert failed", 2, 5);
        assert!(plan.wants_escalation());
    }

    #[test]
    fn replans_when_exhausted() {
        let plan = plan("architecture is wrong", 3, 3);
        assert!(plan.blocking);
        assert!(plan.needs_replan());
    }

    #[test]
    fn permission_escalates_to_human() {
        let plan = plan("permission denied", 1, 5);
        assert!(plan.blocking);
        assert!(plan.steps.contains(&RecoveryStep::EscalateHuman));
    }
}
