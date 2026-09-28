use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub fn estimate_tokens(text: &str) -> usize {
    let chars = text.chars().count();
    (chars / 4).max(if chars == 0 { 0 } else { 1 })
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: f64,
}

impl Usage {
    pub fn merge(&mut self, other: &Usage) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.cost_usd += other.cost_usd;
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextSection {
    pub name: String,
    pub tokens: usize,
    pub included: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ContextBill {
    pub budget: usize,
    pub used: usize,
    pub sections: Vec<ContextSection>,
}

impl ContextBill {
    pub fn new(budget: usize) -> Self {
        ContextBill {
            budget,
            used: 0,
            sections: Vec::new(),
        }
    }

    pub fn remaining(&self) -> usize {
        self.budget.saturating_sub(self.used)
    }

    pub fn offer(
        &mut self,
        name: impl Into<String>,
        text: &str,
        mandatory: bool,
    ) -> Option<String> {
        let tokens = estimate_tokens(text);
        if mandatory || tokens <= self.remaining() {
            self.used += tokens;
            self.sections.push(ContextSection {
                name: name.into(),
                tokens,
                included: true,
            });
            Some(text.to_string())
        } else {
            self.sections.push(ContextSection {
                name: name.into(),
                tokens,
                included: false,
            });
            None
        }
    }

    pub fn report(&self) -> BTreeMap<String, usize> {
        self.sections
            .iter()
            .filter(|section| section.included)
            .map(|section| (section.name.clone(), section.tokens))
            .collect()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Metrics {
    pub counters: BTreeMap<String, u64>,
    pub timings_ms: BTreeMap<String, u64>,
}

impl Metrics {
    pub fn incr(&mut self, name: &str) {
        *self.counters.entry(name.to_string()).or_insert(0) += 1;
    }

    pub fn add(&mut self, name: &str, value: u64) {
        *self.counters.entry(name.to_string()).or_insert(0) += value;
    }

    pub fn time(&mut self, name: &str, elapsed_ms: u64) {
        let entry = self.timings_ms.entry(name.to_string()).or_insert(0);
        *entry += elapsed_ms;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_drops_optional_sections() {
        let mut bill = ContextBill::new(4);
        assert!(bill
            .offer("principles", "12345678901234567890", true)
            .is_some());
        assert!(bill
            .offer("history", "12345678901234567890", false)
            .is_none());
        assert_eq!(bill.used, estimate_tokens("12345678901234567890"));
        assert!(!bill.report().contains_key("history"));
    }
}
