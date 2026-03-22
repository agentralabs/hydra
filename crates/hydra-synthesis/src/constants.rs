//! Synthesis constants — all tunable parameters in one place.

/// Minimum structural similarity to trigger a synthesis insight.
pub const SYNTHESIS_SIMILARITY_THRESHOLD: f64 = 0.60;

/// Maximum synthesis insights returned per cycle.
pub const MAX_SYNTHESIS_INSIGHTS: usize = 3;

/// Minimum domains in reasoning history to attempt cross-domain synthesis.
pub const MIN_DOMAINS_FOR_CROSS_SYNTHESIS: usize = 2;

/// Genome similarity threshold for pattern transfer.
pub const GENOME_PATTERN_SIMILARITY: f64 = 0.55;

/// Confidence floor for a synthesis insight.
pub const SYNTHESIS_CONFIDENCE_FLOOR: f64 = 0.45;

/// Maximum number of structural patterns stored in the library.
pub const MAX_PATTERN_LIBRARY: usize = 10_000;
