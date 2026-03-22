//! Reasoning observation — records which modes contributed and their accuracy.

use chrono::{DateTime, Utc};
use hydra_reasoning::conclusion::ReasoningMode;
use hydra_reasoning::ReasoningResult;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The outcome of a reasoning observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObservationOutcome {
    /// The primary conclusion was correct.
    Correct,
    /// The primary conclusion was incorrect.
    Incorrect,
    /// Correctness could not be determined.
    Unknown,
}

/// A single observation of a reasoning cycle's performance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningObservation {
    /// Unique identifier for this observation.
    pub id: String,
    /// Which modes produced conclusions.
    pub contributing_modes: Vec<ReasoningMode>,
    /// The mode with the highest confidence conclusion.
    pub primary_mode: Option<ReasoningMode>,
    /// The domain in which reasoning occurred.
    pub domain: String,
    /// The intent type that triggered reasoning.
    pub intent_type: String,
    /// The outcome of this reasoning cycle.
    pub outcome: ObservationOutcome,
    /// When this observation was recorded.
    pub timestamp: DateTime<Utc>,
    /// Synthesis confidence from the reasoning result.
    pub synthesis_confidence: f64,
}

impl ReasoningObservation {
    /// Create an observation from a reasoning result.
    pub fn from_result(
        result: &ReasoningResult,
        domain: impl Into<String>,
        intent_type: impl Into<String>,
        outcome: ObservationOutcome,
    ) -> Self {
        let contributing_modes: Vec<ReasoningMode> = result
            .mode_summary
            .iter()
            .filter(|(_, active)| *active)
            .filter_map(|(label, _)| label_to_mode(label))
            .collect();

        let primary_mode = result.primary.as_ref().map(|c| c.mode.clone());

        Self {
            id: Uuid::new_v4().to_string(),
            contributing_modes,
            primary_mode,
            domain: domain.into(),
            intent_type: intent_type.into(),
            outcome,
            timestamp: Utc::now(),
            synthesis_confidence: result.synthesis_confidence,
        }
    }

    /// Return a one-line summary for display.
    pub fn summary(&self) -> String {
        let mode_count = self.contributing_modes.len();
        let primary_label = self
            .primary_mode
            .as_ref()
            .map(|m| m.label().to_string())
            .unwrap_or_else(|| "none".to_string());
        format!(
            "obs[{}] domain={} primary={} modes={} outcome={:?}",
            &self.id[..8],
            self.domain,
            primary_label,
            mode_count,
            self.outcome,
        )
    }
}

/// Convert a mode label string back to a `ReasoningMode`.
fn label_to_mode(label: &str) -> Option<ReasoningMode> {
    match label {
        "deductive" => Some(ReasoningMode::Deductive),
        "inductive" => Some(ReasoningMode::Inductive),
        "abductive" => Some(ReasoningMode::Abductive),
        "analogical" => Some(ReasoningMode::Analogical),
        "adversarial" => Some(ReasoningMode::Adversarial),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcome_variants() {
        assert_eq!(ObservationOutcome::Correct, ObservationOutcome::Correct);
        assert_ne!(ObservationOutcome::Correct, ObservationOutcome::Incorrect);
    }

    #[test]
    fn label_to_mode_roundtrip() {
        let modes = [
            ReasoningMode::Deductive,
            ReasoningMode::Inductive,
            ReasoningMode::Abductive,
            ReasoningMode::Analogical,
            ReasoningMode::Adversarial,
        ];
        for mode in &modes {
            let label = mode.label();
            let recovered = label_to_mode(label);
            assert!(recovered.is_some());
            assert_eq!(recovered.as_ref().map(|m| m.label()), Some(label));
        }
    }

    #[test]
    fn unknown_label_returns_none() {
        assert!(label_to_mode("nonexistent").is_none());
    }
}
