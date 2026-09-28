use include_dir::{include_dir, Dir};
use majstack_core::{MajstackError, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

pub static EMBEDDED: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/../../skills");

pub fn install_assets(target: &Path) -> Result<usize> {
    let mut count = 0;
    install_dir(&EMBEDDED, target, &mut count)?;
    Ok(count)
}

#[derive(Debug, Clone, Copy)]
pub struct HostLayout {
    pub project_skills: &'static str,
    pub global_skills: &'static str,
    pub instructions: &'static str,
}

pub fn host_layout(host: &str) -> Option<HostLayout> {
    let layout = match host {
        "claude" => HostLayout {
            project_skills: ".claude/skills",
            global_skills: ".claude/skills",
            instructions: "CLAUDE.md",
        },
        "codex" => HostLayout {
            project_skills: ".codex/skills",
            global_skills: ".codex/skills",
            instructions: "AGENTS.md",
        },
        "opencode" => HostLayout {
            project_skills: ".agents/skills",
            global_skills: ".config/opencode/skills",
            instructions: "AGENTS.md",
        },
        "cursor" => HostLayout {
            project_skills: ".cursor/skills",
            global_skills: ".cursor/skills",
            instructions: "AGENTS.md",
        },
        "factory" => HostLayout {
            project_skills: ".factory/skills",
            global_skills: ".factory/skills",
            instructions: "AGENTS.md",
        },
        "kiro" => HostLayout {
            project_skills: ".kiro/skills",
            global_skills: ".kiro/skills",
            instructions: "AGENTS.md",
        },
        _ => return None,
    };
    Some(layout)
}

pub const HOSTS: &[&str] = &["claude", "codex", "opencode", "cursor", "factory", "kiro"];

pub fn install_skills(base: &Path, prefix: &str) -> Result<usize> {
    let skills_dir = EMBEDDED
        .get_dir("skills")
        .ok_or_else(|| MajstackError::State("embedded skills are missing".into()))?;
    let mut count = 0;
    for file in skills_dir.files() {
        let Some(stem) = file.path().file_stem().and_then(|name| name.to_str()) else {
            continue;
        };
        let text = String::from_utf8_lossy(file.contents()).to_string();
        let (meta, body) = parse_frontmatter(&text);
        let description = meta.get("summary").cloned().unwrap_or_default();
        let directory = base.join(format!("{prefix}{stem}"));
        std::fs::create_dir_all(&directory)?;
        let document =
            format!("---\nname: {prefix}{stem}\ndescription: {description}\n---\n\n{body}\n");
        std::fs::write(directory.join("SKILL.md"), document)?;
        count += 1;
    }
    Ok(count)
}

fn install_dir(dir: &Dir<'_>, target: &Path, count: &mut usize) -> Result<()> {
    for file in dir.files() {
        let relative = file.path();
        let is_root_config = relative
            .parent()
            .map(|parent| parent.as_os_str().is_empty())
            .unwrap_or(true)
            && relative.file_name().and_then(|name| name.to_str()) == Some("config.toml");
        if is_root_config {
            continue;
        }
        let destination = target.join(relative);
        if destination.exists() {
            continue;
        }
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&destination, file.contents())?;
        *count += 1;
    }
    for sub in dir.dirs() {
        install_dir(sub, target, count)?;
    }
    Ok(())
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkillMeta {
    pub name: String,
    pub group: String,
    pub summary: String,
    pub mode: String,
    pub requires: Vec<String>,
    pub tools: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Skill {
    pub meta: SkillMeta,
    pub body: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Playbook {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub steps: Vec<String>,
    #[serde(default)]
    pub gate: Vec<String>,
}

pub fn parse_frontmatter(text: &str) -> (BTreeMap<String, String>, String) {
    let mut meta = BTreeMap::new();
    let trimmed = text.trim_start_matches('\u{feff}');
    if let Some(after) = trimmed.strip_prefix("---") {
        let after = after
            .strip_prefix("\r\n")
            .or_else(|| after.strip_prefix('\n'));
        if let Some(after) = after {
            if let Some(position) = after.find("\n---") {
                let block = &after[..position];
                for line in block.lines() {
                    if let Some((key, value)) = line.split_once(':') {
                        meta.insert(
                            key.trim().to_string(),
                            value.trim().trim_matches('"').to_string(),
                        );
                    }
                }
                let body = after[position + 4..]
                    .trim_start_matches(['\n', '\r'])
                    .to_string();
                return (meta, body);
            }
        }
    }
    (meta, text.to_string())
}

fn split_list(value: &str) -> Vec<String> {
    value
        .split([',', ' '])
        .map(|item| item.trim().trim_matches(['[', ']', '"', '\''].as_ref()))
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

pub fn load_skill(dir: &Path, name: &str) -> Option<Skill> {
    let path = dir.join(format!("{name}.md"));
    let text = std::fs::read_to_string(&path).ok()?;
    let (meta, body) = parse_frontmatter(&text);
    let skill_meta = SkillMeta {
        name: meta
            .get("name")
            .cloned()
            .unwrap_or_else(|| name.to_string()),
        group: meta
            .get("group")
            .cloned()
            .unwrap_or_else(|| "misc".to_string()),
        summary: meta.get("summary").cloned().unwrap_or_default(),
        mode: meta
            .get("mode")
            .cloned()
            .unwrap_or_else(|| "write".to_string()),
        requires: meta
            .get("requires")
            .map(|value| split_list(value))
            .unwrap_or_default(),
        tools: meta
            .get("tools")
            .map(|value| split_list(value))
            .unwrap_or_default(),
    };
    Some(Skill {
        meta: skill_meta,
        body,
        path,
    })
}

pub fn list_skills(dir: &Path) -> Vec<SkillMeta> {
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return out,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) {
            if let Some(skill) = load_skill(dir, stem) {
                out.push(skill.meta);
            }
        }
    }
    out.sort_by_key(|meta| (meta.group.clone(), meta.name.clone()));
    out
}

pub fn principles_text(dir: &Path) -> String {
    let mut parts = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return String::new(),
    };
    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("md"))
        .collect();
    paths.sort();
    for path in paths {
        if let Ok(text) = std::fs::read_to_string(&path) {
            parts.push(text.trim().to_string());
        }
    }
    parts.join("\n")
}

