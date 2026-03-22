//! Constants for hydra-context.
//!
//! All tunable parameters live here. No magic numbers elsewhere.

/// Maximum items in any single context window.
pub const CONTEXT_WINDOW_MAX_ITEMS: usize = 50;

/// How many historical inputs to retain for the historical window.
pub const HISTORICAL_CONTEXT_DEPTH: usize = 20;

/// Minimum confidence for an anomaly to be included.
pub const ANOMALY_CONFIDENCE_THRESHOLD: f64 = 0.6;

/// Minimum significance for a gap signal to be included.
pub const GAP_SIGNIFICANCE_THRESHOLD: f64 = 0.5;

/// Time-to-live for a context window in seconds.
pub const CONTEXT_WINDOW_TTL_SECONDS: u64 = 30;

/// Maximum number of active gap signals tracked.
pub const MAX_ACTIVE_GAPS: usize = 10;

/// Maximum number of active anomaly signals tracked.
pub const MAX_ACTIVE_ANOMALIES: usize = 10;
