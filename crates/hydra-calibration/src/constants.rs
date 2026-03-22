/// Minimum records before a bias profile is meaningful.
pub const MIN_RECORDS_FOR_BIAS: usize = 10;

/// Minimum records before calibration is reported as reliable.
pub const MIN_RECORDS_FOR_RELIABLE_CALIBRATION: usize = 30;

/// Significant bias threshold — offsets above this are reported.
pub const SIGNIFICANT_BIAS_THRESHOLD: f64 = 0.05;

/// Perfect calibration: stated confidence matches actual accuracy.
pub const PERFECT_CALIBRATION_SCORE: f64 = 1.0;

/// Maximum stored calibration records.
pub const MAX_CALIBRATION_RECORDS: usize = 1_000_000;

/// Decay factor for older records — recent calibration matters more.
pub const RECENCY_DECAY_PER_MONTH: f64 = 0.03;

/// Maximum bias offset that can be applied in one adjustment.
pub const MAX_BIAS_CORRECTION: f64 = 0.30;
