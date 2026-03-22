//! Prediction stager — records intents and generates predictions.

use crate::constants::{PATTERN_WINDOW_SIZE, PREDICTION_MIN_CONFIDENCE};
use crate::intent::{IntentPrediction, PredictionBasis, PredictionStage};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A recorded intent from the principal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedIntent {
    /// Description of the intent.
    pub description: String,
    /// When this intent was recorded.
    pub recorded_at: DateTime<Utc>,
    /// Optional task context.
    pub task_context: Option<String>,
}

/// The prediction stager: records intents and generates predictions.
pub struct PredictionStager {
    history: Vec<RecordedIntent>,
    stage: PredictionStage,
}

impl PredictionStager {
    /// Create a new prediction stager.
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            stage: PredictionStage::new(),
        }
    }

    /// Record a new intent from the principal.
    pub fn record_intent(&mut self, description: impl Into<String>, task_context: Option<String>) {
        self.history.push(RecordedIntent {
            description: description.into(),
            recorded_at: Utc::now(),
            task_context,
        });

        // Keep only the most recent window
        if self.history.len() > PATTERN_WINDOW_SIZE {
            let drain_count = self.history.len() - PATTERN_WINDOW_SIZE;
            self.history.drain(..drain_count);
        }
    }

    /// Run a prediction cycle, generating new predictions from history.
    ///
    /// Generates predictions from:
    /// 1. Temporal patterns (repeated intents)
    /// 2. Task state (if an active task context exists)
    /// 3. History-based patterns (recent intent sequences)
    pub fn run_cycle(&mut self) {
        if self.history.is_empty() {
            return;
        }

        // 1. Temporal patterns — find repeated descriptions
        let temporal_preds = self.detect_temporal_patterns();
        for pred in temporal_preds {
            self.stage.update(pred);
        }

        // 2. Task-based predictions
        let task_preds = self.detect_task_patterns();
        for pred in task_preds {
            self.stage.update(pred);
        }

        // 3. History sequence prediction — predict the next based on last
        if let Some(last) = self.history.last() {
            let pred = IntentPrediction::new(
                format!("continuation of: {}", last.description),
                PREDICTION_MIN_CONFIDENCE + 0.1,
                PredictionBasis::SessionPattern,
            );
            self.stage.update(pred);
        }
    }

    /// Get the current prediction stage.
    pub fn stage(&self) -> &PredictionStage {
        &self.stage
    }

    /// Get the intent history.
    pub fn history(&self) -> &[RecordedIntent] {
        &self.history
    }

    /// Detect temporal patterns from repeated intents.
    fn detect_temporal_patterns(&self) -> Vec<IntentPrediction> {
        let mut freq: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for intent in &self.history {
            *freq.entry(&intent.description).or_insert(0) += 1;
        }

        freq.iter()
            .filter(|(_, &count)| count >= 2)
            .map(|(desc, &count)| {
                let confidence = (count as f64 / self.history.len() as f64).min(0.95);
                IntentPrediction::new(
                    format!("likely repeat: {desc}"),
                    confidence,
                    PredictionBasis::TemporalPattern,
                )
            })
            .collect()
    }

    /// Detect patterns from task contexts.
    fn detect_task_patterns(&self) -> Vec<IntentPrediction> {
        let mut preds = Vec::new();
        let task_intents: Vec<&RecordedIntent> = self
            .history
            .iter()
            .filter(|i| i.task_context.is_some())
            .collect();

        if task_intents.len() >= 2 {
            if let Some(last) = task_intents.last() {
                if let Some(ctx) = &last.task_context {
                    preds.push(IntentPrediction::new(
                        format!("task continuation: {ctx}"),
                        0.6,
                        PredictionBasis::ActiveTaskState,
                    ));
                }
            }
        }

        preds
    }
}

impl Default for PredictionStager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_predict() {
        let mut stager = PredictionStager::new();
        stager.record_intent("check deployment", None);
        stager.record_intent("check deployment", None);
        stager.record_intent("review code", None);
        stager.run_cycle();
        assert!(!stager.stage().top().is_empty());
    }

    #[test]
    fn history_window_respected() {
        let mut stager = PredictionStager::new();
        for i in 0..30 {
            stager.record_intent(format!("intent-{i}"), None);
        }
        assert_eq!(stager.history().len(), PATTERN_WINDOW_SIZE);
    }

    #[test]
    fn task_context_predictions() {
        let mut stager = PredictionStager::new();
        stager.record_intent("start deploy", Some("deploy-task".into()));
        stager.record_intent("check status", Some("deploy-task".into()));
        stager.run_cycle();
        let preds = stager.stage().top();
        let has_task_pred = preds
            .iter()
            .any(|p| p.basis == PredictionBasis::ActiveTaskState);
        assert!(has_task_pred, "should generate task-based prediction");
    }
}
