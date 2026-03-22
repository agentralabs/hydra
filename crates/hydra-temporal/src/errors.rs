//! Error types for hydra-temporal.

use thiserror::Error;

/// All errors that hydra-temporal can produce.
#[derive(Debug, Error)]
pub enum TemporalError {
    /// A timestamp already exists in the append-only B+ tree.
    #[error("duplicate timestamp: {0}")]
    DuplicateTimestamp(u64),

    /// A timestamp of zero or otherwise invalid value was provided.
    #[error("invalid timestamp: {0}")]
    InvalidTimestamp(String),

    /// A cycle was detected in the constraint/decision DAG.
    #[error("constraint cycle detected: {0}")]
    ConstraintCycleDetected(String),

    /// A decision ID was not found in the graph.
    #[error("decision not found: {0}")]
    DecisionNotFound(String),

    /// DFS traversal exceeded the maximum allowed depth.
    #[error("graph depth exceeded maximum of {0}")]
    GraphDepthExceeded(usize),

    /// A range query spans more than the allowed number of days.
    #[error("range query too large: {days} days exceeds maximum of {max_days}")]
    RangeQueryTooLarge {
        /// Requested span in days.
        days: u64,
        /// Maximum allowed span.
        max_days: u64,
    },

    /// A spatial partition cell has reached its capacity.
    #[error("spatial partition full at cell ({x}, {y}, {z})")]
    SpatialPartitionFull {
        /// Grid x coordinate.
        x: usize,
        /// Grid y coordinate.
        y: usize,
        /// Grid z coordinate.
        z: usize,
    },

    /// A query returned no results.
    #[error("empty result for query: {0}")]
    EmptyResult(String),
}
