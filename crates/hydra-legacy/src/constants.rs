/// Minimum days of accumulated wisdom before a legacy artifact can be created.
pub const MIN_DAYS_FOR_LEGACY: u32 = 365;

/// Maximum number of legacy artifacts in the archive.
pub const MAX_LEGACY_ARTIFACTS: usize = 10_000;

/// Maximum characters allowed in a single legacy artifact's content.
pub const MAX_LEGACY_CONTENT_CHARS: usize = 100_000;

/// Minimum confidence score for a legacy artifact to be accepted.
pub const LEGACY_CONFIDENCE_FLOOR: f64 = 0.70;