pub const PRINCIPLE_STAGES: &[&str] = &[
    "understand",
    "plan",
    "execute",
    "verify",
    "review",
    "release",
    "recover",
];

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PrincipleMeta {
    pub id: String,
    pub name: String,
    pub rule: String,
    pub stages: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Principle {
    pub meta: PrincipleMeta,
    pub body: String,
}

fn derive_name(id: &str) -> String {
    let trimmed = id.trim_start_matches(|ch: char| ch.is_ascii_digit() || ch == '-' || ch == '_');
    trimmed.replace(['-', '_'], " ")
}

pub fn load_principle(dir: &Path, file_name: &str) -> Option<Principle> {
    let path = dir.join(file_name);
    let text = std::fs::read_to_string(&path).ok()?;
    let id = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("principle")
        .to_string();
    let (meta, body) = parse_frontmatter(&text);
    let stages = meta
        .get("stages")
        .map(|value| split_list(value))
        .filter(|list| !list.is_empty())
        .unwrap_or_else(|| {
            PRINCIPLE_STAGES
                .iter()
                .map(|stage| stage.to_string())
                .collect()
        });
    let rule = meta
        .get("rule")
        .cloned()
        .or_else(|| {
            body.lines()
                .find(|line| !line.trim().is_empty())
                .map(|line| line.trim().to_string())
        })
        .unwrap_or_default();
    Some(Principle {
        meta: PrincipleMeta {
            name: meta
                .get("name")
                .cloned()
                .unwrap_or_else(|| derive_name(&id)),
            id,
            rule,
            stages,
        },
        body: body.trim().to_string(),
    })
}

pub fn list_principles(dir: &Path) -> Vec<PrincipleMeta> {
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return out,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        if let Some(file) = path.file_name().and_then(|name| name.to_str()) {
            if let Some(principle) = load_principle(dir, file) {
                out.push(principle.meta);
            }
        }
    }
    out.sort_by_key(|meta| meta.id.clone());
    out
}

