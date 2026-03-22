//! Reasoning constants — all tunables in one place.
//!
//! No magic numbers elsewhere. Every threshold lives here.

/// Minimum confidence for deductive conclusions.
pub const DEDUCTIVE_MIN_CONFIDENCE: f64 = 0.3;

/// Minimum number of genome occurrences for inductive reasoning.
pub const INDUCTIVE_MIN_OCCURRENCES: usize = 2;

/// Confidence threshold below which abductive reasoning flags LLM need.
pub const ABDUCTIVE_LLM_THRESHOLD: f64 = 0.5;

/// Minimum similarity for analogical cross-domain matching.
pub const ANALOGICAL_MIN_SIMILARITY: f64 = 0.6;

/// Minimum confidence for adversarial threat conclusions.
pub const ADVERSARIAL_MIN_THREAT_CONFIDENCE: f64 = 0.4;

/// Weight for deductive reasoning in synthesis.
pub const WEIGHT_DEDUCTIVE: f64 = 0.30;

/// Weight for inductive reasoning in synthesis.
pub const WEIGHT_INDUCTIVE: f64 = 0.25;

/// Weight for abductive reasoning in synthesis.
pub const WEIGHT_ABDUCTIVE: f64 = 0.20;

/// Weight for analogical reasoning in synthesis.
pub const WEIGHT_ANALOGICAL: f64 = 0.15;

/// Weight for adversarial reasoning in synthesis.
pub const WEIGHT_ADVERSARIAL: f64 = 0.10;

/// Maximum number of conclusions in a reasoning result.
pub const MAX_CONCLUSIONS: usize = 5;

/// Minimum synthesis confidence to accept a reasoning result.
pub const SYNTHESIS_CONFIDENCE_THRESHOLD: f64 = 0.35;
