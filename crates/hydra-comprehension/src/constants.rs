//! Constants for the comprehension engine.
//!
//! All tunable parameters live here. No magic numbers elsewhere.

/// Minimum number of tokens (whitespace-separated words) for an input
/// to be considered substantial enough for full comprehension.
pub const MIN_SUBSTANTIAL_TOKEN_COUNT: usize = 3;

/// Maximum number of domains detected per input.
pub const MAX_DETECTED_DOMAINS: usize = 4;

/// Jaccard similarity threshold for genome resonance matching.
/// Lowered from 0.65 to 0.10 to match genome store's threshold.
/// The genome store now uses IDF-weighted scoring; the resonance
/// re-check should not filter out entries that passed IDF scoring.
pub const RESONANCE_SIMILARITY_THRESHOLD: f64 = 0.10;

/// Maximum number of resonance matches to return.
pub const RESONANCE_TOP_N: usize = 3;

/// Maximum number of axiom primitives extracted per input.
pub const MAX_PRIMITIVES_PER_INPUT: usize = 8;

/// Minimum overall confidence for a comprehension result to be accepted.
pub const COMPREHENSION_CONFIDENCE_THRESHOLD: f64 = 0.7;

/// LLM fallback token budget for input (future phase).
pub const LLM_FALLBACK_BUDGET_IN: usize = 1000;

/// LLM fallback token budget for output (future phase).
pub const LLM_FALLBACK_BUDGET_OUT: usize = 500;

/// Minimum fraction of vocabulary words that must match for a domain to register.
pub const DOMAIN_VOCAB_MATCH_THRESHOLD: f64 = 0.15;
