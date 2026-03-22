/// Minimum records needed before crystallizing a playbook.
pub const MIN_RECORDS_FOR_PLAYBOOK: usize = 5;

/// Minimum records for a post-mortem.
pub const MIN_RECORDS_FOR_POSTMORTEM: usize = 3;

/// Maximum artifact content length (chars).
pub const MAX_ARTIFACT_CONTENT_CHARS: usize = 50_000;

/// Maximum stored crystallized artifacts.
pub const MAX_STORED_ARTIFACTS: usize = 10_000;

/// Confidence floor for including a pattern in an artifact.
pub const ARTIFACT_PATTERN_CONFIDENCE: f64 = 0.65;
