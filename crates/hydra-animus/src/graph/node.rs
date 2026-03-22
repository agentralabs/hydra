//! Node types for the Animus Prime semantic graph.
//! Nodes are semantic entities — the "nouns" of the Prime language.

use crate::errors::AnimusError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a node in a Prime graph.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(String);

impl NodeId {
    /// Generate a new unique node ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Create from an existing string (for deserialization).
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Returns the string representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The type of a node in the Prime graph.
/// Base types are always available. Domain types are registered by skills.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeType {
    // -- Base vocabulary (never removed) --
    /// A concrete value (integer, float, string, bool, null).
    Value,
    /// A type definition or type annotation.
    TypeDef,
    /// A function or callable unit of logic.
    Function,
    /// A soul — the identity and purpose of a module or agent.
    Soul,
    /// An expression node (computation, transformation).
    Expression,
    /// A statement node (action, assignment, control flow).
    Statement,
    /// A belief — a proposition Hydra holds about the world.
    Belief,
    /// A trust relationship between entities.
    Trust,
    /// A signal in the causal semiring.
    Signal,
    /// An immutable receipt of an action.
    Receipt,
    /// An agent in the fleet.
    Agent,
    /// A skill loaded into the skill substrate.
    Skill,
    /// A domain persona.
    Persona,
    /// An intent expressed by the principal.
    Intent,
    /// A causal chain node (ancestor in composition).
    CausalAncestor,
    // -- Domain vocabulary (registered by skills at runtime) --
    /// A domain-specific node type registered by a skill.
    Domain {
        /// Which domain registered this type.
        domain: String,
        /// The type name within that domain.
        name: String,
    },
}

impl NodeType {
    /// Returns true if this is a base vocabulary type.
    pub fn is_base(&self) -> bool {
        !matches!(self, Self::Domain { .. })
    }

    /// Returns true if this is a domain-registered type.
    pub fn is_domain(&self) -> bool {
        matches!(self, Self::Domain { .. })
    }

    /// Returns the domain name if this is a domain type.
    pub fn domain_name(&self) -> Option<&str> {
        match self {
            Self::Domain { domain, .. } => Some(domain.as_str()),
            _ => None,
        }
    }
}

impl std::fmt::Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Domain { domain, name } => write!(f, "{}::{}", domain, name),
            other => write!(f, "{:?}", other),
        }
    }
}

/// A node in the Animus Prime semantic graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    /// Unique identifier.
    pub id: NodeId,

    /// What kind of semantic entity this is.
    pub node_type: NodeType,

    /// The content of this node, serialized as JSON.
    /// Kept as raw JSON to support arbitrary domain-specific payloads.
    pub content: serde_json::Value,

    /// Confidence in this node's content (0.0 = no confidence, 1.0 = certain).
    pub confidence: f64,

    /// When this node was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Node {
    /// Create a new node with the given type and content.
    pub fn new(node_type: NodeType, content: serde_json::Value) -> Self {
        Self {
            id: NodeId::new(),
            node_type,
            content,
            confidence: 1.0,
            created_at: chrono::Utc::now(),
        }
    }

    /// Create with explicit confidence.
    pub fn with_confidence(mut self, confidence: f64) -> Result<Self, AnimusError> {
        if !(0.0..=1.0).contains(&confidence) {
            return Err(AnimusError::InvalidSignalWeight {
                weight: confidence,
                min: 0.0,
                max: 1.0,
            });
        }
        self.confidence = confidence;
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_id_is_unique() {
        assert_ne!(NodeId::new(), NodeId::new());
    }

    #[test]
    fn base_types_are_base() {
        assert!(NodeType::Belief.is_base());
        assert!(NodeType::Trust.is_base());
        assert!(!NodeType::Domain {
            domain: "finance".into(),
            name: "Trade".into()
        }
        .is_base());
    }

    #[test]
    fn domain_type_reports_domain() {
        let t = NodeType::Domain {
            domain: "finance".into(),
            name: "Position".into(),
        };
        assert_eq!(t.domain_name(), Some("finance"));
    }

    #[test]
    fn node_creation() {
        let n = Node::new(NodeType::Belief, serde_json::json!({"fact": "test"}));
        assert_eq!(n.confidence, 1.0);
        assert!(matches!(n.node_type, NodeType::Belief));
    }

    #[test]
    fn invalid_confidence_rejected() {
        let n = Node::new(NodeType::Belief, serde_json::Value::Null);
        assert!(n.clone().with_confidence(1.5).is_err());
        assert!(n.with_confidence(-0.1).is_err());
    }
}
