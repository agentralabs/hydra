//! AcquisitionResult — what was found and how confident we are.

use crate::constants::MIN_ACQUISITION_CONFIDENCE;
use serde::{Deserialize, Serialize};

/// The result of one acquisition attempt from one source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcquisitionResult {
    pub gap_id:      String,
    pub source:      String,
    pub content:     String,
    pub confidence:  f64,
    pub provenance:  String,
    pub acquired_at: chrono::DateTime<chrono::Utc>,
}

impl AcquisitionResult {
    pub fn new(
        gap_id:     &str,
        source:     &str,
        content:    impl Into<String>,
        confidence: f64,
        provenance: impl Into<String>,
    ) -> Self {
        Self {
            gap_id:      gap_id.to_string(),
            source:      source.to_string(),
            content:     content.into(),
            confidence:  confidence.clamp(0.0, 1.0),
            provenance:  provenance.into(),
            acquired_at: chrono::Utc::now(),
        }
    }

    pub fn meets_threshold(&self) -> bool {
        self.confidence >= MIN_ACQUISITION_CONFIDENCE
    }

    /// Format for belief integration — what enters the manifold.
    pub fn belief_statement(&self) -> String {
        format!(
            "[acquired from {} | confidence:{:.2}] {}",
            self.source,
            self.confidence,
            self.content,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_confidence_meets_threshold() {
        let r = AcquisitionResult::new(
            "gap-1", "codebase",
            "Kubernetes rolling update uses maxSurge and maxUnavailable",
            0.87, "github.com/kubernetes/kubernetes",
        );
        assert!(r.meets_threshold());
    }

    #[test]
    fn low_confidence_below_threshold() {
        let r = AcquisitionResult::new(
            "gap-1", "web", "uncertain information", 0.3, "unknown",
        );
        assert!(!r.meets_threshold());
    }

    #[test]
    fn belief_statement_contains_source() {
        let r = AcquisitionResult::new(
            "gap-1", "agentic-codebase",
            "knowledge content", 0.85, "provenance",
        );
        let bs = r.belief_statement();
        assert!(bs.contains("agentic-codebase"));
        assert!(bs.contains("0.85"));
    }
}
