//! Temporal query engine — unified interface for all temporal queries.

use crate::btree::{ChronoSpatialBTree, ManifoldCoord, MemoryId, TemporalEntry};
use crate::causal_index::CausalChainIndex;
use crate::decision_graph::{ConstraintConflict, DecisionGraph};
use crate::errors::TemporalError;
use crate::timestamp::Timestamp;

/// The type of temporal query being performed.
#[derive(Debug, Clone)]
pub enum QueryType {
    /// Exact timestamp lookup.
    ExactTimestamp(Timestamp),
    /// Range scan between two timestamps.
    TimeRange(Timestamp, Timestamp),
    /// Most recent N entries.
    MostRecent(usize),
    /// Manifold proximity search.
    ManifoldProximity(ManifoldCoord, f64),
    /// Causal root search.
    CausalRoot(String),
    /// Check for decision conflicts.
    CheckConflicts(String),
}

/// Result of a temporal query.
#[derive(Debug)]
pub enum QueryResult<'a> {
    /// A single entry found by exact lookup.
    Single(Option<&'a TemporalEntry>),
    /// Multiple entries from range/spatial/recent queries.
    Multiple(Vec<&'a TemporalEntry>),
    /// Memory IDs from causal root lookup.
    MemoryIds(Vec<&'a MemoryId>),
    /// Conflicts from a conflict check.
    Conflicts(Vec<ConstraintConflict>),
}

/// Unified temporal query engine.
///
/// Wraps the B+ tree, causal index, and decision graph to provide
/// a single entry point for all temporal queries.
pub struct TemporalQueryEngine {
    /// The chrono-spatial B+ tree.
    pub btree: ChronoSpatialBTree,
    /// The causal chain index.
    pub causal_index: CausalChainIndex,
    /// The decision constraint graph.
    pub decision_graph: DecisionGraph,
}

impl Default for TemporalQueryEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TemporalQueryEngine {
    /// Create a new, empty query engine.
    pub fn new() -> Self {
        Self {
            btree: ChronoSpatialBTree::new(),
            causal_index: CausalChainIndex::new(),
            decision_graph: DecisionGraph::new(),
        }
    }

    /// Look up an entry by exact timestamp.
    pub fn exact_timestamp(&self, ts: &Timestamp) -> Option<&TemporalEntry> {
        self.btree.get_exact(ts)
    }

    /// Scan entries in a time range (inclusive).
    pub fn time_range(
        &self,
        from: &Timestamp,
        to: &Timestamp,
    ) -> Result<Vec<&TemporalEntry>, TemporalError> {
        self.btree.range_scan(from, to)
    }

    /// Return the N most recent entries.
    pub fn most_recent(&self, n: usize) -> Vec<&TemporalEntry> {
        self.btree.most_recent(n)
    }

    /// Find entries near a manifold coordinate.
    pub fn manifold_proximity(&self, center: &ManifoldCoord, radius: f64) -> Vec<&TemporalEntry> {
        self.btree.spatial_range(center, radius)
    }

    /// Find all memory IDs associated with a causal root.
    pub fn causal_root(&self, root: &str) -> Vec<&MemoryId> {
        self.causal_index.memories_for_root(root)
    }

    /// Check for decision conflicts against a proposed action.
    pub fn check_conflicts(
        &self,
        proposed_action: &str,
        elapsed_seconds: f64,
    ) -> Vec<ConstraintConflict> {
        self.decision_graph
            .check_conflicts(proposed_action, elapsed_seconds)
    }

    /// Execute a typed query.
    pub fn query(&self, qt: &QueryType) -> Result<QueryResult<'_>, TemporalError> {
        match qt {
            QueryType::ExactTimestamp(ts) => Ok(QueryResult::Single(self.exact_timestamp(ts))),
            QueryType::TimeRange(from, to) => Ok(QueryResult::Multiple(self.time_range(from, to)?)),
            QueryType::MostRecent(n) => Ok(QueryResult::Multiple(self.most_recent(*n))),
            QueryType::ManifoldProximity(center, radius) => Ok(QueryResult::Multiple(
                self.manifold_proximity(center, *radius),
            )),
            QueryType::CausalRoot(root) => Ok(QueryResult::MemoryIds(self.causal_root(root))),
            QueryType::CheckConflicts(action) => {
                Ok(QueryResult::Conflicts(self.check_conflicts(action, 0.0)))
            }
        }
    }
}
