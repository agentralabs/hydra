//! Constants for the noticing engine.
//!
//! All tunables live here — no magic numbers in library code.

/// Drift threshold before generating a noticing signal (fraction).
pub const DRIFT_THRESHOLD_FRACTION: f64 = 0.10;

/// Minimum samples before drift is meaningful.
pub const DRIFT_MIN_SAMPLES: usize = 3;

/// Trend window — how many samples define a trend.
pub const TREND_WINDOW_SIZE: usize = 5;

/// Compound risk threshold — how many small issues trigger compounding.
pub const COMPOUND_RISK_THRESHOLD: usize = 3;

/// Pattern break threshold — how many days before a missing pattern is noticed.
pub const PATTERN_BREAK_DAYS: u64 = 7;

/// Maximum noticing signals queued at once.
pub const MAX_QUEUED_SIGNALS: usize = 100;

/// Noticing interval — how often the engine samples (seconds).
/// In tests: configurable. In production: 300 seconds (5 minutes).
pub const NOTICING_INTERVAL_SECONDS: u64 = 300;

/// Minimum significance for a noticing signal to surface.
pub const SIGNAL_SIGNIFICANCE_FLOOR: f64 = 0.40;

/// Trend significance multiplier — sustained trends are more significant.
pub const TREND_SIGNIFICANCE_MULTIPLIER: f64 = 1.5;
