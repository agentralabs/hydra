//! Topology map — bidirectional neighbor relationships.

use crate::constants::MAX_TOPOLOGY_NEIGHBORS;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A bidirectional topology map of system neighbors.
///
/// Each system can have at most `MAX_TOPOLOGY_NEIGHBORS` neighbors.
/// Neighbors are stored with their similarity scores.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TopologyMap {
    /// Adjacency map: system name → sorted neighbors (name, similarity).
    adjacency: BTreeMap<String, Vec<(String, f64)>>,
}

impl TopologyMap {
    /// Create an empty topology map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a bidirectional neighbor relationship.
    ///
    /// Only adds if the similarity is above the threshold. Maintains
    /// the top-N neighbors per system (sorted by descending similarity).
    pub fn add_neighbor(&mut self, a: &str, b: &str, similarity: f64, threshold: f64) {
        if similarity < threshold || a == b {
            return;
        }
        Self::insert_neighbor(&mut self.adjacency, a, b, similarity);
        Self::insert_neighbor(&mut self.adjacency, b, a, similarity);
    }

    /// Get the neighbors of a system, sorted by descending similarity.
    pub fn neighbors(&self, name: &str) -> &[(String, f64)] {
        self.adjacency.get(name).map_or(&[], |v| v.as_slice())
    }

    /// Insert a neighbor into the adjacency list, maintaining sorted order.
    fn insert_neighbor(
        adjacency: &mut BTreeMap<String, Vec<(String, f64)>>,
        from: &str,
        to: &str,
        similarity: f64,
    ) {
        let neighbors = adjacency.entry(from.to_string()).or_default();

        // Remove existing entry for this neighbor if present.
        neighbors.retain(|(name, _)| name != to);

        neighbors.push((to.to_string(), similarity));
        neighbors.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        neighbors.truncate(MAX_TOPOLOGY_NEIGHBORS);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_neighbor_bidirectional() {
        let mut topo = TopologyMap::new();
        topo.add_neighbor("api-a", "api-b", 0.8, 0.5);
        assert_eq!(topo.neighbors("api-a").len(), 1);
        assert_eq!(topo.neighbors("api-b").len(), 1);
    }

    #[test]
    fn below_threshold_rejected() {
        let mut topo = TopologyMap::new();
        topo.add_neighbor("api-a", "api-b", 0.3, 0.5);
        assert!(topo.neighbors("api-a").is_empty());
    }

    #[test]
    fn self_loop_rejected() {
        let mut topo = TopologyMap::new();
        topo.add_neighbor("api-a", "api-a", 1.0, 0.5);
        assert!(topo.neighbors("api-a").is_empty());
    }

    #[test]
    fn neighbors_sorted_by_similarity() {
        let mut topo = TopologyMap::new();
        topo.add_neighbor("api-a", "api-b", 0.8, 0.5);
        topo.add_neighbor("api-a", "api-c", 0.9, 0.5);
        let n = topo.neighbors("api-a");
        assert_eq!(n[0].0, "api-c");
        assert_eq!(n[1].0, "api-b");
    }
}
