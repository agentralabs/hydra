//! Divergence detection — triggers belief revision on large divergence.

use crate::constants::DIVERGENCE_THRESHOLD;
use crate::errors::PredictionError;
use crate::shadow::{compute_divergence, ActualOutcome, OutcomeDivergence, ShadowOutcome};
use hydra_belief::{revise, Belief, BeliefStore};

/// Evaluates divergence and optionally triggers belief revision.
pub struct DivergenceDetector {
    threshold: f64,
}

impl DivergenceDetector {
    /// Create a new divergence detector with the default threshold.
    pub fn new() -> Self {
        Self {
            threshold: DIVERGENCE_THRESHOLD,
        }
    }

    /// Create a divergence detector with a custom threshold.
    pub fn with_threshold(threshold: f64) -> Self {
        Self { threshold }
    }

    /// Evaluate divergence between shadow and actual outcomes.
    ///
    /// If divergence exceeds the threshold, revises the relevant belief
    /// in the store and returns an error indicating excessive divergence.
    pub fn evaluate(
        &self,
        shadow: &ShadowOutcome,
        actual: &ActualOutcome,
        belief_store: &mut BeliefStore,
        prediction_topic: &str,
    ) -> Result<OutcomeDivergence, PredictionError> {
        let divergence = compute_divergence(shadow, actual);

        if divergence.score > self.threshold {
            // Create a corrective belief to revise the prediction model
            let corrective = Belief::world(
                format!(
                    "prediction for '{}' diverged by {:.2} — model needs update",
                    prediction_topic, divergence.score
                ),
                1.0 - divergence.score,
            );
            // Best effort: ignore revision errors here as the primary
            // concern is reporting the divergence
            let _result = revise(belief_store, corrective);

            return Err(PredictionError::DivergenceExceeded {
                divergence: divergence.score,
                threshold: self.threshold,
            });
        }

        Ok(divergence)
    }
}

impl Default for DivergenceDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_divergence_ok() {
        let detector = DivergenceDetector::new();
        let shadow = ShadowOutcome {
            description: "test".into(),
            confidence: 0.8,
            state_changes: vec![("a".into(), "1".into())],
        };
        let actual = ActualOutcome {
            description: "test".into(),
            state_changes: vec![("a".into(), "1".into())],
        };
        let mut store = BeliefStore::new();
        let result = detector.evaluate(&shadow, &actual, &mut store, "test");
        assert!(result.is_ok());
    }

    #[test]
    fn high_divergence_triggers_revision() {
        let detector = DivergenceDetector::new();
        let shadow = ShadowOutcome {
            description: "test".into(),
            confidence: 0.8,
            state_changes: vec![("a".into(), "1".into())],
        };
        let actual = ActualOutcome {
            description: "test".into(),
            state_changes: vec![("a".into(), "completely_different".into())],
        };
        let mut store = BeliefStore::new();
        let result = detector.evaluate(&shadow, &actual, &mut store, "deployment");
        assert!(result.is_err());
        // A corrective belief should have been added
        assert!(!store.is_empty());
    }
}
