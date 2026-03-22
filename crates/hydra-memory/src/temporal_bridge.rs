//! Bridge between AgenticMemory's TemporalIndex and hydra-temporal's B+ tree.
//!
//! AgenticMemory has its own temporal index built in.
//! hydra-temporal has the Chrono-Spatial B+ tree.
//! This bridge keeps both in sync so queries can use either.
//!
//! The B+ tree is the primary index for latency-critical queries.
//! AgenticMemory's index is the durable store for long-term retrieval.

use crate::errors::MemoryError;
use hydra_temporal::{
    btree::{ChronoSpatialBTree, ManifoldCoord, MemoryId, TemporalEntry},
    causal_index::CausalChainIndex,
    spatial::SpatialPartitionIndex,
    timestamp::Timestamp,
};

/// The temporal bridge — keeps hydra-temporal and AgenticMemory in sync.
pub struct TemporalBridge {
    /// The primary B+ tree index (fast queries).
    pub btree: ChronoSpatialBTree,
    /// The spatial partition index (manifold queries).
    pub spatial: SpatialPartitionIndex,
    /// The causal chain index (causal queries).
    pub causal: CausalChainIndex,
}

impl TemporalBridge {
    /// Create a new temporal bridge with empty indices.
    pub fn new() -> Self {
        Self {
            btree: ChronoSpatialBTree::new(),
            spatial: SpatialPartitionIndex::new(),
            causal: CausalChainIndex::new(),
        }
    }

    /// Index a new memory event across all three indices.
    /// Called after every successful AgenticMemory write.
    pub fn index(
        &mut self,
        memory_id: &str,
        ts: Timestamp,
        manifold: ManifoldCoord,
        causal_root: &str,
        _session_id: &str,
    ) -> Result<(), MemoryError> {
        let mem_id = MemoryId::from_value(memory_id);

        // Index in the B+ tree
        let entry = TemporalEntry {
            timestamp: ts,
            memory_id: mem_id.clone(),
            coord: manifold,
            causal_root: Some(causal_root.to_string()),
        };

        self.btree
            .insert(entry)
            .map_err(|e| MemoryError::WriteError {
                reason: format!("B+ tree insert failed: {}", e),
            })?;

        // Index in spatial partition
        self.spatial
            .insert(mem_id.clone(), &manifold)
            .map_err(|e| MemoryError::WriteError {
                reason: format!("Spatial index insert failed: {}", e),
            })?;

        // Index in causal chain
        let inserted = self.causal.insert(causal_root.to_string(), mem_id);
        if !inserted {
            return Err(MemoryError::WriteError {
                reason: "Causal index insert failed: max roots reached".to_string(),
            });
        }

        Ok(())
    }

    /// Look up a memory by exact timestamp. O(log n).
    pub fn get_exact(&self, ts: &Timestamp) -> Option<&TemporalEntry> {
        self.btree.get_exact(ts)
    }

    /// Get the most recent N memories. O(1).
    pub fn most_recent(&self, n: usize) -> Vec<&TemporalEntry> {
        self.btree.most_recent(n)
    }

    /// Range scan. O(k log n).
    pub fn range_scan(
        &self,
        start: &Timestamp,
        end: &Timestamp,
    ) -> Result<Vec<&TemporalEntry>, MemoryError> {
        self.btree
            .range_scan(start, end)
            .map_err(|e| MemoryError::QueryError {
                reason: format!("Range scan failed: {}", e),
            })
    }

    /// Find memories by causal root.
    pub fn by_causal_root(&self, root: &str) -> Vec<&MemoryId> {
        self.causal.memories_for_root(root)
    }

    /// Total indexed memories.
    pub fn total_indexed(&self) -> u64 {
        self.btree.len() as u64
    }
}

impl Default for TemporalBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(n: u64) -> Timestamp {
        Timestamp::from_nanos(n).expect("valid nanos")
    }

    #[test]
    fn index_and_retrieve() {
        let mut bridge = TemporalBridge::new();
        bridge
            .index(
                "mem-001",
                ts(1_000_000_000),
                ManifoldCoord::new(0.1, 0.1, 0.0),
                "const-identity",
                "session-001",
            )
            .expect("should index");

        assert!(bridge.get_exact(&ts(1_000_000_000)).is_some());
        assert_eq!(bridge.total_indexed(), 1);
    }

    #[test]
    fn most_recent_returns_latest() {
        let mut bridge = TemporalBridge::new();
        for i in 1..=5u64 {
            bridge
                .index(
                    &format!("mem-{}", i),
                    ts(i * 1_000_000_000),
                    ManifoldCoord::new(0.0, 0.0, 0.0),
                    "const-identity",
                    "session-001",
                )
                .expect("should index");
        }
        let recent = bridge.most_recent(3);
        assert_eq!(recent.len(), 3);
    }

    #[test]
    fn causal_root_lookup() {
        let mut bridge = TemporalBridge::new();
        bridge
            .index(
                "mem-001",
                ts(1_000_000_000),
                ManifoldCoord::new(0.0, 0.0, 0.0),
                "decision-xyz",
                "session-001",
            )
            .expect("should index");
        bridge
            .index(
                "mem-002",
                ts(2_000_000_000),
                ManifoldCoord::new(0.0, 0.0, 0.0),
                "decision-xyz",
                "session-001",
            )
            .expect("should index");

        let results = bridge.by_causal_root("decision-xyz");
        assert_eq!(results.len(), 2);
    }
}
