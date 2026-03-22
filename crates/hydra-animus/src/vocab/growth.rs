//! Growth layer vocabulary — base node and edge types for Tier 7 crates.
//! Registered as base vocabulary — always present, never removed.
//! These types enable hydra-genome, hydra-cartography, hydra-antifragile,
//! hydra-generative, and hydra-plastic to communicate via Animus Prime.

use crate::graph::NodeType;

// ── Growth Layer Node Type Names ───────────────────────────────────────

/// All growth layer node type names.
/// These are registered in the base vocabulary alongside the core types.
pub const GROWTH_NODE_TYPES: &[&str] = &[
    // hydra-genome
    "GenomeEntry",        // a proven approach from operational experience
    "SituationSignature", // fingerprint of a task context
    "ObstacleSignature",  // fingerprint of an obstacle encountered
    "ApproachRecord",     // what Hydra did to resolve a situation
    // hydra-cartography
    "SystemProfile",      // a mapped digital system type
    "SystemClass",        // the class of a system (API, embedded, mainframe, etc.)
    "ProtocolFamily",     // the protocol family a system speaks
    "InterfaceSignature", // fingerprint of a system's interface
    "TopologyNeighbor",   // a similar system in the cartography
    // hydra-antifragile
    "AntifragileRecord",  // obstacle class + proven resolution + resistance gained
    "ObstacleClass",      // the category of obstacle
    "ResistanceLevel",    // how resistant Hydra is to this obstacle class now
    // hydra-generative
    "SynthesisAttempt",      // an attempt to synthesize a new capability
    "MissingPrimitive",      // an axiom primitive that is not yet covered
    "SynthesizedCapability", // a capability invented from first principles
    // hydra-plastic
    "PlasticityProfile",      // execution profile for a specific environment
    "EnvironmentConstraint",  // a constraint of an execution environment
    "ExecutionMode",          // how Hydra is executing in an environment
    // growth layer signals
    "CapabilityGrowthEvent", // Γ̂(Ψ) delta — a capability was gained
    "GrowthInvariantCheck",  // result of a Γ̂ ≥ 0 verification
];

/// All growth layer edge type names.
pub const GROWTH_EDGE_TYPES: &[&str] = &[
    "LearnedFrom",      // genome entry was learned from this experience
    "SimilarTo",        // two system profiles are topologically similar
    "ResolvedBy",       // obstacle was resolved by this approach
    "SynthesizedUsing", // capability was synthesized using these primitives
    "AdaptedFor",       // execution was adapted for this environment
    "StrengthensFrom",  // resistance increases from this encounter
    "GrowsBy",          // Γ̂ contribution from this event
];

/// Returns true if a node type name is in the growth layer vocabulary.
pub fn is_growth_node_type(name: &str) -> bool {
    GROWTH_NODE_TYPES.contains(&name)
}

/// Returns true if an edge type name is in the growth layer vocabulary.
pub fn is_growth_edge_type(name: &str) -> bool {
    GROWTH_EDGE_TYPES.contains(&name)
}

/// Convenience: build a Domain NodeType for a growth layer node.
/// Growth layer nodes are in the "hydra-growth" domain.
pub fn growth_node_type(name: impl Into<String>) -> NodeType {
    NodeType::Domain {
        domain: "hydra-growth".to_string(),
        name: name.into(),
    }
}

/// Which growth layer a node type belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrowthLayer {
    /// Genome layer: task patterns and skill acquisitions.
    Genome,
    /// Cartography layer: system profiles and environment maps.
    Cartography,
    /// Antifragile layer: error patterns that strengthen behavior.
    Antifragile,
    /// Generative layer: capability synthesis from first principles.
    Generative,
    /// Plastic layer: environment-adaptive execution.
    Plastic,
    /// Cross-cutting growth metric or signal.
    Metric,
}

