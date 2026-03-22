//! Historical context window — session-level history of comprehended inputs.

use crate::constants::HISTORICAL_CONTEXT_DEPTH;
use crate::window::{ContextItem, ContextWindow};
use hydra_comprehension::ComprehendedInput;

/// Stores a rolling window of comprehended inputs for historical context.
#[derive(Debug, Clone)]
pub struct SessionHistory {
    /// The stored inputs, most recent last.
    entries: Vec<ComprehendedInput>,
}

impl SessionHistory {
    /// Create a new empty session history.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Add a comprehended input to the history.
    ///
    /// If the history exceeds the depth limit, the oldest entry is removed.
    pub fn add(&mut self, input: ComprehendedInput) {
        self.entries.push(input);
        if self.entries.len() > HISTORICAL_CONTEXT_DEPTH {
            let drain_count = self.entries.len() - HISTORICAL_CONTEXT_DEPTH;
            self.entries.drain(..drain_count);
        }
    }

    /// Return the number of entries in the history.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check whether the history is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Return all entries.
    pub fn entries(&self) -> &[ComprehendedInput] {
        &self.entries
    }

    /// Filter entries by domain label.
    pub fn by_domain(&self, domain_label: &str) -> Vec<&ComprehendedInput> {
        self.entries
            .iter()
            .filter(|e| e.primary_domain.label() == domain_label)
            .collect()
    }
}

impl Default for SessionHistory {
    fn default() -> Self {
        Self::new()
    }
}

/// Build a historical context window from the session history.
///
/// Each historical entry becomes a context item with its confidence as significance.
pub fn build_historical(history: &SessionHistory) -> ContextWindow {
    let mut window = ContextWindow::new("historical");
    for entry in &history.entries {
        window.add(ContextItem::with_domain(
            entry.summary(),
            entry.confidence,
            entry.primary_domain.label(),
        ));
    }
    window
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydra_comprehension::{
        ConstraintStatus, Domain, Horizon, InputSource, ResonanceResult, TemporalContext,
    };

    fn make_input(raw: &str, domain: Domain) -> ComprehendedInput {
        ComprehendedInput {
            raw: raw.to_string(),
            primary_domain: domain.clone(),
            all_domains: vec![(domain, 0.6)],
            primitives: vec![],
            temporal: TemporalContext {
                urgency: 0.5,
                horizon: Horizon::ShortTerm,
                constraint_status: ConstraintStatus::None,
            },
            resonance: ResonanceResult::empty(),
            source: InputSource::PrincipalText,
            confidence: 0.7,
            used_llm: false,
        }
    }

    #[test]
    fn history_depth_capped() {
        let mut h = SessionHistory::new();
        for i in 0..30 {
            h.add(make_input(&format!("input-{i}"), Domain::Engineering));
        }
        assert_eq!(h.len(), HISTORICAL_CONTEXT_DEPTH);
    }

    #[test]
    fn by_domain_filters_correctly() {
        let mut h = SessionHistory::new();
        h.add(make_input("deploy api", Domain::Engineering));
        h.add(make_input("check budget", Domain::Finance));
        h.add(make_input("fix code", Domain::Engineering));
        assert_eq!(h.by_domain("engineering").len(), 2);
        assert_eq!(h.by_domain("finance").len(), 1);
    }

    #[test]
    fn build_historical_creates_window() {
        let mut h = SessionHistory::new();
        h.add(make_input("deploy api", Domain::Engineering));
        let window = build_historical(&h);
        assert_eq!(window.len(), 1);
        assert_eq!(window.label, "historical");
    }
}
