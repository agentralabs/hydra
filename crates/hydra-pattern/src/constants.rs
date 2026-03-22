/// Minimum pattern similarity to trigger a match (0.0-1.0).
pub const PATTERN_MATCH_THRESHOLD: f64 = 0.65;

/// Maximum patterns in the library.
pub const MAX_PATTERN_LIBRARY_SIZE: usize = 100_000;

/// Anti-pattern warning confidence floor.
pub const ANTIPATTERN_WARNING_THRESHOLD: f64 = 0.70;

/// Minimum observations for a pattern to be considered proven.
pub const MIN_PROVEN_OBSERVATIONS: usize = 3;

/// Pattern decay factor -- older patterns weighted slightly less.
pub const PATTERN_DECAY_PER_YEAR: f64 = 0.02;
