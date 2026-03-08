//! TemporalPredictor — predict future states based on temporal patterns.

use serde::{Deserialize, Serialize};

/// A temporal prediction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalPrediction {
    pub id: String,
    pub description: String,
    pub confidence: f64,
    pub predicted_time: String,
    pub basis: String,
}

/// Predicts future events based on temporal patterns
pub struct TemporalPredictor {
    patterns: parking_lot::RwLock<Vec<TemporalPattern>>,
    predictions: parking_lot::RwLock<Vec<TemporalPrediction>>,
}

#[derive(Debug, Clone)]
struct TemporalPattern {
    description: String,
    frequency_hours: f64,
    last_occurrence_ms: i64,
    occurrence_count: u64,
}

impl TemporalPredictor {
    pub fn new() -> Self {
        Self {
            patterns: parking_lot::RwLock::new(Vec::new()),
            predictions: parking_lot::RwLock::new(Vec::new()),
        }
    }

    /// Record a recurring event
    pub fn record_event(&self, description: &str, frequency_hours: f64) {
        let mut patterns = self.patterns.write();
        if let Some(p) = patterns.iter_mut().find(|p| p.description == description) {
            p.occurrence_count += 1;
            p.last_occurrence_ms = chrono::Utc::now().timestamp_millis();
        } else {
            patterns.push(TemporalPattern {
                description: description.into(),
                frequency_hours,
                last_occurrence_ms: chrono::Utc::now().timestamp_millis(),
                occurrence_count: 1,
            });
        }
    }

    /// Predict next occurrences of known patterns
    pub fn predict_next(&self) -> Vec<TemporalPrediction> {
        let patterns = self.patterns.read();
        let mut predictions = Vec::new();

        for pattern in patterns.iter() {
            if pattern.occurrence_count < 2 {
                continue; // Need at least 2 occurrences for prediction
            }

            let next_ms =
                pattern.last_occurrence_ms + (pattern.frequency_hours * 3_600_000.0) as i64;
            let next_time = chrono::DateTime::from_timestamp_millis(next_ms)
                .unwrap_or(chrono::Utc::now())
                .to_rfc3339();

            let confidence = (pattern.occurrence_count as f64 / 10.0).min(0.95);

            let prediction = TemporalPrediction {
                id: uuid::Uuid::new_v4().to_string(),
                description: format!("Next occurrence of '{}'", pattern.description),
                confidence,
                predicted_time: next_time,
                basis: format!(
                    "Observed {} times at ~{:.1}h intervals",
                    pattern.occurrence_count, pattern.frequency_hours,
                ),
            };
            predictions.push(prediction);
        }

        self.predictions.write().extend(predictions.clone());
        predictions
    }

    pub fn pattern_count(&self) -> usize {
        self.patterns.read().len()
    }

    pub fn prediction_count(&self) -> usize {
        self.predictions.read().len()
    }
}

impl Default for TemporalPredictor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temporal_prediction() {
        let predictor = TemporalPredictor::new();
        predictor.record_event("daily standup", 24.0);
        predictor.record_event("daily standup", 24.0);
        predictor.record_event("daily standup", 24.0);

        let predictions = predictor.predict_next();
        assert_eq!(predictions.len(), 1);
        assert!(predictions[0].confidence > 0.0);
        assert!(predictions[0].description.contains("daily standup"));
    }

    #[test]
    fn test_insufficient_data() {
        let predictor = TemporalPredictor::new();
        predictor.record_event("one-off", 1.0);

        let predictions = predictor.predict_next();
        assert!(predictions.is_empty()); // Need 2+ occurrences
    }

    #[test]
    fn test_multiple_patterns() {
        let predictor = TemporalPredictor::new();
        for _ in 0..3 {
            predictor.record_event("event_a", 12.0);
            predictor.record_event("event_b", 24.0);
        }

        let predictions = predictor.predict_next();
        assert_eq!(predictions.len(), 2);
        assert_eq!(predictor.pattern_count(), 2);
    }
}
