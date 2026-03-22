//! Learning engine — the main entry point for reasoning weight evolution.

use crate::observation::{ObservationOutcome, ReasoningObservation};
use crate::record::LearningRecord;
use crate::tracker::ModeTracker;
use hydra_reasoning::ReasoningResult;

/// The learning engine. Observes reasoning outcomes and proposes weight adjustments.
///
/// This engine is an observer — it never modifies reasoning weights directly.
/// It produces `LearningRecord` proposals that must be approved externally.
#[derive(Debug)]
pub struct LearningEngine {
    /// The underlying mode tracker.
    tracker: ModeTracker,
    /// Total observations processed.
    observation_count: usize,
}

impl LearningEngine {
    /// Create a new learning engine.
    pub fn new() -> Self {
        Self {
            tracker: ModeTracker::new(),
            observation_count: 0,
        }
    }

    /// Observe a reasoning result and record it.
    ///
    /// Returns the observation that was recorded.
    pub fn observe(
        &mut self,
        result: &ReasoningResult,
        domain: &str,
        intent_type: &str,
    ) -> ReasoningObservation {
        self.observe_with_outcome(result, domain, intent_type, ObservationOutcome::Unknown)
    }

    /// Observe a reasoning result with a known outcome.
    ///
    /// Returns the observation that was recorded.
    pub fn observe_with_outcome(
        &mut self,
        result: &ReasoningResult,
        domain: &str,
        intent_type: &str,
        outcome: ObservationOutcome,
    ) -> ReasoningObservation {
        let observation = ReasoningObservation::from_result(result, domain, intent_type, outcome);
        self.tracker.record(&observation);
        self.observation_count += 1;
        observation
    }

    /// Check whether any weight adjustments should be proposed for a domain.
    ///
    /// Returns an empty vec if there are insufficient observations or no
    /// adjustments are warranted.
    pub fn check_adjustments(&self, domain: &str) -> Vec<LearningRecord> {
        self.tracker.check_adjustments(domain)
    }

    /// Return a TUI-friendly summary of the learning engine state.
    pub fn summary(&self) -> String {
        format!(
            "learning: observations={} domains={}",
            self.observation_count,
            self.tracker.domain_count(),
        )
    }

    /// Return the total number of observations processed.
    pub fn observation_count(&self) -> usize {
        self.observation_count
    }

    /// Return the number of tracked domains.
    pub fn domain_count(&self) -> usize {
        self.tracker.domain_count()
    }
}

impl Default for LearningEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydra_reasoning::conclusion::{ReasoningConclusion, ReasoningMode};
    use hydra_reasoning::ReasoningResult;

    fn make_result() -> ReasoningResult {
        let conclusion = ReasoningConclusion::new(
            ReasoningMode::Deductive,
            "test conclusion",
            0.8,
            vec![],
            false,
        );
        ReasoningResult {
            conclusions: vec![conclusion.clone()],
            synthesis_confidence: 0.8,
            used_llm: false,
            active_modes: 1,
            primary: Some(conclusion),
            mode_summary: vec![("deductive".to_string(), true)],
        }
    }

    #[test]
    fn observe_increments_count() {
        let mut engine = LearningEngine::new();
        let result = make_result();
        engine.observe(&result, "engineering", "action");
        assert_eq!(engine.observation_count(), 1);
    }

    #[test]
    fn summary_format() {
        let engine = LearningEngine::new();
        let s = engine.summary();
        assert!(s.contains("learning:"));
        assert!(s.contains("observations="));
        assert!(s.contains("domains="));
    }

    #[test]
    fn domain_count_tracks() {
        let mut engine = LearningEngine::new();
        let result = make_result();
        engine.observe(&result, "engineering", "action");
        engine.observe(&result, "finance", "query");
        assert_eq!(engine.domain_count(), 2);
    }
}
