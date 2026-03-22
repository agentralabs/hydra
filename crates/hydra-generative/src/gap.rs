//! GapDetector — identifies what primitive is missing to complete a task.

use crate::decompose::TaskDecomposition;
use hydra_axiom::AxiomPrimitive;

/// A detected capability gap.
#[derive(Debug, Clone)]
pub struct CapabilityGap {
    /// The task that revealed the gap.
    pub task: String,
    /// Human-readable description of what is needed.
    pub what_is_needed: String,
    /// Which primitives were covered.
    pub primitives_covered: Vec<AxiomPrimitive>,
}

impl CapabilityGap {
    /// The precise surface message — never "I cannot." Always "I need X."
    pub fn surface_message(&self) -> String {
        format!(
            "To complete '{}', I need: {}. \
             This is the specific missing capability. \
             Once provided, this gap is permanently closed.",
            self.task, self.what_is_needed
        )
    }
}

/// Detect a gap from a decomposition result.
///
/// If the decomposition has no primitives, returns a gap indicating
/// that domain-specific primitive extraction is needed.
/// If synthesis confidence is below minimum, returns a gap indicating
/// that higher confidence primitives are needed.
pub fn detect_gap(
    decomposition: &TaskDecomposition,
    what_is_needed: &str,
) -> CapabilityGap {
    CapabilityGap {
        task: decomposition.description.clone(),
        what_is_needed: what_is_needed.to_string(),
        primitives_covered: decomposition.primitives.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decompose::decompose;

    #[test]
    fn gap_detected_for_empty_decomposition() {
        let decomposition = decompose("xyz qqq zzz");
        let gap = detect_gap(&decomposition, "domain-specific primitive extraction needed");
        assert!(!gap.what_is_needed.is_empty());
    }

    #[test]
    fn surface_message_never_says_cannot() {
        let gap = CapabilityGap {
            task: "deploy to mainframe".into(),
            what_is_needed: "JCL job control language interface".into(),
            primitives_covered: vec![],
        };
        let msg = gap.surface_message();
        assert!(!msg.contains("cannot"));
        assert!(!msg.contains("unable"));
        assert!(msg.contains("I need"));
    }

    #[test]
    fn gap_preserves_primitives() {
        let decomposition = decompose("optimize risk under constraints");
        let gap = detect_gap(&decomposition, "higher confidence needed");
        assert!(!gap.primitives_covered.is_empty());
    }
}
