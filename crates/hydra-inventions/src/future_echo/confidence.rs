//! ConfidenceModel — scoring and calibration for predictions.

use serde::{Deserialize, Serialize};

/// A confidence score with calibration metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceScore {
    pub value: f32,
    pub calibrated: bool,
    pub evidence_count: usize,
}

impl ConfidenceScore {
    pub fn new(value: f32) -> Self {
        Self {
            value: value.clamp(0.0, 1.0),
            calibrated: false,
            evidence_count: 0,
        }
    }

    pub fn with_evidence(mut self, count: usize) -> Self {
        self.evidence_count = count;
        self
    }

    pub fn is_high(&self) -> bool {
        self.value >= 0.8
    }

    pub fn is_medium(&self) -> bool {
        self.value >= 0.5 && self.value < 0.8
    }

    pub fn is_low(&self) -> bool {
        self.value < 0.5
    }
}

/// Confidence model that tracks prediction accuracy
pub struct ConfidenceModel {
    predictions: parking_lot::RwLock<Vec<PredictionRecord>>,
    calibration_factor: parking_lot::Mutex<f32>,
}

#[derive(Debug, Clone)]
struct PredictionRecord {
    predicted: f32,
    actual: Option<bool>,
}

impl ConfidenceModel {
    pub fn new() -> Self {
        Self {
            predictions: parking_lot::RwLock::new(Vec::new()),
            calibration_factor: parking_lot::Mutex::new(1.0),
        }
    }

    /// Record a prediction
    pub fn record_prediction(&self, confidence: f32) -> usize {
        let mut preds = self.predictions.write();
        let id = preds.len();
        preds.push(PredictionRecord {
            predicted: confidence,
            actual: None,
        });
        id
    }

    /// Record the actual outcome for a prediction
    pub fn record_outcome(&self, prediction_id: usize, success: bool) {
        let mut preds = self.predictions.write();
        if let Some(record) = preds.get_mut(prediction_id) {
            record.actual = Some(success);
        }
        drop(preds);
        self.recalibrate();
    }

    /// Get calibrated confidence score
    pub fn calibrate(&self, raw_confidence: f32) -> ConfidenceScore {
        let factor = *self.calibration_factor.lock();
        let calibrated = (raw_confidence * factor).clamp(0.0, 1.0);
        let evidence = self
            .predictions
            .read()
            .iter()
            .filter(|p| p.actual.is_some())
            .count();

        ConfidenceScore {
            value: calibrated,
            calibrated: true,
            evidence_count: evidence,
        }
    }

    /// Recalibrate based on prediction history
    fn recalibrate(&self) {
        let preds = self.predictions.read();
        let resolved: Vec<_> = preds.iter().filter(|p| p.actual.is_some()).collect();

        if resolved.len() < 3 {
            return; // Not enough data
        }

        let mut total_predicted = 0.0f32;
        let mut total_actual = 0.0f32;

        for record in &resolved {
            total_predicted += record.predicted;
            total_actual += if record.actual.unwrap() { 1.0 } else { 0.0 };
        }

        let avg_predicted = total_predicted / resolved.len() as f32;
        let avg_actual = total_actual / resolved.len() as f32;

        if avg_predicted > 0.01 {
            *self.calibration_factor.lock() = (avg_actual / avg_predicted).clamp(0.5, 2.0);
        }
    }

    /// Get prediction accuracy
    pub fn accuracy(&self) -> Option<f32> {
        let preds = self.predictions.read();
        let resolved: Vec<_> = preds.iter().filter(|p| p.actual.is_some()).collect();

        if resolved.is_empty() {
            return None;
        }

        let correct = resolved
            .iter()
            .filter(|p| {
                let predicted_success = p.predicted >= 0.5;
                p.actual.unwrap() == predicted_success
            })
            .count();

        Some(correct as f32 / resolved.len() as f32)
    }

    pub fn total_predictions(&self) -> usize {
        self.predictions.read().len()
    }
}

impl Default for ConfidenceModel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confidence_scoring() {
        let score = ConfidenceScore::new(0.85);
        assert!(score.is_high());
        assert!(!score.is_medium());
        assert!(!score.is_low());

        let low = ConfidenceScore::new(0.3);
        assert!(low.is_low());
    }

    #[test]
    fn test_prediction_accuracy() {
        let model = ConfidenceModel::new();

        let id1 = model.record_prediction(0.9);
        model.record_outcome(id1, true);

        let id2 = model.record_prediction(0.8);
        model.record_outcome(id2, true);

        let id3 = model.record_prediction(0.2);
        model.record_outcome(id3, false);

        let accuracy = model.accuracy().unwrap();
        assert_eq!(accuracy, 1.0); // All predictions correct
    }

    #[test]
    fn test_calibration() {
        let model = ConfidenceModel::new();

        // Record overly confident predictions that fail
        for i in 0..5 {
            let id = model.record_prediction(0.9);
            model.record_outcome(id, i < 2); // Only 2/5 succeed
        }

        let calibrated = model.calibrate(0.9);
        assert!(calibrated.calibrated);
        assert!(calibrated.value < 0.9); // Should be adjusted down
    }
}
