//! Speculative processor — matches partial transcripts to predictions.
//!
//! Enables Hydra to begin processing before the user finishes speaking.

use crate::constants;

/// A prediction candidate for speculative matching.
#[derive(Debug, Clone)]
pub struct PredictionCandidate {
    /// The predicted intent.
    pub intent: String,
    /// Confidence in this prediction (0.0 to 1.0).
    pub confidence: f64,
}

/// Result of speculative matching.
#[derive(Debug, Clone, PartialEq)]
pub enum SpeculativeResult {
    /// No match found.
    NoMatch,
    /// A match was found with the given intent and confidence.
    Match {
        /// The matched intent.
        intent: String,
        /// Confidence in the match.
        confidence: f64,
    },
    /// Waiting for more input.
    Pending,
    /// A previous speculative match was confirmed by the final transcript.
    Confirmed {
        /// The confirmed intent.
        intent: String,
    },
    /// A previous speculative match was rejected by the final transcript.
    Rejected,
}

/// Matches partial transcripts against prediction candidates.
#[derive(Debug, Clone)]
pub struct SpeculativeProcessor {
    /// Current prediction candidates.
    candidates: Vec<PredictionCandidate>,
    /// The last speculative match (if any).
    last_match: Option<String>,
}

impl SpeculativeProcessor {
    /// Create a new speculative processor.
    pub fn new() -> Self {
        Self {
            candidates: Vec::new(),
            last_match: None,
        }
    }

    /// Update the prediction candidates.
    pub fn update_predictions(&mut self, candidates: Vec<PredictionCandidate>) {
        let max = constants::MAX_PREDICTIONS;
        if candidates.len() > max {
            self.candidates = candidates[..max].to_vec();
        } else {
            self.candidates = candidates;
        }
    }

    /// Check a partial transcript against predictions.
    pub fn check_partial(&mut self, text: &str) -> SpeculativeResult {
        if text.len() < constants::MIN_PARTIAL_LENGTH {
            return SpeculativeResult::Pending;
        }

        let lower = text.to_lowercase();
        let mut best_match: Option<(&str, f64)> = None;

        for candidate in &self.candidates {
            let intent_lower = candidate.intent.to_lowercase();
            // Simple prefix/substring matching. In production, use
            // fuzzy matching or the LLM.
            let similarity = if intent_lower.starts_with(&lower) || lower.starts_with(&intent_lower)
            {
                candidate.confidence
            } else if intent_lower.contains(&lower) || lower.contains(&intent_lower) {
                candidate.confidence * 0.8
            } else {
                0.0
            };

            if similarity >= constants::SPECULATIVE_MATCH_THRESHOLD
                && best_match.is_none_or(|(_, s)| similarity > s)
            {
                best_match = Some((&candidate.intent, similarity));
            }
        }

        if let Some((intent, confidence)) = best_match {
            self.last_match = Some(intent.to_string());
            SpeculativeResult::Match {
                intent: intent.to_string(),
                confidence,
            }
        } else if self.candidates.is_empty() {
            SpeculativeResult::NoMatch
        } else {
            SpeculativeResult::Pending
        }
    }

    /// Validate the final transcript against the last speculative match.
    pub fn validate_final(&mut self, text: &str) -> SpeculativeResult {
        let Some(ref last) = self.last_match else {
            return SpeculativeResult::NoMatch;
        };

        let lower = text.to_lowercase();
        let intent_lower = last.to_lowercase();

        let confirmed = lower.contains(&intent_lower) || intent_lower.contains(&lower);

        let result = if confirmed {
            SpeculativeResult::Confirmed {
                intent: last.clone(),
            }
        } else {
            SpeculativeResult::Rejected
        };

        self.last_match = None;
        result
    }

    /// Return the current prediction candidates.
    pub fn candidates(&self) -> &[PredictionCandidate] {
        &self.candidates
    }

    /// Return the last speculative match intent.
    pub fn last_match(&self) -> Option<&str> {
        self.last_match.as_deref()
    }

    /// Clear all state.
    pub fn clear(&mut self) {
        self.candidates.clear();
        self.last_match = None;
    }
}

impl Default for SpeculativeProcessor {
    fn default() -> Self {
        Self::new()
    }
}
