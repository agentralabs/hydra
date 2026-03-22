//! SharedBelief — a belief received from another Hydra agent.
//! Carries provenance and evidence alongside the claim.

use serde::{Deserialize, Serialize};

/// One belief from one agent — to be merged with our own.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedBelief {
    pub id: String,
    pub topic: String,
    pub claim: String,
    pub confidence: f64,
    pub evidence_count: usize,
    pub evidence_labels: Vec<String>,
    pub source_peer_id: String,
    pub calibration_offset: f64,
    pub received_at: chrono::DateTime<chrono::Utc>,
}

impl SharedBelief {
    pub fn new(
        topic: impl Into<String>,
        claim: impl Into<String>,
        confidence: f64,
        evidence_count: usize,
        evidence_labels: Vec<String>,
        source_peer_id: impl Into<String>,
        calibration_offset: f64,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            topic: topic.into(),
            claim: claim.into(),
            confidence: confidence.clamp(0.0, 1.0),
            evidence_count,
            evidence_labels,
            source_peer_id: source_peer_id.into(),
            calibration_offset,
            received_at: chrono::Utc::now(),
        }
    }

    /// Calibration-adjusted confidence.
    pub fn adjusted_confidence(&self) -> f64 {
        (self.confidence + self.calibration_offset).clamp(0.0, 1.0)
    }

    /// Evidence strength (normalized 0.0–1.0 based on count).
    pub fn evidence_strength(&self) -> f64 {
        (self.evidence_count as f64 / 100.0).min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calibration_applied() {
        let b = SharedBelief::new("topic", "claim", 0.80, 10, vec![], "peer-a", -0.10);
        assert!((b.adjusted_confidence() - 0.70).abs() < 1e-10);
    }

    #[test]
    fn evidence_strength_bounded() {
        let strong = SharedBelief::new("t", "c", 0.8, 150, vec![], "p", 0.0);
        let weak = SharedBelief::new("t", "c", 0.8, 2, vec![], "p", 0.0);
        assert_eq!(strong.evidence_strength(), 1.0);
        assert!(weak.evidence_strength() < 1.0);
    }
}
