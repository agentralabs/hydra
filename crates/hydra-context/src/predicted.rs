//! Predicted context window — built from staged intent predictions.

use crate::window::{ContextItem, ContextWindow};
use serde::{Deserialize, Serialize};

/// A staged intent that Hydra is preparing for.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StagedIntent {
    /// Description of the predicted intent.
    pub description: String,
    /// Confidence in this prediction (0.0 to 1.0).
    pub confidence: f64,
    /// Why this was predicted.
    pub basis: String,
}

impl StagedIntent {
    /// Create a new staged intent.
    pub fn new(description: impl Into<String>, confidence: f64, basis: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            confidence: confidence.clamp(0.0, 1.0),
            basis: basis.into(),
        }
    }
}

/// Build a predicted context window from staged intents.
///
/// Items are ordered by confidence (highest first via the window's
/// significance-based ordering).
pub fn build_predicted(staged: &[StagedIntent]) -> ContextWindow {
    let mut window = ContextWindow::new("predicted");
    for intent in staged {
        window.add(ContextItem::new(
            format!("[{}] {}", intent.basis, intent.description),
            intent.confidence,
        ));
    }
    window
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn predicted_window_ordered_by_confidence() {
        let staged = vec![
            StagedIntent::new("low priority", 0.3, "session"),
            StagedIntent::new("high priority", 0.9, "temporal"),
            StagedIntent::new("medium priority", 0.6, "task"),
        ];
        let window = build_predicted(&staged);
        assert_eq!(window.len(), 3);
        assert!(window.items[0].significance >= window.items[1].significance);
        assert!(window.items[1].significance >= window.items[2].significance);
    }

    #[test]
    fn empty_staged_empty_window() {
        let window = build_predicted(&[]);
        assert!(window.is_empty());
    }
}
