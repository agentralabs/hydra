//! Depth detection — identifies whether input has surface or underlying meaning.
//!
//! Zero LLM calls. Detects repetition patterns and frustration indicators.

use serde::{Deserialize, Serialize};

/// Repetition keywords that suggest an underlying concern.
const REPETITION_KEYWORDS: &[&str] = &["again", "keep", "always", "every time", "still"];

/// Frustration indicators that suggest underlying depth.
const FRUSTRATION_INDICATORS: &[&str] = &[
    "third time",
    "keeps happening",
    "not working",
    "same issue",
    "fed up",
];

/// The depth level of an input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DepthLevel {
    /// Surface-level — the input means what it says.
    Surface,
    /// Has an underlying concern beyond the literal text.
    HasUnderlying {
        /// Description of the detected underlying concern.
        concern: String,
    },
}

impl DepthLevel {
    /// Return a human-readable label.
    pub fn label(&self) -> &str {
        match self {
            Self::Surface => "surface",
            Self::HasUnderlying { .. } => "has-underlying",
        }
    }
}

/// Detect the depth level of input text.
///
/// Looks for repetition patterns and frustration indicators.
/// Zero LLM calls.
pub fn detect_depth(text: &str) -> DepthLevel {
    let lower = text.to_lowercase();

    // Check frustration indicators first (stronger signal).
    for indicator in FRUSTRATION_INDICATORS {
        if lower.contains(indicator) {
            return DepthLevel::HasUnderlying {
                concern: format!("frustration detected: '{indicator}'"),
            };
        }
    }

    // Check repetition keywords.
    let rep_count = REPETITION_KEYWORDS
        .iter()
        .filter(|kw| lower.contains(**kw))
        .count();

    if rep_count >= 2 {
        return DepthLevel::HasUnderlying {
            concern: "multiple repetition signals — possible recurring issue".to_string(),
        };
    }

    if rep_count == 1 {
        // Single repetition keyword — mild underlying signal.
        let kw = REPETITION_KEYWORDS
            .iter()
            .find(|kw| lower.contains(**kw))
            .unwrap_or(&"unknown");
        return DepthLevel::HasUnderlying {
            concern: format!("repetition signal: '{kw}'"),
        };
    }

    DepthLevel::Surface
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_for_normal_input() {
        let d = detect_depth("deploy the api service");
        assert_eq!(d, DepthLevel::Surface);
    }

    #[test]
    fn underlying_from_frustration() {
        let d = detect_depth("this keeps happening with the deploy");
        match d {
            DepthLevel::HasUnderlying { concern } => {
                assert!(concern.contains("keeps happening"));
            }
            _ => panic!("expected HasUnderlying"),
        }
    }

    #[test]
    fn underlying_from_repetition() {
        let d = detect_depth("the build keeps failing again");
        assert!(matches!(d, DepthLevel::HasUnderlying { .. }));
    }
}
