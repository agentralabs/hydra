//! Hedge detection — identifies uncertainty markers in text.
//!
//! Zero LLM calls. Pure keyword matching.

use crate::constants::HEDGE_CONFIDENCE_PENALTY;
use serde::{Deserialize, Serialize};

/// Hedge words that indicate uncertainty.
const HEDGE_WORDS: &[&str] = &[
    "maybe", "possibly", "perhaps", "might", "could", "probably", "i think", "sort of", "kind of",
    "somewhat",
];

/// Result of hedge detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HedgeResult {
    /// Whether any hedge words were detected.
    pub is_hedged: bool,
    /// The hedge words that were found.
    pub hedge_words: Vec<String>,
    /// The penalty to apply to confidence (sum of per-word penalties).
    pub penalty: f64,
}

/// Detect hedge words in text.
///
/// Returns which hedge words were found and the total penalty.
pub fn detect_hedges(text: &str) -> HedgeResult {
    let lower = text.to_lowercase();
    let found: Vec<String> = HEDGE_WORDS
        .iter()
        .filter(|hw| lower.contains(**hw))
        .map(|hw| hw.to_string())
        .collect();

    let penalty = found.len() as f64 * HEDGE_CONFIDENCE_PENALTY;

    HedgeResult {
        is_hedged: !found.is_empty(),
        hedge_words: found,
        penalty,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_hedged_text() {
        let r = detect_hedges("maybe we should probably deploy");
        assert!(r.is_hedged);
        assert!(r.hedge_words.contains(&"maybe".to_string()));
        assert!(r.hedge_words.contains(&"probably".to_string()));
    }

    #[test]
    fn certain_text_not_hedged() {
        let r = detect_hedges("deploy the service now");
        assert!(!r.is_hedged);
        assert!(r.hedge_words.is_empty());
        assert!((r.penalty - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn penalty_scales_with_count() {
        let r = detect_hedges("maybe possibly perhaps");
        assert_eq!(r.hedge_words.len(), 3);
        let expected = 3.0 * HEDGE_CONFIDENCE_PENALTY;
        assert!((r.penalty - expected).abs() < f64::EPSILON);
    }
}
