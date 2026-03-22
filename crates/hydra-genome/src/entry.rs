//! Genome entries — individual capability records.

use crate::constants::GENOME_MIN_CONFIDENCE;
use crate::signature::{ApproachSignature, SituationSignature};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single genome entry recording a situation-approach pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenomeEntry {
    /// Unique identifier for this entry.
    pub id: String,
    /// The situation this entry applies to.
    pub situation: SituationSignature,
    /// The approach used in this situation.
    pub approach: ApproachSignature,
    /// Initial confidence when this entry was created.
    pub initial_confidence: f64,
    /// Number of times this entry has been used.
    pub use_count: u64,
    /// Number of successful uses.
    pub success_count: u64,
    /// When this entry was created.
    pub created_at: DateTime<Utc>,
    /// When this entry was last used.
    pub last_used_at: DateTime<Utc>,
}

impl GenomeEntry {
    /// Create a genome entry from an operation description.
    ///
    /// Initial confidence is clamped to [0.0, 1.0].
    pub fn from_operation(description: &str, approach: ApproachSignature, confidence: f64) -> Self {
        let now = Utc::now();
        let clamped = confidence.clamp(0.0, 1.0);
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            situation: SituationSignature::from_description(description),
            approach,
            initial_confidence: clamped,
            use_count: 0,
            success_count: 0,
            created_at: now,
            last_used_at: now,
        }
    }

    /// Record a use of this genome entry.
    ///
    /// Increments use and success counts, updates last-used timestamp.
    pub fn record_use(&mut self, success: bool) {
        self.use_count += 1;
        if success {
            self.success_count += 1;
        }
        self.last_used_at = Utc::now();
    }

    /// Compute effective confidence using Bayesian updating.
    ///
    /// Models confidence as Beta(α, β) distribution:
    ///   Prior:     α₀ = initial_confidence × 10, β₀ = (1 - initial_confidence) × 10
    ///   Posterior: α = α₀ + successes, β = β₀ + failures
    ///   E[θ] = α / (α + β)
    ///
    /// This naturally handles: high initial confidence with no uses → wide prior,
    /// many successful uses → converges to observed rate,
    /// few uses → stays near prior.
    pub fn effective_confidence(&self) -> f64 {
        let prior_strength = 10.0;
        let alpha_0 = self.initial_confidence * prior_strength;
        let beta_0 = (1.0 - self.initial_confidence) * prior_strength;

        let alpha = alpha_0 + self.success_count as f64;
        let beta = beta_0 + (self.use_count - self.success_count) as f64;

        let posterior_mean = alpha / (alpha + beta);
        posterior_mean.clamp(GENOME_MIN_CONFIDENCE, 1.0)
    }

    /// Generate a calibrated confidence statement (CCA).
    /// Computed in Rust — not LLM-generated. Mathematically grounded.
    pub fn confidence_statement(&self) -> String {
        let prior_strength = 10.0;
        let alpha = self.initial_confidence * prior_strength + self.success_count as f64;
        let beta = (1.0 - self.initial_confidence) * prior_strength
            + (self.use_count.saturating_sub(self.success_count)) as f64;
        let mean = alpha / (alpha + beta);
        let variance = (alpha * beta) / ((alpha + beta).powi(2) * (alpha + beta + 1.0));
        let std_dev = variance.sqrt();
        let lower = (mean - 1.96 * std_dev).max(0.0);
        let upper = (mean + 1.96 * std_dev).min(1.0);

        let strength = if mean > 0.85 && (upper - lower) < 0.15 {
            "STRONG"
        } else if mean > 0.60 {
            "MODERATE"
        } else {
            "EXPLORATORY"
        };

        let obs = self.use_count.max(
            (self.initial_confidence * prior_strength) as u64,
        );

        format!(
            "conf={:.0}% [{:.0}%-{:.0}%] obs={} strength={}",
            mean * 100.0,
            lower * 100.0,
            upper * 100.0,
            obs,
            strength,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signature::ApproachSignature;

    fn make_approach() -> ApproachSignature {
        ApproachSignature::new("test", vec!["step1".into()], vec!["tool1".into()])
    }

    #[test]
    fn from_operation_clamps_confidence() {
        let entry = GenomeEntry::from_operation("test task", make_approach(), 1.5);
        assert!((entry.initial_confidence - 1.0).abs() < f64::EPSILON);

        let entry2 = GenomeEntry::from_operation("test task", make_approach(), -0.5);
        assert!((entry2.initial_confidence).abs() < f64::EPSILON);
    }

    #[test]
    fn record_use_increments() {
        let mut entry = GenomeEntry::from_operation("test task", make_approach(), 0.8);
        entry.record_use(true);
        assert_eq!(entry.use_count, 1);
        assert_eq!(entry.success_count, 1);

        entry.record_use(false);
        assert_eq!(entry.use_count, 2);
        assert_eq!(entry.success_count, 1);
    }

    #[test]
    fn effective_confidence_unused() {
        let entry = GenomeEntry::from_operation("test", make_approach(), 0.5);
        assert!((entry.effective_confidence() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn effective_confidence_bayesian_update() {
        let mut entry = GenomeEntry::from_operation("test", make_approach(), 0.5);
        entry.record_use(true);
        entry.record_use(true);
        let ec = entry.effective_confidence();
        // Beta(5+2, 5+0) = Beta(7, 5) → E[θ] = 7/12 ≈ 0.583
        assert!((ec - 7.0 / 12.0).abs() < 0.01);
    }

    #[test]
    fn effective_confidence_clamped() {
        let mut entry = GenomeEntry::from_operation("test", make_approach(), 1.0);
        for _ in 0..100 {
            entry.record_use(true);
        }
        assert!(entry.effective_confidence() <= 1.0);
    }
}
