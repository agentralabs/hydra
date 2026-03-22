/// How many times the same pattern must fire before proposal.
pub const CRYSTALLIZATION_THRESHOLD: usize = 3;

/// Maximum observations kept before pruning oldest.
pub const MAX_OBSERVATIONS: usize = 10_000;

/// Maximum pending proposals.
pub const MAX_PENDING_PROPOSALS: usize = 100;

/// Observation window for pattern matching (days).
pub const PATTERN_WINDOW_DAYS: i64 = 30;

/// Minimum parameter consistency for crystallization (fraction).
/// 0.8 = 80% of calls use the same params — worth crystallizing.
pub const MIN_PARAM_CONSISTENCY: f64 = 0.5;

/// Generated skill output directory (relative to Hydra root).
pub const SKILL_OUTPUT_DIR: &str = "skills/generated";
