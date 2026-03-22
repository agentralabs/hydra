//! CrystallizationSource — operational data that feeds artifact generation.

use hydra_attribution::AttributionTree;
use hydra_settlement::SettlementRecord;

/// What data is available for crystallization.
#[derive(Debug, Clone, Default)]
pub struct CrystallizationSource {
    pub domain: String,
    /// Successful settlement records for this domain.
    pub successes: Vec<SettlementRecord>,
    /// Failed/denied settlement records.
    pub failures: Vec<SettlementRecord>,
    /// Attribution trees (WHY things happened).
    pub attributions: Vec<AttributionTree>,
    /// Known avoidable patterns.
    pub avoidable_causes: Vec<String>,
    /// Known successful approaches (from genome-like patterns).
    pub proven_approaches: Vec<(String, f64)>, // (approach, confidence)
}

impl CrystallizationSource {
    pub fn new(domain: impl Into<String>) -> Self {
        Self {
            domain: domain.into(),
            ..Default::default()
        }
    }

    pub fn with_success(mut self, r: SettlementRecord) -> Self {
        self.successes.push(r);
        self
    }

    pub fn with_failure(mut self, r: SettlementRecord) -> Self {
        self.failures.push(r);
        self
    }

    pub fn with_attribution(mut self, t: AttributionTree) -> Self {
        self.attributions.push(t);
        self
    }

    pub fn with_approach(mut self, approach: impl Into<String>, confidence: f64) -> Self {
        self.proven_approaches.push((approach.into(), confidence));
        self
    }

    pub fn with_avoidable(mut self, cause: impl Into<String>) -> Self {
        self.avoidable_causes.push(cause.into());
        self
    }

    pub fn total_records(&self) -> usize {
        self.successes.len() + self.failures.len()
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.total_records();
        if total == 0 {
            return 0.0;
        }
        self.successes.len() as f64 / total as f64
    }
}
