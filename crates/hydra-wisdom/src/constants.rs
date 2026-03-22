/// Minimum confidence to issue a positive recommendation.
pub const MIN_RECOMMENDATION_CONFIDENCE: f64 = 0.50;

/// Threshold at which a wisdom statement is flagged as uncertain.
pub const UNCERTAINTY_FLAG_THRESHOLD: f64 = 0.60;

/// Maximum wisdom memories stored.
pub const MAX_WISDOM_MEMORIES: usize = 100_000;

/// Similarity threshold for memory recall.
pub const MEMORY_RECALL_THRESHOLD: f64 = 0.65;

/// Weight of pattern evidence in synthesis.
pub const WEIGHT_PATTERN: f64 = 0.25;
/// Weight of oracle scenarios in synthesis.
pub const WEIGHT_ORACLE: f64 = 0.25;
/// Weight of red team findings in synthesis.
pub const WEIGHT_REDTEAM: f64 = 0.20;
/// Weight of calibration adjustment in synthesis.
pub const WEIGHT_CALIBRATION: f64 = 0.15;
/// Weight of learning history in synthesis.
pub const WEIGHT_LEARNING: f64 = 0.15;
