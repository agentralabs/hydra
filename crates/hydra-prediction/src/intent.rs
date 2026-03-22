//! Intent prediction types and the prediction stage.

use crate::constants::{PREDICTION_MIN_CONFIDENCE, PREDICTION_SLOT_COUNT};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The basis for a prediction — why Hydra thinks this will happen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PredictionBasis {
    /// Based on temporal patterns (e.g., user does X every morning).
    TemporalPattern,
    /// Based on the consequence of a previous action.
    ActionConsequence,
    /// Based on session-level patterns (e.g., user always does Y after Z).
    SessionPattern,
    /// Based on the current active task state.
    ActiveTaskState,
}

/// A single predicted intent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentPrediction {
    /// Unique identifier.
    pub id: String,
    /// What Hydra predicts will happen.
    pub description: String,
    /// Confidence in this prediction (0.0 to 1.0).
    pub confidence: f64,
    /// Why this prediction was made.
    pub basis: PredictionBasis,
    /// When this prediction was generated.
    pub generated_at: DateTime<Utc>,
}

impl IntentPrediction {
    /// Create a new intent prediction.
    pub fn new(description: impl Into<String>, confidence: f64, basis: PredictionBasis) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            description: description.into(),
            confidence: confidence.clamp(0.0, 1.0),
            basis,
            generated_at: Utc::now(),
        }
    }
}

/// Maintains a fixed-size set of top predictions.
#[derive(Debug, Clone)]
pub struct PredictionStage {
    predictions: Vec<IntentPrediction>,
}

impl PredictionStage {
    /// Create a new empty prediction stage.
    pub fn new() -> Self {
        Self {
            predictions: Vec::new(),
        }
    }

    /// Update the stage with a new prediction.
    ///
    /// If below the slot count, the prediction is simply added.
    /// If at capacity, the lowest-confidence prediction is replaced
    /// if the new one has higher confidence.
    pub fn update(&mut self, prediction: IntentPrediction) {
        if prediction.confidence < PREDICTION_MIN_CONFIDENCE {
            return;
        }

        if self.predictions.len() < PREDICTION_SLOT_COUNT {
            self.predictions.push(prediction);
            self.sort_descending();
            return;
        }

        // Replace lowest if new one is better
        if let Some(lowest) = self.predictions.last() {
            if prediction.confidence > lowest.confidence {
                self.predictions.pop();
                self.predictions.push(prediction);
                self.sort_descending();
            }
        }
    }

    /// Return the top predictions, ordered by confidence descending.
    pub fn top(&self) -> &[IntentPrediction] {
        &self.predictions
    }

    /// Return the single best prediction, if any.
    pub fn best(&self) -> Option<&IntentPrediction> {
        self.predictions.first()
    }

    /// Sort predictions by confidence, highest first.
    fn sort_descending(&mut self) {
        self.predictions.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
}

impl Default for PredictionStage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_respects_slot_count() {
        let mut stage = PredictionStage::new();
        for i in 0..10 {
            stage.update(IntentPrediction::new(
                format!("pred-{i}"),
                0.3 + (i as f64) * 0.05,
                PredictionBasis::TemporalPattern,
            ));
        }
        assert_eq!(stage.top().len(), PREDICTION_SLOT_COUNT);
    }

    #[test]
    fn low_confidence_rejected() {
        let mut stage = PredictionStage::new();
        stage.update(IntentPrediction::new(
            "low",
            0.1,
            PredictionBasis::SessionPattern,
        ));
        assert!(stage.top().is_empty());
    }

    #[test]
    fn best_is_highest_confidence() {
        let mut stage = PredictionStage::new();
        stage.update(IntentPrediction::new(
            "a",
            0.5,
            PredictionBasis::TemporalPattern,
        ));
        stage.update(IntentPrediction::new(
            "b",
            0.9,
            PredictionBasis::ActionConsequence,
        ));
        assert_eq!(stage.best().unwrap().description, "b");
    }
}
