//! Learning constants — all tunable parameters in one place.

/// Minimum observations before weight adjustment is proposed.
pub const MIN_OBSERVATIONS_FOR_ADJUSTMENT: usize = 5;

/// Accuracy threshold above which a mode is considered reliable.
pub const MODE_RELIABLE_THRESHOLD: f64 = 0.70;

/// Accuracy threshold below which a mode is flagged.
pub const MODE_FLAGGED_THRESHOLD: f64 = 0.40;

/// Maximum weight adjustment per cycle (prevents wild swings).
pub const MAX_WEIGHT_DELTA: f64 = 0.05;

/// Weight floor — no mode weight drops below this.
pub const MODE_WEIGHT_FLOOR: f64 = 0.02;

/// Weight ceiling — no mode weight exceeds this.
pub const MODE_WEIGHT_CEILING: f64 = 0.60;

/// Maximum learning records stored.
pub const MAX_LEARNING_RECORDS: usize = 10_000;

/// Decay factor — old observations weighted less over time.
pub const OBSERVATION_DECAY_FACTOR: f64 = 0.98;

/// Maximum number of observations stored per domain.
pub const MAX_HISTORY_PER_DOMAIN: usize = 1000;

/// Sliding window size for running accuracy computation.
pub const OBSERVATION_WINDOW_SIZE: usize = 50;
