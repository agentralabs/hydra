//! Constants for the genome crate.

/// Maximum number of genome entries stored.
pub const GENOME_MAX_ENTRIES: usize = 1_000_000;

/// Minimum confidence threshold for genome entries.
pub const GENOME_MIN_CONFIDENCE: f64 = 0.3;

/// Jaccard similarity threshold for situation matching.
/// Lowered from 0.7 to 0.15 to support indirect phrasings.
/// At 0.7, "Netflix failures spreading" vs "service failures cascading"
/// yields Jaccard=0.05 (no match). At 0.15, 2+ shared stems out of ~15
/// keywords is sufficient for retrieval. False positives are acceptable
/// because the LLM filters relevance; false negatives are not.
pub const SITUATION_SIMILARITY_THRESHOLD: f64 = 0.10;

/// Confidence boost applied per successful use.
pub const GENOME_CONFIDENCE_BOOST: f64 = 0.02;

/// Maximum keywords in a situation signature.
pub const SIGNATURE_MAX_KEYWORDS: usize = 32;

/// Number of top results returned by genome queries.
pub const GENOME_QUERY_TOP_N: usize = 5;

/// Weight of initial confidence in effective confidence blend.
pub const INITIAL_CONFIDENCE_WEIGHT: f64 = 0.4;

/// Weight of observed success rate in effective confidence blend.
pub const OBSERVED_CONFIDENCE_WEIGHT: f64 = 0.6;
