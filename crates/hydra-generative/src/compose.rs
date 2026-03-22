//! CapabilityComposer — composes axiom primitives into a new approach.

use crate::constants::SYNTHESIS_MIN_CONFIDENCE;
use crate::decompose::TaskDecomposition;

/// The result of composing primitives into a capability.
#[derive(Debug, Clone)]
pub struct CompositionResult {
    /// The name of the composed capability.
    pub capability_name: String,
    /// Confidence in the composition (0.0 to 1.0).
    pub confidence: f64,
}

/// Compute confidence for a synthesized capability.
///
/// Based on the number and type of primitives extracted.
/// More base primitives and broader coverage yield higher confidence.
pub fn compute_synthesis_confidence(decomposition: &TaskDecomposition) -> f64 {
    let base_count = decomposition
        .primitives
        .iter()
        .filter(|p| p.is_base())
        .count();
    let total = decomposition.primitives.len();

    if total == 0 {
        return 0.0;
    }

    let base_ratio = base_count as f64 / total as f64;
    let coverage_bonus = (total as f64 / 5.0).min(1.0) * 0.2;
    (base_ratio * 0.6 + coverage_bonus + SYNTHESIS_MIN_CONFIDENCE).clamp(0.0, 1.0)
}

/// Generate a human-readable name for a synthesized capability.
///
/// Combines the labels of up to 3 leading primitives.
pub fn generate_capability_name(decomposition: &TaskDecomposition) -> String {
    let labels: Vec<&str> = decomposition
        .primitives
        .iter()
        .take(3)
        .map(|p| p.label())
        .collect();
    if labels.is_empty() {
        return "synthesized-capability".to_string();
    }
    labels.join("-")
}

/// Compose primitives into a capability result.
///
/// Combines confidence computation and naming into a single result.
pub fn compose(decomposition: &TaskDecomposition) -> CompositionResult {
    CompositionResult {
        capability_name: generate_capability_name(decomposition),
        confidence: compute_synthesis_confidence(decomposition),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decompose::decompose;

    #[test]
    fn confidence_above_minimum_for_known_task() {
        let td = decompose("optimize resource allocation under constraints");
        let confidence = compute_synthesis_confidence(&td);
        assert!(confidence >= SYNTHESIS_MIN_CONFIDENCE);
    }

    #[test]
    fn confidence_zero_for_empty() {
        let td = decompose("");
        let confidence = compute_synthesis_confidence(&td);
        assert!((confidence - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn name_generated_from_primitives() {
        let td = decompose("optimize risk under constraints");
        let name = generate_capability_name(&td);
        assert!(!name.is_empty());
        assert_ne!(name, "synthesized-capability");
    }

    #[test]
    fn compose_returns_valid_result() {
        let td = decompose("optimize resource allocation under constraints");
        let result = compose(&td);
        assert!(result.confidence >= SYNTHESIS_MIN_CONFIDENCE);
        assert!(!result.capability_name.is_empty());
    }
}
