//! Structural pattern — captures the primitive-type signature of a domain input.

use chrono::{DateTime, Utc};
use hydra_axiom::AxiomPrimitive;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A structural pattern extracted from comprehended input.
///
/// Captures which axiom primitive types appeared, enabling cross-domain
/// comparison via Jaccard similarity over primitive type labels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuralPattern {
    /// Unique identifier for this pattern.
    pub id: String,
    /// The domain this pattern was observed in.
    pub domain: String,
    /// Sorted, deduplicated list of axiom primitive variant names.
    pub primitive_types: Vec<String>,
    /// Human-readable description of the pattern.
    pub description: String,
    /// Confidence in the pattern (0.0 to 1.0).
    pub confidence: f64,
    /// When this pattern was created.
    pub timestamp: DateTime<Utc>,
}

impl StructuralPattern {
    /// Create a structural pattern from axiom primitives.
    pub fn from_primitives(
        domain: impl Into<String>,
        primitives: &[AxiomPrimitive],
        description: impl Into<String>,
    ) -> Self {
        let mut labels: Vec<String> = primitives.iter().map(|p| p.label().to_string()).collect();
        labels.sort();
        labels.dedup();

        Self {
            id: Uuid::new_v4().to_string(),
            domain: domain.into(),
            primitive_types: labels,
            description: description.into(),
            confidence: compute_pattern_confidence(primitives),
            timestamp: Utc::now(),
        }
    }

    /// Compute Jaccard similarity with another pattern.
    ///
    /// Jaccard = |intersection| / |union| over primitive type labels.
    pub fn similarity(&self, other: &Self) -> f64 {
        if self.primitive_types.is_empty() && other.primitive_types.is_empty() {
            return 1.0;
        }
        if self.primitive_types.is_empty() || other.primitive_types.is_empty() {
            return 0.0;
        }

        let set_a: std::collections::HashSet<&str> =
            self.primitive_types.iter().map(|s| s.as_str()).collect();
        let set_b: std::collections::HashSet<&str> =
            other.primitive_types.iter().map(|s| s.as_str()).collect();

        let intersection = set_a.intersection(&set_b).count();
        let union = set_a.union(&set_b).count();

        if union == 0 {
            return 0.0;
        }

        intersection as f64 / union as f64
    }

    /// Return a one-line summary for display.
    pub fn summary(&self) -> String {
        format!(
            "pattern[{}] domain={} primitives=[{}] conf={:.2}",
            &self.id[..8],
            self.domain,
            self.primitive_types.join(", "),
            self.confidence,
        )
    }
}

/// Compute pattern confidence based on the number and diversity of primitives.
fn compute_pattern_confidence(primitives: &[AxiomPrimitive]) -> f64 {
    if primitives.is_empty() {
        return 0.0;
    }
    let mut labels: Vec<String> = primitives.iter().map(|p| p.label().to_string()).collect();
    labels.sort();
    labels.dedup();
    // More unique primitives = higher confidence, capped at 1.0.
    (labels.len() as f64 * 0.2).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydra_axiom::AxiomPrimitive;

    #[test]
    fn same_primitives_similarity_one() {
        let a = StructuralPattern::from_primitives(
            "eng",
            &[AxiomPrimitive::Risk, AxiomPrimitive::CausalLink],
            "test a",
        );
        let b = StructuralPattern::from_primitives(
            "fin",
            &[AxiomPrimitive::Risk, AxiomPrimitive::CausalLink],
            "test b",
        );
        assert!((a.similarity(&b) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn disjoint_primitives_similarity_zero() {
        let a = StructuralPattern::from_primitives("eng", &[AxiomPrimitive::Risk], "test a");
        let b =
            StructuralPattern::from_primitives("fin", &[AxiomPrimitive::Optimization], "test b");
        assert!((a.similarity(&b)).abs() < f64::EPSILON);
    }

    #[test]
    fn partial_overlap() {
        let a = StructuralPattern::from_primitives(
            "eng",
            &[AxiomPrimitive::Risk, AxiomPrimitive::CausalLink],
            "test a",
        );
        let b = StructuralPattern::from_primitives(
            "fin",
            &[
                AxiomPrimitive::Risk,
                AxiomPrimitive::CausalLink,
                AxiomPrimitive::Optimization,
            ],
            "test b",
        );
        // Jaccard: 2/3
        let sim = a.similarity(&b);
        assert!((sim - 2.0 / 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn empty_primitives() {
        let a = StructuralPattern::from_primitives("eng", &[], "test a");
        let b = StructuralPattern::from_primitives("fin", &[], "test b");
        assert!((a.similarity(&b) - 1.0).abs() < f64::EPSILON);
    }
}