pub fn principles_for_stage(dir: &Path, stage: &str) -> Vec<Principle> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        if let Some(file) = path.file_name().and_then(|name| name.to_str()) {
            if let Some(principle) = load_principle(dir, file) {
                if principle
                    .meta
                    .stages
                    .iter()
                    .any(|candidate| candidate == stage)
                {
                    out.push(principle);
                }
            }
        }
    }
    out.sort_by_key(|principle| principle.meta.id.clone());
    out
}

pub fn load_playbook(dir: &Path, name: &str) -> Option<Playbook> {
    let path = dir.join(format!("{name}.toml"));
    let text = std::fs::read_to_string(&path).ok()?;
    toml::from_str(&text).ok()
}

pub fn list_playbooks(dir: &Path) -> Vec<Playbook> {
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return out,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("toml") {
            continue;
        }
        if let Ok(text) = std::fs::read_to_string(&path) {
            if let Ok(playbook) = toml::from_str::<Playbook>(&text) {
                out.push(playbook);
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

pub fn validate_composition(metas: &[SkillMeta]) -> Result<()> {
    let known: HashSet<&str> = metas.iter().map(|meta| meta.name.as_str()).collect();
    let graph: HashMap<&str, &Vec<String>> = metas
        .iter()
        .map(|meta| (meta.name.as_str(), &meta.requires))
        .collect();
    for meta in metas {
        for dependency in &meta.requires {
            if !known.contains(dependency.as_str()) {
                return Err(MajstackError::Invalid(format!(
                    "skill '{}' requires unknown skill '{dependency}'",
                    meta.name
                )));
            }
        }
    }
    let mut visiting: HashSet<&str> = HashSet::new();
    let mut visited: HashSet<&str> = HashSet::new();
    for meta in metas {
        visit(meta.name.as_str(), &graph, &mut visiting, &mut visited)?;
    }
    Ok(())
}

fn visit<'a>(
    node: &'a str,
    graph: &HashMap<&'a str, &'a Vec<String>>,
    visiting: &mut HashSet<&'a str>,
    visited: &mut HashSet<&'a str>,
) -> Result<()> {
    if visited.contains(node) {
        return Ok(());
    }
    if !visiting.insert(node) {
        return Err(MajstackError::Invalid(format!(
            "circular skill requirement at '{node}'"
        )));
    }
    if let Some(dependencies) = graph.get(node) {
        for dependency in dependencies.iter() {
            if let Some(key) = graph
                .get_key_value(dependency.as_str())
                .map(|(key, _)| *key)
            {
                visit(key, graph, visiting, visited)?;
            }
        }
    }
    visiting.remove(node);
    visited.insert(node);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_frontmatter_and_body() {
        let text = "---\nname: build\ngroup: build\nsummary: Build it\n---\n\nBody here";
        let (meta, body) = parse_frontmatter(text);
        assert_eq!(meta.get("name").unwrap(), "build");
        assert_eq!(body, "Body here");
    }

    #[test]
    fn detects_circular_composition() {
        let metas = vec![
            SkillMeta {
                name: "a".into(),
                requires: vec!["b".into()],
                ..SkillMeta::default()
            },
            SkillMeta {
                name: "b".into(),
                requires: vec!["a".into()],
                ..SkillMeta::default()
            },
        ];
        assert!(validate_composition(&metas).is_err());
        let unknown = vec![SkillMeta {
            name: "a".into(),
            requires: vec!["ghost".into()],
            ..SkillMeta::default()
        }];
        assert!(validate_composition(&unknown).is_err());
    }
}
