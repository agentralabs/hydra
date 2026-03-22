//! Edge types for the Animus Prime semantic graph.
//! Edges are relationships — the "verbs" of the Prime language.

use crate::graph::node::NodeId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for an edge.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EdgeId(String);

impl EdgeId {
    /// Generate a new unique edge ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
    /// Returns the string representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for EdgeId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for EdgeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The type of relationship an edge represents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EdgeType {
    // -- Structural edges --
    /// Parent contains child.
    Contains,
    /// Node references another without owning it.
    References,
    /// Type or soul extends another.
    Extends,
    /// Type implements a trait or interface.
    Implements,

    // -- Data flow edges --
    /// Value flows from source to sink.
    DataFlow { transform: DataTransform },
    /// Node depends on another to function.
    Dependency { kind: DependencyKind },

    // -- Control flow edges --
    /// Execution flows from source to target.
    ControlFlow { condition: Option<String> },
    /// Function calls another function.
    Calls,

    // -- Cognitive edges --
    /// Function or node has persistent memory.
    Remembers,
    /// Node proves a claim.
    Proves,
    /// Node has foresight about another.
    Foresees,
    /// Trust relationship between entities.
    Trusts { tier: u8 },
    /// Node is bound to a soul (identity anchor).
    SoulBound,

    // -- Causal edges — the semiring backbone --
    /// A caused B (composition result).
    CausalLink { strength: f64 },
    /// Temporal ordering relationship.
    TemporalLink { relation: TemporalRelation },

    // -- Domain edges (registered by skills) --
    /// A domain-specific edge type.
    Domain { domain: String, name: String },
}

/// How data is transformed as it flows along a DataFlow edge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DataTransform {
    /// No transformation.
    None,
    /// Mapped transformation.
    Mapped,
    /// Filtered transformation.
    Filtered,
    /// Aggregated transformation.
    Aggregated,
    /// Projected transformation.
    Projected,
}

/// The kind of dependency relationship.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DependencyKind {
    /// Compile-time dependency.
    Compile,
    /// Runtime dependency.
    Runtime,
    /// Test dependency.
    Test,
    /// Optional dependency.
    Optional,
}

/// How two nodes are related in time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TemporalRelation {
    /// Before in time.
    Before,
    /// After in time.
    After,
    /// During the same period.
    During,
    /// A version of another.
    VersionOf,
    /// Precedes in sequence.
    Precedes,
    /// Follows in sequence.
    Follows,
}

/// An edge connecting two nodes in the Prime graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    /// Unique identifier.
    pub id: EdgeId,
    /// The type of relationship.
    pub edge_type: EdgeType,
    /// Source node.
    pub from: NodeId,
    /// Target node.
    pub to: NodeId,
    /// Strength of this relationship (0.0 = weak, 1.0 = strong).
    pub strength: f64,
}

impl Edge {
    /// Create a new edge between two nodes.
    pub fn new(edge_type: EdgeType, from: NodeId, to: NodeId) -> Self {
        Self {
            id: EdgeId::new(),
            edge_type,
            from,
            to,
            strength: 1.0,
        }
    }

    /// Create with explicit strength.
    pub fn with_strength(mut self, strength: f64) -> Self {
        self.strength = strength.clamp(0.0, 1.0);
        self
    }

    /// Returns true if this is a causal edge (part of the semiring).
    pub fn is_causal(&self) -> bool {
        matches!(self.edge_type, EdgeType::CausalLink { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_id_unique() {
        assert_ne!(EdgeId::new(), EdgeId::new());
    }

    #[test]
    fn causal_edge_detected() {
        let from = NodeId::new();
        let to = NodeId::new();
        let e = Edge::new(EdgeType::CausalLink { strength: 0.9 }, from, to);
        assert!(e.is_causal());
    }

    #[test]
    fn non_causal_edge() {
        let from = NodeId::new();
        let to = NodeId::new();
        let e = Edge::new(EdgeType::References, from, to);
        assert!(!e.is_causal());
    }

    #[test]
    fn strength_clamped() {
        let from = NodeId::new();
        let to = NodeId::new();
        let e = Edge::new(EdgeType::References, from, to).with_strength(5.0);
        assert_eq!(e.strength, 1.0);
    }
}
