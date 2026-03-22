//! AdoptionRecord — when an instance adopts a pattern.
//! Provenance preserved. Outcomes tracked back to the source.

use serde::{Deserialize, Serialize};

/// One adoption record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdoptionRecord {
    pub id:              String,
    pub pattern_id:      String,
    pub pattern_title:   String,
    pub adopting_lineage: String,
    pub source_lineage:  String,
    pub adopted_confidence: f64,
    pub provenance:      String,
    pub outcome_count:   usize,
    pub success_count:   usize,
    pub adopted_at:      chrono::DateTime<chrono::Utc>,
}

impl AdoptionRecord {
    pub fn new(
        pattern_id:       &str,
        pattern_title:    &str,
        adopting_lineage: impl Into<String>,
        source_lineage:   &str,
        confidence:       f64,
        source_days:      u32,
    ) -> Self {
        let adopter = adopting_lineage.into();
        let provenance = format!(
            "Adopted from '{}' (day {}, confidence {:.2}). Pattern: '{}'. \
             Provenance preserved — source lineage retains attribution.",
            source_lineage, source_days, confidence, pattern_id,
        );
        Self {
            id:                  uuid::Uuid::new_v4().to_string(),
            pattern_id:          pattern_id.to_string(),
            pattern_title:       pattern_title.to_string(),
            adopting_lineage:    adopter,
            source_lineage:      source_lineage.to_string(),
            adopted_confidence:  confidence.clamp(0.0, 1.0),
            provenance,
            outcome_count:       0,
            success_count:       0,
            adopted_at:          chrono::Utc::now(),
        }
    }

    /// Record an outcome for this adopted pattern.
    pub fn record_outcome(&mut self, success: bool) {
        self.outcome_count += 1;
        if success { self.success_count += 1; }
    }

    pub fn success_rate(&self) -> Option<f64> {
        if self.outcome_count == 0 { return None; }
        Some(self.success_count as f64 / self.outcome_count as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adoption_provenance_contains_source() {
        let a = AdoptionRecord::new(
            "pattern-123", "Circuit Breaker",
            "hydra-b-lineage", "hydra-agentra-lineage",
            0.88, 7300,
        );
        assert!(a.provenance.contains("hydra-agentra-lineage"));
        assert!(a.provenance.contains("7300"));
        assert!(a.provenance.contains("0.88"));
    }

    #[test]
    fn outcome_tracking() {
        let mut a = AdoptionRecord::new(
            "p", "t", "adopter", "source", 0.85, 1000,
        );
        a.record_outcome(true);
        a.record_outcome(true);
        a.record_outcome(false);
        assert_eq!(a.outcome_count, 3);
        assert!((a.success_rate().unwrap() - 0.667).abs() < 0.01);
    }
}
