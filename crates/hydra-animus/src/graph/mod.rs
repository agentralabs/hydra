//! The Animus Prime semantic graph.
//! A PrimeGraph is the unit of communication between Hydra modules.

pub mod edge;
pub mod node;
pub mod proof;

pub use edge::{DataTransform, DependencyKind, Edge, EdgeId, EdgeType, TemporalRelation};
pub use node::{Node, NodeId, NodeType};
pub use proof::{Proof, ProofId, ProofStatus};

use crate::{
    constants::{PRIME_GRAPH_MAX_EDGES, PRIME_GRAPH_MAX_NODES},
    errors::AnimusError,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The core data structure of Animus Prime.
/// A complete semantic graph carrying meaning between Hydra modules.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PrimeGraph {
    nodes: HashMap<NodeId, Node>,
    edges: HashMap<EdgeId, Edge>,
    proofs: HashMap<ProofId, Proof>,
}

impl PrimeGraph {
    /// Create a new empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a node. Returns an error if the graph is at capacity.
    pub fn add_node(&mut self, node: Node) -> Result<NodeId, AnimusError> {
        if self.nodes.len() >= PRIME_GRAPH_MAX_NODES {
            return Err(AnimusError::GraphTooLarge {
                nodes: self.nodes.len(),
                edges: self.edges.len(),
                max_nodes: PRIME_GRAPH_MAX_NODES,
                max_edges: PRIME_GRAPH_MAX_EDGES,
            });
        }
        let id = node.id.clone();
        self.nodes.insert(id.clone(), node);
        Ok(id)
    }

    /// Add an edge. Validates that both endpoint nodes exist.
    pub fn add_edge(&mut self, edge: Edge) -> Result<EdgeId, AnimusError> {
        if self.edges.len() >= PRIME_GRAPH_MAX_EDGES {
            return Err(AnimusError::GraphTooLarge {
                nodes: self.nodes.len(),
                edges: self.edges.len(),
                max_nodes: PRIME_GRAPH_MAX_NODES,
                max_edges: PRIME_GRAPH_MAX_EDGES,
            });
        }

        if !self.nodes.contains_key(&edge.from) {
            return Err(AnimusError::UnknownNodeReference {
                edge_id: edge.id.to_string(),
                node_id: edge.from.to_string(),
            });
        }

        if !self.nodes.contains_key(&edge.to) {
            return Err(AnimusError::UnknownNodeReference {
                edge_id: edge.id.to_string(),
                node_id: edge.to.to_string(),
            });
        }

        let id = edge.id.clone();
        self.edges.insert(id.clone(), edge);
        Ok(id)
    }

    /// Add a proof to this graph.
    pub fn add_proof(&mut self, proof: Proof) -> ProofId {
        let id = proof.id.clone();
        self.proofs.insert(id.clone(), proof);
        id
    }

    /// Look up a node by ID.
    pub fn get_node(&self, id: &NodeId) -> Option<&Node> {
        self.nodes.get(id)
    }

    /// Look up an edge by ID.
    pub fn get_edge(&self, id: &EdgeId) -> Option<&Edge> {
        self.edges.get(id)
    }

    /// Total nodes in this graph.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Total edges in this graph.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Total proofs in this graph.
    pub fn proof_count(&self) -> usize {
        self.proofs.len()
    }

    /// True if this graph has no nodes.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// All edges that are causal links (part of the semiring).
    pub fn causal_edges(&self) -> impl Iterator<Item = &Edge> {
        self.edges.values().filter(|e| e.is_causal())
    }

    /// All nodes of a given type.
    pub fn nodes_of_type(&self, node_type: &NodeType) -> Vec<&Node> {
        self.nodes
            .values()
            .filter(|n| &n.node_type == node_type)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_graph_with_two_nodes() -> (PrimeGraph, NodeId, NodeId) {
        let mut g = PrimeGraph::new();
        let a = g
            .add_node(Node::new(NodeType::Intent, serde_json::json!("deploy")))
            .unwrap();
        let b = g
            .add_node(Node::new(NodeType::Receipt, serde_json::json!("done")))
            .unwrap();
        (g, a, b)
    }

    #[test]
    fn empty_graph() {
        let g = PrimeGraph::new();
        assert!(g.is_empty());
        assert_eq!(g.node_count(), 0);
    }

    #[test]
    fn add_node_and_edge() {
        let (mut g, a, b) = make_graph_with_two_nodes();
        assert_eq!(g.node_count(), 2);

        g.add_edge(Edge::new(EdgeType::CausalLink { strength: 0.9 }, a, b))
            .unwrap();
        assert_eq!(g.edge_count(), 1);
    }

    #[test]
    fn edge_with_unknown_node_rejected() {
        let mut g = PrimeGraph::new();
        let ghost = NodeId::from_str("ghost-node");
        let real = g
            .add_node(Node::new(NodeType::Intent, serde_json::Value::Null))
            .unwrap();
        let err = g.add_edge(Edge::new(EdgeType::References, ghost, real));
        assert!(matches!(err, Err(AnimusError::UnknownNodeReference { .. })));
    }

    #[test]
    fn causal_edges_filtered_correctly() {
        let (mut g, a, b) = make_graph_with_two_nodes();
        g.add_edge(Edge::new(
            EdgeType::CausalLink { strength: 0.8 },
            a.clone(),
            b.clone(),
        ))
        .unwrap();
        g.add_edge(Edge::new(EdgeType::References, a, b)).unwrap();
        let causal: Vec<_> = g.causal_edges().collect();
        assert_eq!(causal.len(), 1);
    }
}
