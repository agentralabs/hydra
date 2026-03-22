//! Constants for the prediction crate.

/// Number of prediction slots maintained at any time.
pub const PREDICTION_SLOT_COUNT: usize = 3;

/// Minimum confidence for a prediction to be considered.
pub const PREDICTION_MIN_CONFIDENCE: f64 = 0.2;

/// Number of recent intents to consider for pattern detection.
pub const PATTERN_WINDOW_SIZE: usize = 20;

/// Maximum acceptable divergence before triggering belief revision.
pub const DIVERGENCE_THRESHOLD: f64 = 0.4;

/// Timeout in milliseconds for shadow execution.
pub const SHADOW_EXECUTION_TIMEOUT_MS: u64 = 500;
