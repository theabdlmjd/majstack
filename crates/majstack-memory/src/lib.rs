use majstack_core::Result;
use majstack_state::Store;
use majstack_telemetry::{estimate_tokens, ContextBill};
use std::path::Path;

pub fn remember(
    store: &Store,
    run_id: Option<&str>,
    text: &str,
    tags: &[String],
) -> Result<String> {
    store.insert_discovery(run_id, "knowledge", text, tags)
}

pub fn remember_pattern(store: &Store, run_id: Option<&str>, text: &str) -> Result<String> {
    remember(store, run_id, text, &["pattern".to_string()])
}

pub fn remember_lesson(store: &Store, run_id: Option<&str>, text: &str) -> Result<String> {
    remember(store, run_id, text, &["lesson".to_string()])
}

fn sanitize_query(query: &str) -> String {
    let words: Vec<&str> = query
        .split(|ch: char| !ch.is_alphanumeric() && ch != '_')
        .filter(|word| word.len() >= 2)
        .take(12)
        .collect();
    words.join(" OR ")
}

pub fn search(store: &Store, query: &str, limit: i64) -> Vec<String> {
    let sanitized = sanitize_query(query);
    if sanitized.is_empty() {
        return Vec::new();
    }
    store
        .search_knowledge(&sanitized, limit)
        .unwrap_or_default()
}

pub fn append_line(path: &Path, text: &str) -> Result<()> {
    use std::io::Write;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(text.trim_end().as_bytes())?;
    file.write_all(b"\n")?;
    Ok(())
}

pub fn tail(path: &Path, lines: usize) -> String {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let collected: Vec<&str> = text.lines().collect();
    let start = collected.len().saturating_sub(lines);
    collected[start..].join("\n")
}

pub struct ContextBuilder {
    bill: ContextBill,
    sections: Vec<(String, String)>,
}

impl ContextBuilder {
    pub fn new(budget: usize) -> Self {
        ContextBuilder {
            bill: ContextBill::new(budget),
            sections: Vec::new(),
        }
    }

    pub fn add(&mut self, name: &str, text: &str, mandatory: bool) {
        if text.trim().is_empty() {
            return;
        }
        if let Some(included) = self.bill.offer(name, text, mandatory) {
            self.sections.push((name.to_string(), included));
        }
    }

    pub fn render(&self) -> String {
        self.sections
            .iter()
            .map(|(name, text)| format!("# {}\n{}", name.to_uppercase().replace(' ', "_"), text))
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    pub fn bill(&self) -> &ContextBill {
        &self.bill
    }

    pub fn tokens(&self) -> usize {
        self.bill.used
    }
}

pub fn estimate(text: &str) -> usize {
    estimate_tokens(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn searches_knowledge() {
        let store = Store::memory().unwrap();
        remember_pattern(&store, None, "the build uses vite").unwrap();
        assert_eq!(search(&store, "vite build", 5).len(), 1);
    }

    #[test]
    fn context_builder_respects_budget() {
        let mut builder = ContextBuilder::new(5);
        builder.add("principles", "12345678901234567890", true);
        builder.add("history", "12345678901234567890", false);
        assert!(builder.render().contains("PRINCIPLES"));
        assert!(!builder.render().contains("HISTORY"));
    }
}
