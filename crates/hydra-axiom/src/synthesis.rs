//! Capability synthesis from axiom primitives.

use crate::constants::SYNTHESIS_CONFIDENCE_FLOOR;
use crate::errors::AxiomError;
use crate::morphisms::AxiomMorphism;
use crate::primitives::AxiomPrimitive;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A synthesized capability derived from multiple axiom primitives.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesizedCapability {
    /// Unique identifier for this synthesized capability.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// The axiom primitives that compose this capability.
    pub components: Vec<AxiomPrimitive>,
    /// The morphisms connecting the components.
    pub connections: Vec<(usize, usize, AxiomMorphism)>,
    /// Confidence in the synthesis (0.0 to 1.0).
    pub confidence: f64,
}

/// Synthesize a new capability from a set of axiom primitives and connections.
///
/// Returns an error if no components are provided.
/// Confidence is computed as the average similarity of connected pairs,
/// floored at `SYNTHESIS_CONFIDENCE_FLOOR`.
pub fn synthesize(
    name: impl Into<String>,
    components: Vec<AxiomPrimitive>,
    connections: Vec<(usize, usize, AxiomMorphism)>,
) -> Result<SynthesizedCapability, AxiomError> {
    if components.is_empty() {
        return Err(AxiomError::SynthesisMissingPrimitive);
    }

    let confidence = if connections.is_empty() {
        SYNTHESIS_CONFIDENCE_FLOOR
    } else {
        let total: f64 = connections
            .iter()
            .filter_map(|(a, b, _)| {
                let pa = components.get(*a)?;
                let pb = components.get(*b)?;
                Some(pa.similarity(pb))
            })
            .sum();
        let avg = total / connections.len() as f64;
        avg.max(SYNTHESIS_CONFIDENCE_FLOOR)
    };

    Ok(SynthesizedCapability {
        id: Uuid::new_v4().to_string(),
        name: name.into(),
        components,
        connections,
        confidence,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_components_rejected() {
        let result = synthesize("test", vec![], vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn single_component_synthesis() {
        let cap = synthesize("risk-monitor", vec![AxiomPrimitive::Risk], vec![]).unwrap();
        assert_eq!(cap.name, "risk-monitor");
        assert!((cap.confidence - SYNTHESIS_CONFIDENCE_FLOOR).abs() < f64::EPSILON);
    }

    #[test]
    fn multi_component_synthesis() {
        let cap = synthesize(
            "risk-optimizer",
            vec![AxiomPrimitive::Risk, AxiomPrimitive::Optimization],
            vec![(0, 1, AxiomMorphism::OptimizesFor)],
        )
        .unwrap();
        assert!(!cap.id.is_empty());
        assert!(cap.confidence >= SYNTHESIS_CONFIDENCE_FLOOR);
    }
}
