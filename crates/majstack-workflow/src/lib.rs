use majstack_core::WorkType;
use majstack_skills::{list_playbooks, load_playbook, Playbook};
use std::path::{Path, PathBuf};

pub const WORK_TYPE_KEYWORDS: &[(WorkType, &[&str])] = &[
    (
        WorkType::BugFix,
        &[
            "bug",
            "crash",
            "broken",
            "regression",
            "fails",
            "error",
            "fix",
            "hotfix",
        ],
    ),
    (
        WorkType::Performance,
        &[
            "slow",
            "performance",
            "latency",
            "optimi",
            "benchmark",
            "profil",
            "throughput",
        ],
    ),
    (
        WorkType::Security,
        &[
            "security", "vulnerab", "owasp", "stride", "exploit", "cve", "audit",
        ],
    ),
    (
        WorkType::Investigation,
        &[
            "why",
            "investigate",
            "root cause",
            "diagnose",
            "figure out",
            "debug",
        ],
    ),
    (
        WorkType::RuntimeForensics,
        &["runtime", "forensic", "heap", "memory leak"],
    ),
    (
        WorkType::TraceForensics,
        &["trace", "span", "otel", "opentelemetry", "forensic"],
    ),
    (
        WorkType::Refactoring,
        &["refactor", "clean up", "simplify", "restructure"],
    ),
    (
        WorkType::Cleanup,
        &["unslop", "dead code", "tidy", "remove unused"],
    ),
    (
        WorkType::Prototype,
        &["prototype", "spike", "experiment", "throwaway"],
    ),
    (
        WorkType::Architecture,
        &[
            "architecture",
            "redesign",
            "system design",
            "module boundary",
        ],
    ),
    (
        WorkType::VisualParity,
        &[
            "visual parity",
            "pixel",
            "match the design",
            "pixel-perfect",
        ],
    ),
    (
        WorkType::Design,
        &["design", "ux", "ui", "wireframe", "figma"],
    ),
    (
        WorkType::Documentation,
        &["docs", "documentation", "readme", "changelog"],
    ),
    (
        WorkType::TechnicalWriting,
        &["write", "article", "guide", "explain in plain"],
    ),
    (
        WorkType::Release,
        &["ship", "release", "deploy", "land", "publish"],
    ),
    (
        WorkType::Qa,
        &["qa", "browser test", "end to end", "e2e", "smoke test"],
    ),
    (WorkType::Migration, &["migrate", "migration", "port to"]),
    (
        WorkType::DependencyUpgrade,
        &["upgrade dependency", "bump", "update package", "renovate"],
    ),
    (
        WorkType::IncidentRepair,
        &["incident", "outage", "sev1", "production down"],
    ),
    (
        WorkType::RepositoryUnderstanding,
        &[
            "understand the codebase",
            "explain the codebase",
            "onboard",
            "how does this work",
        ],
    ),
    (
        WorkType::SkillAuthoring,
        &["author a skill", "new skill", "create a skill"],
    ),
    (
        WorkType::Evaluation,
        &["eval", "evaluate", "benchmark the agent"],
    ),
];

pub const WORK_TYPE_WORKFLOW: &[(WorkType, &str)] = &[
    (WorkType::Feature, "feature"),
    (WorkType::BugFix, "bugfix"),
    (WorkType::Investigation, "investigation"),
    (WorkType::Refactoring, "refactor"),
    (WorkType::Performance, "perf"),
    (WorkType::Prototype, "prototype"),
    (WorkType::Architecture, "multi-phase-plan"),
    (WorkType::Design, "design"),
    (WorkType::VisualParity, "visual-parity"),
    (WorkType::RuntimeForensics, "runtime-forensics"),
    (WorkType::TraceForensics, "trace-forensics"),
    (WorkType::Security, "security"),
    (WorkType::Documentation, "docs"),
    (WorkType::Release, "release"),
    (WorkType::Qa, "qa"),
    (WorkType::Migration, "refactor"),
    (WorkType::DependencyUpgrade, "refactor"),
    (WorkType::IncidentRepair, "bugfix"),
    (WorkType::RepositoryUnderstanding, "investigation"),
    (WorkType::TechnicalWriting, "docs"),
    (WorkType::SkillAuthoring, "authoring-a-skill"),
    (WorkType::Evaluation, "eval"),
    (WorkType::Cleanup, "refactor"),
    (WorkType::Unknown, "feature"),
];