/// Returns the growth layer a node type belongs to, if any.
pub fn growth_layer(name: &str) -> Option<GrowthLayer> {
    match name {
        "GenomeEntry" | "SituationSignature" | "ObstacleSignature" | "ApproachRecord" => {
            Some(GrowthLayer::Genome)
        }
        "SystemProfile" | "SystemClass" | "ProtocolFamily" | "InterfaceSignature"
        | "TopologyNeighbor" => Some(GrowthLayer::Cartography),
        "AntifragileRecord" | "ObstacleClass" | "ResistanceLevel" => {
            Some(GrowthLayer::Antifragile)
        }
        "SynthesisAttempt" | "MissingPrimitive" | "SynthesizedCapability" => {
            Some(GrowthLayer::Generative)
        }
        "PlasticityProfile" | "EnvironmentConstraint" | "ExecutionMode" => {
            Some(GrowthLayer::Plastic)
        }
        "CapabilityGrowthEvent" | "GrowthInvariantCheck" => Some(GrowthLayer::Metric),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn growth_node_types_non_empty() {
        assert!(!GROWTH_NODE_TYPES.is_empty());
    }

    #[test]
    fn genome_entry_is_growth_type() {
        assert!(is_growth_node_type("GenomeEntry"));
    }

    #[test]
    fn system_profile_is_growth_type() {
        assert!(is_growth_node_type("SystemProfile"));
    }

    #[test]
    fn antifragile_record_is_growth_type() {
        assert!(is_growth_node_type("AntifragileRecord"));
    }

    #[test]
    fn synthesized_capability_is_growth_type() {
        assert!(is_growth_node_type("SynthesizedCapability"));
    }

    #[test]
    fn plasticity_profile_is_growth_type() {
        assert!(is_growth_node_type("PlasticityProfile"));
    }

    #[test]
    fn growth_edge_type_check() {
        assert!(is_growth_edge_type("LearnedFrom"));
        assert!(is_growth_edge_type("GrowsBy"));
        assert!(!is_growth_edge_type("References")); // base type, not growth
    }

    #[test]
    fn growth_node_type_builder() {
        let t = growth_node_type("GenomeEntry");
        assert!(t.is_domain());
        assert_eq!(t.domain_name(), Some("hydra-growth"));
    }

    #[test]
    fn capability_growth_event_is_registered() {
        assert!(is_growth_node_type("CapabilityGrowthEvent"));
        assert!(is_growth_node_type("GrowthInvariantCheck"));
    }

    #[test]
    fn all_growth_nodes_have_layers() {
        for name in GROWTH_NODE_TYPES {
            assert!(
                growth_layer(name).is_some(),
                "{} should have a growth layer",
                name
            );
        }
    }

    #[test]
    fn growth_layer_classification() {
        assert_eq!(growth_layer("GenomeEntry"), Some(GrowthLayer::Genome));
        assert_eq!(
            growth_layer("SystemProfile"),
            Some(GrowthLayer::Cartography)
        );
        assert_eq!(
            growth_layer("AntifragileRecord"),
            Some(GrowthLayer::Antifragile)
        );
        assert_eq!(
            growth_layer("SynthesizedCapability"),
            Some(GrowthLayer::Generative)
        );
        assert_eq!(
            growth_layer("PlasticityProfile"),
            Some(GrowthLayer::Plastic)
        );
        assert_eq!(
            growth_layer("CapabilityGrowthEvent"),
            Some(GrowthLayer::Metric)
        );
        assert_eq!(growth_layer("Unknown"), None);
    }

    #[test]
    fn new_node_types_present() {
        // Verify all spec-required node types
        assert!(is_growth_node_type("SituationSignature"));
        assert!(is_growth_node_type("ObstacleSignature"));
        assert!(is_growth_node_type("ApproachRecord"));
        assert!(is_growth_node_type("SystemClass"));
        assert!(is_growth_node_type("ProtocolFamily"));
        assert!(is_growth_node_type("InterfaceSignature"));
        assert!(is_growth_node_type("TopologyNeighbor"));
        assert!(is_growth_node_type("ObstacleClass"));
        assert!(is_growth_node_type("ResistanceLevel"));
        assert!(is_growth_node_type("SynthesisAttempt"));
        assert!(is_growth_node_type("MissingPrimitive"));
        assert!(is_growth_node_type("EnvironmentConstraint"));
        assert!(is_growth_node_type("ExecutionMode"));
    }

    #[test]
    fn all_seven_edge_types_present() {
        assert!(is_growth_edge_type("LearnedFrom"));
        assert!(is_growth_edge_type("SimilarTo"));
        assert!(is_growth_edge_type("ResolvedBy"));
        assert!(is_growth_edge_type("SynthesizedUsing"));
        assert!(is_growth_edge_type("AdaptedFor"));
        assert!(is_growth_edge_type("StrengthensFrom"));
        assert!(is_growth_edge_type("GrowsBy"));
        assert_eq!(GROWTH_EDGE_TYPES.len(), 7);
    }

    #[test]
    fn non_growth_types_rejected() {
        assert!(!is_growth_node_type("Belief"));
        assert!(!is_growth_node_type("Unknown"));
        assert!(!is_growth_edge_type("CausalLink"));
    }
}
