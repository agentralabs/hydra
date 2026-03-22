//! Learning record — a proposed weight adjustment.

use chrono::{DateTime, Utc};
use hydra_reasoning::conclusion::ReasoningMode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A proposed weight adjustment for a reasoning mode in a specific domain.
///
/// Learning records are proposals only — they never modify weights directly.
/// The kernel or a human operator must approve and apply them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningRecord {
    /// Unique identifier for this record.
    pub id: String,
    /// The reasoning mode this adjustment targets.
    pub mode: ReasoningMode,
    /// The domain in which the adjustment is proposed.
    pub domain: String,
    /// The current weight of the mode (at time of proposal).
    pub current_weight: f64,
    /// The proposed delta (positive = boost, negative = reduce).
    pub proposed_delta: f64,
    /// Human-readable reason for the proposal.
    pub reason: String,
    /// Confidence in this proposal (0.0 to 1.0).
    pub confidence: f64,
    /// When this record was created.
    pub timestamp: DateTime<Utc>,
}

impl LearningRecord {
    /// Create a new learning record.
    pub fn new(
        mode: ReasoningMode,
        domain: impl Into<String>,
        current_weight: f64,
        proposed_delta: f64,
        reason: impl Into<String>,
        confidence: f64,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            mode,
            domain: domain.into(),
            current_weight,
            proposed_delta,
            reason: reason.into(),
            confidence: confidence.clamp(0.0, 1.0),
            timestamp: Utc::now(),
        }
    }

    /// Return the proposed new weight after applying the delta.
    pub fn proposed_weight(&self) -> f64 {
        (self.current_weight + self.proposed_delta).clamp(0.0, 1.0)
    }

    /// Return a one-line summary for display.
    pub fn summary(&self) -> String {
        let direction = if self.proposed_delta >= 0.0 { "+" } else { "" };
        format!(
            "learn[{}] {}:{} weight={:.3}{}{:.3}={:.3} conf={:.2} reason=\"{}\"",
            &self.id[..8],
            self.domain,
            self.mode.label(),
            self.current_weight,
            direction,
            self.proposed_delta,
            self.proposed_weight(),
            self.confidence,
            self.reason,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proposed_weight_clamped() {
        let r = LearningRecord::new(
            ReasoningMode::Deductive,
            "engineering",
            0.98,
            0.05,
            "high accuracy",
            0.9,
        );
        assert!((r.proposed_weight() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn proposed_weight_negative_clamped() {
        let r = LearningRecord::new(
            ReasoningMode::Inductive,
            "finance",
            0.02,
            -0.05,
            "low accuracy",
            0.8,
        );
        assert!((r.proposed_weight()).abs() < f64::EPSILON);
    }

    #[test]
    fn summary_contains_mode() {
        let r = LearningRecord::new(
            ReasoningMode::Abductive,
            "security",
            0.5,
            0.03,
            "improving",
            0.7,
        );
        let s = r.summary();
        assert!(s.contains("abductive"));
        assert!(s.contains("security"));
    }
}
