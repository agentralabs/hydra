//! The meaning graph — an append-only structure of accumulated meaning.

use std::collections::HashMap;

use uuid::Uuid;

use crate::constants::{
    MAX_MEANING_NODES, ORIENTATION_CONFIDENCE_THRESHOLD, ORIENTATION_VECTOR_K,
    SOUL_MIN_EXCHANGES_TO_SPEAK,
};
use crate::errors::SoulError;
use crate::node::{MeaningNode, NodeKind};

/// The meaning graph. Append-only — no deletes, no resets.
///
/// The single write path is `record_exchange()`.
#[derive(Debug, Clone)]
pub struct MeaningGraph {
    /// All meaning nodes, keyed by UUID.
    pub(crate) nodes: HashMap<String, MeaningNode>,
    /// Index from label to node ID for fast lookup.
    label_index: HashMap<String, String>,
    /// Total exchanges recorded (monotonically increasing).
    exchange_count: u64,
}

impl MeaningGraph {
    /// Create a new, empty meaning graph.
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            label_index: HashMap::new(),
            exchange_count: 0,
        }
    }

    /// Record an exchange. This is the single write path into the graph.
    ///
    /// If a node with the given label already exists, it is reinforced.
    /// Otherwise a new node is created (if capacity allows).
    pub fn record_exchange(&mut self, label: &str, kind: NodeKind) -> Result<(), SoulError> {
        self.exchange_count += 1;

        if let Some(id) = self.label_index.get(label) {
            if let Some(node) = self.nodes.get_mut(id) {
                node.reinforce();
            }
            return Ok(());
        }

        // New node — check capacity
        if self.nodes.len() >= MAX_MEANING_NODES {
            self.prune_lowest_weight();
        }
        if self.nodes.len() >= MAX_MEANING_NODES {
            return Err(SoulError::GraphAtCapacity(self.nodes.len()));
        }

        let id = Uuid::new_v4().to_string();
        let node = MeaningNode::new(label, kind);
        self.label_index.insert(label.to_string(), id.clone());
        self.nodes.insert(id, node);
        Ok(())
    }

    /// Return the top-K nodes by weight (the orientation vector).
    pub fn orientation_vector(&self) -> Vec<&MeaningNode> {
        let mut sorted: Vec<&MeaningNode> = self.nodes.values().collect();
        sorted.sort_by(|a, b| {
            b.weight
                .partial_cmp(&a.weight)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted.truncate(ORIENTATION_VECTOR_K);
        sorted
    }

    /// Compute orientation confidence based on exchange count.
    ///
    /// Grows logarithmically toward 1.0 as exchanges accumulate.
    pub fn orientation_confidence(&self) -> f64 {
        if self.exchange_count == 0 {
            return 0.0;
        }
        let raw = (self.exchange_count as f64).ln() / (SOUL_MIN_EXCHANGES_TO_SPEAK as f64).ln();
        raw.min(1.0)
    }

    /// Returns true if confidence exceeds the threshold.
    pub fn is_ready_to_speak(&self) -> bool {
        self.orientation_confidence() >= ORIENTATION_CONFIDENCE_THRESHOLD
    }

    /// Apply time-based decay to all nodes.
    pub fn decay_all(&mut self, days: f64) {
        for node in self.nodes.values_mut() {
            node.decay(days);
        }
    }

    /// Remove the lowest-weight node to make room for new ones.
    fn prune_lowest_weight(&mut self) {
        let min_id = self
            .nodes
            .iter()
            .min_by(|a, b| {
                a.1.weight
                    .partial_cmp(&b.1.weight)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(id, _)| id.clone());

        if let Some(id) = min_id {
            if let Some(node) = self.nodes.remove(&id) {
                self.label_index.remove(&node.label);
            }
        }
    }

    /// Total number of exchanges recorded.
    pub fn exchange_count(&self) -> u64 {
        self.exchange_count
    }

    /// Total number of nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Read-only access to all nodes (for inspection/testing).
    pub fn all_nodes(&self) -> impl Iterator<Item = &MeaningNode> {
        self.nodes.values()
    }
}

impl Default for MeaningGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_graph_not_ready() {
        let g = MeaningGraph::new();
        assert!(!g.is_ready_to_speak());
        assert_eq!(g.orientation_confidence(), 0.0);
    }

    #[test]
    fn record_and_retrieve() {
        let mut g = MeaningGraph::new();
        g.record_exchange("reliability", NodeKind::RecurringChoice)
            .expect("record");
        assert_eq!(g.node_count(), 1);
        assert_eq!(g.exchange_count(), 1);
    }

    #[test]
    fn reinforcement_on_duplicate_label() {
        let mut g = MeaningGraph::new();
        g.record_exchange("care", NodeKind::RecurringReturn)
            .expect("first");
        g.record_exchange("care", NodeKind::RecurringReturn)
            .expect("second");
        assert_eq!(g.node_count(), 1);
        assert_eq!(g.exchange_count(), 2);
        let node = g.nodes.values().next().expect("one node");
        assert_eq!(node.reinforcement_count, 2);
    }
}
