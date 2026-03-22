//! Constants for the transform crate.
//!
//! All tunables live here — no magic numbers elsewhere.

/// Maximum registered format vocabularies.
pub const MAX_FORMAT_VOCABULARIES: usize = 1_000;

/// Maximum conversion chain depth (A->B->C counts as 2).
pub const MAX_CONVERSION_CHAIN_DEPTH: usize = 5;

/// Confidence floor for a conversion path.
pub const CONVERSION_CONFIDENCE_FLOOR: f64 = 0.5;
