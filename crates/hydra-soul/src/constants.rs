//! Constants for the soul orientation layer.
//!
//! All tunable parameters live here. No magic numbers elsewhere.

/// Minimum number of days before a deepening can be reconfirmed.
pub const DEEPENING_MIN_REFLECTION_DAYS: i64 = 30;

/// Number of top meaning nodes returned in the orientation vector.
pub const ORIENTATION_VECTOR_K: usize = 5;

/// Minimum exchanges before the soul is ready to speak.
pub const SOUL_MIN_EXCHANGES_TO_SPEAK: u64 = 1000;

/// Daily decay rate applied to node weights.
pub const NODE_WEIGHT_DECAY_PER_DAY: f64 = 0.0001;

/// Floor below which node weight never drops.
pub const NODE_WEIGHT_FLOOR: f64 = 0.001;

/// Weight multiplier for pressure-type reinforcement.
pub const WEIGHT_PRESSURE_MULTIPLIER: f64 = 3.0;

/// Weight multiplier for return-type reinforcement.
pub const WEIGHT_RETURN_MULTIPLIER: f64 = 2.0;

/// Maximum number of meaning nodes in the graph.
pub const MAX_MEANING_NODES: usize = 100_000;

/// Maximum number of deepening records stored.
pub const MAX_DEEPENING_RECORDS: usize = 1000;

/// Confidence threshold above which the soul is ready to orient.
pub const ORIENTATION_CONFIDENCE_THRESHOLD: f64 = 0.7;
