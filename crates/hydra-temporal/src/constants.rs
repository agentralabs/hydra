//! All temporal constants for hydra-temporal.
//!
//! No magic numbers anywhere else — every configurable value lives here.

// ── B+ Tree ──────────────────────────────────────────────────────────

/// Maximum entries per B+ tree node before splitting.
pub const BTREE_NODE_CAPACITY: usize = 64;

/// When total entries exceed this, the tree emits a compaction hint.
pub const BTREE_MAX_ENTRIES_BEFORE_HINT: u64 = 10_000_000;

// ── Nanosecond Helpers ───────────────────────────────────────────────

/// Nanoseconds in one second.
pub const NANOS_PER_SECOND: u64 = 1_000_000_000;

/// Nanoseconds in one millisecond.
pub const NANOS_PER_MS: u64 = 1_000_000;

/// Nanoseconds in one microsecond.
pub const NANOS_PER_US: u64 = 1_000;

// ── Constraint Decay ─────────────────────────────────────────────────

/// Lambda for exponential decay: S(t) = S0 * e^(-lambda * t).
/// Chosen so a constraint with S0=1.0 reaches the floor after ~90 days.
pub const CONSTRAINT_DECAY_LAMBDA: f64 = 2.67e-7;

/// Minimum strength — a constraint never decays below this.
pub const CONSTRAINT_DECAY_FLOOR: f64 = 0.001;

// ── Decision Graph ───────────────────────────────────────────────────

/// Maximum depth for DFS traversal in the decision graph.
pub const DECISION_GRAPH_MAX_DEPTH: usize = 1000;

// ── Spatial Partition ────────────────────────────────────────────────

/// Grid dimension for the spatial partition index (N x N x N).
pub const SPATIAL_PARTITION_GRID_SIZE: usize = 32;

/// Maximum memories stored in a single grid cell.
pub const SPATIAL_PARTITION_MAX_PER_CELL: usize = 100_000;

// ── Causal Index ─────────────────────────────────────────────────────

/// Maximum number of distinct causal roots the index tracks.
pub const CAUSAL_INDEX_MAX_ROOTS: usize = 1_000_000;

// ── Query Engine ─────────────────────────────────────────────────────

/// Number of most-recent entries cached for O(1) retrieval.
pub const RECENT_CACHE_SIZE: usize = 10_000;

/// Target latency for a single query in nanoseconds (50 ms).
pub const QUERY_LATENCY_TARGET_NS: u64 = 50_000_000;

/// Maximum span of a range query in days.
pub const RANGE_QUERY_MAX_DAYS: u64 = 365;
