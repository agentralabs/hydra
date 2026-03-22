//! PatternObservation — one peer's sighting of a pattern.
//! Carries peer identity, trust score, and confidence.

use serde::{Deserialize, Serialize};

/// One pattern observation from one peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternObservation {
    pub id: String,
    pub topic: String,
    pub peer_id: String,
    pub peer_trust: f64,
    pub confidence: f64,
    pub count: usize,
    pub description: String,
    pub domain: String,
    pub observed_at: chrono::DateTime<chrono::Utc>,
}

impl PatternObservation {
    pub fn new(
        topic: impl Into<String>,
        peer_id: impl Into<String>,
        peer_trust: f64,
        confidence: f64,
        count: usize,
        description: impl Into<String>,
        domain: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            topic: topic.into(),
            peer_id: peer_id.into(),
            peer_trust: peer_trust.clamp(0.0, 1.0),
            confidence: confidence.clamp(0.0, 1.0),
            count,
            description: description.into(),
            domain: domain.into(),
            observed_at: chrono::Utc::now(),
        }
    }

    /// Trust-weighted contribution of this observation.
    pub fn weighted_confidence(&self) -> f64 {
        self.confidence
            * self
                .peer_trust
                .powf(crate::constants::TRUST_WEIGHT_EXPONENT)
    }

    /// Total weighted observations (count x weighted confidence).
    pub fn weighted_count(&self) -> f64 {
        self.count as f64 * self.weighted_confidence()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_trust_weights_higher() {
        let high = PatternObservation::new("t", "a", 0.9, 0.8, 5, "d", "eng");
        let low = PatternObservation::new("t", "b", 0.3, 0.8, 5, "d", "eng");
        assert!(high.weighted_confidence() > low.weighted_confidence());
    }

    #[test]
    fn weighted_count_scales_with_observations() {
        let few = PatternObservation::new("t", "a", 0.8, 0.9, 2, "d", "eng");
        let many = PatternObservation::new("t", "a", 0.8, 0.9, 10, "d", "eng");
        assert!(many.weighted_count() > few.weighted_count());
    }
}
