//! Situation and approach signatures for genome matching.

use crate::constants::SIGNATURE_MAX_KEYWORDS;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// A signature derived from a situation description.
///
/// Keywords are extracted from the description and stored as a set
/// for Jaccard similarity computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SituationSignature {
    /// Normalized keywords extracted from the situation.
    pub keywords: BTreeSet<String>,
}

impl SituationSignature {
    /// Create a new situation signature from a description string.
    ///
    /// Extracts lowercase keywords with rudimentary stemming,
    /// filtering out short words and capping at `SIGNATURE_MAX_KEYWORDS`.
    /// Stemming ensures "services"→"service", "failures"→"failur",
    /// "cascading"→"cascad", etc. for indirect phrasing matches.
    pub fn from_description(description: &str) -> Self {
        let keywords: BTreeSet<String> = description
            .split_whitespace()
            .map(|w| w.to_lowercase().replace(|c: char| !c.is_alphanumeric(), ""))
            .filter(|w| w.len() >= 3)
            .map(|w| stem(&w))
            .take(SIGNATURE_MAX_KEYWORDS)
            .collect();
        Self { keywords }
    }

    /// Compute Jaccard similarity between two situation signatures.
    ///
    /// Returns a value in [0.0, 1.0]. Returns 0.0 if both sets are empty.
    pub fn similarity(&self, other: &Self) -> f64 {
        if self.keywords.is_empty() && other.keywords.is_empty() {
            return 0.0;
        }
        let intersection = self.keywords.intersection(&other.keywords).count();
        let union = self.keywords.union(&other.keywords).count();
        if union == 0 {
            return 0.0;
        }
        intersection as f64 / union as f64
    }
}

/// A signature describing an approach to a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApproachSignature {
    /// The type of approach (e.g., "api_call", "file_edit", "decompose").
    pub approach_type: String,
    /// Ordered steps in the approach.
    pub steps: Vec<String>,
    /// Tools used during the approach.
    pub tools_used: Vec<String>,
}

impl ApproachSignature {
    /// Create a new approach signature.
    pub fn new(
        approach_type: impl Into<String>,
        steps: Vec<String>,
        tools_used: Vec<String>,
    ) -> Self {
        Self {
            approach_type: approach_type.into(),
            steps,
            tools_used,
        }
    }
}

/// Rudimentary suffix-stripping stemmer.
/// Not Porter — intentionally simple. Strips common English suffixes
/// so "services"→"servic", "failures"→"failur", "cascading"→"cascad".
/// The point is collision: we want "service" and "services" to collide.
fn stem(word: &str) -> String {
    let w = word.to_string();
    // Order matters: longest suffixes first
    for suffix in &[
        "ation", "ment", "ness", "ting", "ing", "sion", "tion",
        "able", "ible", "ful", "less", "ous", "ive", "ies",
        "ied", "ers", "est", "ely", "ity",
        "ed", "er", "ly", "es", "al",
        "s",
    ] {
        if w.len() > suffix.len() + 2 && w.ends_with(suffix) {
            return w[..w.len() - suffix.len()].to_string();
        }
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jaccard_identical() {
        let a = SituationSignature::from_description("deploy rest api service");
        let b = SituationSignature::from_description("deploy rest api service");
        assert!((a.similarity(&b) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn jaccard_disjoint() {
        let a = SituationSignature::from_description("deploy rest api");
        let b = SituationSignature::from_description("compile rust binary");
        assert!(a.similarity(&b) < 0.01);
    }

    #[test]
    fn jaccard_partial() {
        let a = SituationSignature::from_description("deploy rest api service");
        let b = SituationSignature::from_description("deploy grpc api service");
        let sim = a.similarity(&b);
        assert!(sim > 0.3);
        assert!(sim < 1.0);
    }

    #[test]
    fn jaccard_empty() {
        let a = SituationSignature::from_description("");
        let b = SituationSignature::from_description("");
        assert!((a.similarity(&b)).abs() < f64::EPSILON);
    }

    #[test]
    fn keywords_capped() {
        let long = (0..100)
            .map(|i| format!("keyword{}", i))
            .collect::<Vec<_>>()
            .join(" ");
        let sig = SituationSignature::from_description(&long);
        assert!(sig.keywords.len() <= SIGNATURE_MAX_KEYWORDS);
    }

    #[test]
    fn short_words_filtered() {
        let sig = SituationSignature::from_description("a to do the big deploy");
        assert!(!sig.keywords.contains("a"));
        assert!(!sig.keywords.contains("to"));
        assert!(!sig.keywords.contains("do"));
        assert!(sig.keywords.contains("the"));
        assert!(sig.keywords.contains("big"));
        assert!(sig.keywords.contains("deploy"));
    }
}
