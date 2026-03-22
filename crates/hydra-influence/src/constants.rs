/// Minimum evidence count required to publish a pattern.
pub const MIN_EVIDENCE_FOR_PUBLICATION: usize = 5;

/// Minimum confidence required to publish a pattern.
pub const MIN_CONFIDENCE_FOR_PUBLICATION: f64 = 0.70;

/// Minimum confidence required to adopt a pattern.
pub const MIN_CONFIDENCE_FOR_ADOPTION: f64 = 0.65;

/// Maximum number of published patterns in the registry.
pub const MAX_PUBLISHED_PATTERNS: usize = 100_000;

/// Maximum number of adoption records.
pub const MAX_ADOPTION_RECORDS: usize = 1_000_000;

/// Confidence increment per confirmed outcome.
pub const OUTCOME_CONFIDENCE_INCREMENT: f64 = 0.005;

/// Maximum confidence a pattern can reach.
pub const MAX_PATTERN_CONFIDENCE: f64 = 0.99;
