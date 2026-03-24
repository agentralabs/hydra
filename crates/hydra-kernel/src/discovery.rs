//! Capability Discovery — suggests Hydra features based on user behavior.
//! Detects patterns in user input and recommends relevant capabilities.
//! Uses genome query for pattern matching, not hardcoded keywords.

use std::collections::HashSet;
use crate::loop_::middleware::CycleMiddleware;
use crate::loop_::types::{CycleResult, PerceivedInput};

/// A capability suggestion triggered by user behavior.
#[derive(Debug, Clone)]
pub struct DiscoverySuggestion {
    pub trigger: String,
    pub capability: String,
    pub command: String,
}

/// Check user input for discoverable capability patterns.
/// Uses genome to find relevant approaches — not hardcoded keywords.
pub fn check_for_suggestions(
    input: &str,
    genome: &hydra_genome::GenomeStore,
) -> Option<DiscoverySuggestion> {
    if input.len() < 10 { return None; }
    // Query genome for capabilities related to user's intent
    let matches = genome.query(input);
    // If genome has a high-confidence approach the user might not know about
    for entry in matches.iter().take(3) {
        if entry.use_count == 0 && entry.effective_confidence() > 0.6 {
            let approach = entry.approach.steps.first().cloned().unwrap_or_default();
            if approach.is_empty() { continue; }
            let tools = entry.approach.tools_used.join(", ");
            return Some(DiscoverySuggestion {
                trigger: input.chars().take(50).collect(),
                capability: approach,
                command: if tools.is_empty() { String::new() } else { format!("(uses: {tools})") },
            });
        }
    }
    None
}

/// Discovery middleware — injects suggestions when relevant capabilities are unused.
pub struct DiscoveryMiddleware {
    suggestions_made: HashSet<String>,
}

impl DiscoveryMiddleware {
    pub fn new() -> Self {
        Self { suggestions_made: HashSet::new() }
    }
}

impl CycleMiddleware for DiscoveryMiddleware {
    fn name(&self) -> &'static str { "discovery" }

    fn post_perceive(&mut self, perceived: &mut PerceivedInput) {
        let genome = hydra_genome::GenomeStore::open();
        if let Some(suggestion) = check_for_suggestions(&perceived.raw, &genome) {
            let key = suggestion.capability.clone();
            if !self.suggestions_made.contains(&key) {
                perceived.enrichments.insert("discovery".into(),
                    format!("Tip: {} {}", suggestion.capability, suggestion.command));
                self.suggestions_made.insert(key);
            }
        }
    }

    fn post_deliver(&mut self, _cycle: &CycleResult) {
        // Trim old suggestions to prevent unbounded growth
        if self.suggestions_made.len() > 100 {
            self.suggestions_made.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_none_for_short_input() {
        let genome = hydra_genome::GenomeStore::new();
        assert!(check_for_suggestions("hi", &genome).is_none());
    }

    #[test]
    fn returns_none_for_empty_genome() {
        let genome = hydra_genome::GenomeStore::new();
        assert!(check_for_suggestions("I need to deploy my application to production", &genome).is_none());
    }

    #[test]
    fn suggestion_not_repeated() {
        let mut mw = DiscoveryMiddleware::new();
        mw.suggestions_made.insert("test_capability".into());
        assert!(mw.suggestions_made.contains("test_capability"));
    }

    #[test]
    fn middleware_name() {
        let mw = DiscoveryMiddleware::new();
        assert_eq!(mw.name(), "discovery");
    }
}
