//! Constants for the belief crate.

/// Minimum confidence a belief can hold.
pub const BELIEF_CONFIDENCE_MIN: f64 = 0.0;

/// Maximum confidence a belief can hold.
pub const BELIEF_CONFIDENCE_MAX: f64 = 1.0;

/// Default revision strength when no explicit strength is provided.
pub const REVISION_STRENGTH: f64 = 0.3;

/// Maximum number of beliefs in a belief store.
pub const BELIEF_SET_MAX_SIZE: usize = 100_000;

/// Rate at which beliefs decay per day without reinforcement.
pub const BELIEF_DECAY_RATE_PER_DAY: f64 = 0.001;

/// Default geodesic step size for manifold traversal.
pub const GEODESIC_STEP_SIZE: f64 = 0.1;
