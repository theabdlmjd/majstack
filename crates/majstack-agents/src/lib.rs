use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AgentRole {
    pub name: &'static str,
    pub purpose: &'static str,
    pub lens: &'static str,
    pub read_only: bool,
}

pub const ROLES: &[AgentRole] = &[
    AgentRole {
        name: "investigator",
        purpose: "Find the root cause before any change.",
        lens: "evidence and reproduction",
        read_only: true,
    },
    AgentRole {
        name: "architect",
        purpose: "Choose the structure and module boundaries.",
        lens: "boundaries, coupling, and data flow",
        read_only: true,
    },
    AgentRole {
        name: "planner",
        purpose: "Decompose the goal into small verifiable tasks.",
        lens: "task size and verifiability",
        read_only: true,
    },
    AgentRole {
        name: "implementer",
        purpose: "Make the smallest correct change.",
        lens: "simplicity and correctness",
        read_only: false,
    },
    AgentRole {
        name: "debugger",
        purpose: "Fix a reported failure from its evidence.",
        lens: "root cause over symptom",
        read_only: false,
    },
    AgentRole {
        name: "tester",
        purpose: "Prove behavior with tests.",
        lens: "behavior, not implementation",
        read_only: false,
    },
    AgentRole {
        name: "qa_engineer",
        purpose: "Verify the real artifact end to end.",
        lens: "acceptance criteria and edge cases",
        read_only: false,
    },
    AgentRole {
        name: "browser_tester",
        purpose: "Verify UI behavior in a real browser.",
        lens: "console errors, flows, and visuals",
        read_only: false,
    },
    AgentRole {
        name: "reviewer",
        purpose: "Independent code review.",
        lens: "correctness, tests, and maintainability",
        read_only: true,
    },
    AgentRole {
        name: "security_reviewer",
        purpose: "Attack surface and abuse cases.",
        lens: "trust boundaries and input handling",
        read_only: true,
    },
    AgentRole {
        name: "performance_investigator",
        purpose: "Find and fix measured slowness.",
        lens: "measure before and after",
        read_only: true,
    },
    AgentRole {
        name: "release_engineer",
        purpose: "Ship safely behind gates.",
        lens: "evidence, gates, and rollback",
        read_only: false,
    },
    AgentRole {
        name: "documentation_writer",
        purpose: "Document behavior and decisions.",
        lens: "reader load and accuracy",
        read_only: false,
    },
    AgentRole {
        name: "design_reviewer",
        purpose: "Review interaction and visual design.",
        lens: "hierarchy, consistency, and states",
        read_only: true,
    },
    AgentRole {
        name: "product_reviewer",
        purpose: "Challenge scope and premises.",
        lens: "outcome over output",
        read_only: true,
    },
    AgentRole {
        name: "visual_reviewer",
        purpose: "Check visual parity against a target.",
        lens: "pixel and layout fidelity",
        read_only: true,
    },
    AgentRole {
        name: "evidence_collector",
        purpose: "Gather proof for every claim.",
        lens: "reproducible evidence",
        read_only: true,
    },
    AgentRole {
        name: "recovery_agent",
        purpose: "Diagnose failure and change strategy.",
        lens: "classify, then adapt",
        read_only: true,
    },
    AgentRole {
        name: "comment_sicko",
        purpose: "Remove comments that restate the code.",
        lens: "delete redundant comments",
        read_only: true,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoleStage {
    Understand,
    Plan,
    Execute,
    Verify,
    Review,
    Release,
    Recover,
}

impl RoleStage {
    pub fn as_str(&self) -> &'static str {
        match self {
            RoleStage::Understand => "understand",
            RoleStage::Plan => "plan",
            RoleStage::Execute => "execute",
            RoleStage::Verify => "verify",
            RoleStage::Review => "review",
            RoleStage::Release => "release",
            RoleStage::Recover => "recover",
        }
    }
}

pub fn role(name: &str) -> Option<&'static AgentRole> {
    ROLES.iter().find(|role| role.name == name)
}

pub fn names() -> Vec<&'static str> {
    ROLES.iter().map(|role| role.name).collect()
}

pub fn roles_for_stage(stage: RoleStage) -> Vec<&'static AgentRole> {
    let wanted: &[&str] = match stage {
        RoleStage::Understand => &["investigator", "architect"],
        RoleStage::Plan => &["planner", "architect", "product_reviewer"],
        RoleStage::Execute => &["implementer", "debugger", "tester"],
        RoleStage::Verify => &[
            "qa_engineer",
            "tester",
            "browser_tester",
            "evidence_collector",
        ],
        RoleStage::Review => &[
            "reviewer",
            "security_reviewer",
            "design_reviewer",
            "visual_reviewer",
            "comment_sicko",
        ],
        RoleStage::Release => &["release_engineer", "documentation_writer"],
        RoleStage::Recover => &["recovery_agent", "performance_investigator"],
    };
    ROLES
        .iter()
        .filter(|role| wanted.contains(&role.name))
        .collect()
}

pub mod comment_sicko {
    use super::*;
    use std::path::Path;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CommentFinding {
        pub file: String,
        pub line: usize,
        pub text: String,
    }

    fn is_comment(line: &str) -> bool {
        let trimmed = line.trim_start();
        trimmed.starts_with("//")
            || trimmed.starts_with("/*")
            || trimmed.starts_with('*')
            || trimmed.starts_with("<!--")
            || trimmed.starts_with('#')
            || trimmed.starts_with("--")
    }

    pub fn audit_text(contents: &str) -> Vec<(usize, String)> {
        contents
            .lines()
            .enumerate()
            .filter(|(_, line)| is_comment(line))
            .map(|(index, line)| (index + 1, line.trim().to_string()))
            .collect()
    }

    pub fn scan(root: &Path, limit: usize) -> Vec<CommentFinding> {
        let mut findings = Vec::new();
        for entry in walkdir::WalkDir::new(root).into_iter().flatten() {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            let text_path = path.to_string_lossy();
            if text_path.contains(".git")
                || text_path.contains(".majstack")
                || text_path.contains("target")
                || text_path.contains("node_modules")
            {
                continue;
            }
            let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");
            if !matches!(
                extension,
                "rs" | "ts" | "js" | "tsx" | "jsx" | "py" | "go" | "java" | "c" | "cpp" | "h"
            ) {
                continue;
            }
            if let Ok(contents) = std::fs::read_to_string(path) {
                for (line, text) in audit_text(&contents) {
                    findings.push(CommentFinding {
                        file: path.to_string_lossy().to_string(),
                        line,
                        text,
                    });
                    if findings.len() >= limit {
                        return findings;
                    }
                }
            }
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_expected_roles() {
        assert!(role("comment_sicko").is_some());
        assert!(role("reviewer").is_some());
        assert!(role("nope").is_none());
        assert!(names().len() >= 18);
    }

    #[test]
    fn comment_sicko_flags_comments() {
        let findings = comment_sicko::audit_text("let x = 1;\n// restates the code\n/* block */");
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].0, 2);
    }
}