pub fn classify_work_type(request: &str) -> WorkType {
    let text = request.to_ascii_lowercase();
    let mut best = WorkType::Feature;
    let mut best_score = 0usize;
    for (work_type, keywords) in WORK_TYPE_KEYWORDS {
        let score: usize = keywords
            .iter()
            .filter(|keyword| text.contains(**keyword))
            .map(|keyword| if keyword.contains(' ') { 3 } else { 1 })
            .sum();
        if score > best_score {
            best = *work_type;
            best_score = score;
        }
    }
    let autonomous = [
        "go to bed",
        "keep going",
        "overnight",
        "unattended",
        "until it's done",
    ];
    if autonomous.iter().any(|phrase| text.contains(phrase)) && best_score <= 1 {
        best = WorkType::Unknown;
    }
    best
}

pub fn workflow_for(work_type: WorkType) -> &'static str {
    WORK_TYPE_WORKFLOW
        .iter()
        .find(|(candidate, _)| *candidate == work_type)
        .map(|(_, name)| *name)
        .unwrap_or("feature")
}

pub struct WorkflowRegistry {
    dir: PathBuf,
    playbooks: Vec<Playbook>,
}

impl WorkflowRegistry {
    pub fn load(dir: impl AsRef<Path>) -> Self {
        let dir = dir.as_ref().to_path_buf();
        let playbooks = list_playbooks(&dir);
        WorkflowRegistry { dir, playbooks }
    }

    pub fn names(&self) -> Vec<String> {
        self.playbooks
            .iter()
            .map(|playbook| playbook.name.clone())
            .collect()
    }

    pub fn get(&self, name: &str) -> Option<&Playbook> {
        self.playbooks.iter().find(|playbook| playbook.name == name)
    }

    pub fn reload(&mut self, name: &str) -> Option<&Playbook> {
        if let Some(playbook) = load_playbook(&self.dir, name) {
            self.playbooks.retain(|existing| existing.name != name);
            self.playbooks.push(playbook);
        }
        self.get(name)
    }

    pub fn route(&self, request: &str, explicit: Option<&str>) -> Option<&Playbook> {
        if let Some(name) = explicit {
            return self.get(name);
        }
        let text = request.to_ascii_lowercase();
        let mut best: Option<&Playbook> = None;
        let mut best_score = 0usize;
        for playbook in &self.playbooks {
            let score: usize = playbook
                .keywords
                .iter()
                .filter(|keyword| text.contains(&keyword.to_ascii_lowercase()))
                .map(|keyword| if keyword.contains(' ') { 3 } else { 1 })
                .sum();
            if score > best_score {
                best = Some(playbook);
                best_score = score;
            }
        }
        if best.is_some() {
            return best;
        }
        let work_type = classify_work_type(request);
        self.get(workflow_for(work_type))
    }

    pub fn by_work_type(&self, work_type: WorkType) -> Option<&Playbook> {
        self.get(workflow_for(work_type))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_requests() {
        assert_eq!(
            classify_work_type("this page crashes when empty"),
            WorkType::BugFix
        );
        assert_eq!(
            classify_work_type("why is the endpoint slow"),
            WorkType::Performance
        );
        assert_eq!(
            classify_work_type("run a security audit with owasp"),
            WorkType::Security
        );
        assert_eq!(classify_work_type("add csv export"), WorkType::Feature);
    }

    #[test]
    fn maps_work_type_to_workflow() {
        assert_eq!(workflow_for(WorkType::BugFix), "bugfix");
        assert_eq!(workflow_for(WorkType::Security), "security");
    }
}
