//! ResonanceModel — learns user preferences over time.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A user preference dimension
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreference {
    pub dimension: String,
    pub value: f64,
    pub observations: u32,
}

/// Score from the resonance model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResonanceScore {
    pub overall: f64,
    pub dimensions: Vec<(String, f64)>,
}

/// Learns user preferences from interactions to improve response quality.
///
/// Tracks dimensions like verbosity, formality, detail level, and
/// adjusts future responses to match observed preferences.
pub struct ResonanceModel {
    preferences: parking_lot::Mutex<HashMap<String, UserPreference>>,
    /// How quickly the model adapts (0.0-1.0, higher = faster)
    learning_rate: f64,
}

impl ResonanceModel {
    pub fn new(learning_rate: f64) -> Self {
        Self {
            preferences: parking_lot::Mutex::new(HashMap::new()),
            learning_rate: learning_rate.clamp(0.01, 1.0),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(0.1)
    }

    /// Record an observation for a preference dimension.
    /// Value should be in [0.0, 1.0].
    pub fn observe(&self, dimension: &str, value: f64) {
        let clamped = value.clamp(0.0, 1.0);
        let mut prefs = self.preferences.lock();

        if let Some(pref) = prefs.get_mut(dimension) {
            // Exponential moving average
            pref.value = pref.value * (1.0 - self.learning_rate) + clamped * self.learning_rate;
            pref.observations += 1;
        } else {
            prefs.insert(
                dimension.to_string(),
                UserPreference {
                    dimension: dimension.to_string(),
                    value: clamped,
                    observations: 1,
                },
            );
        }
    }

    /// Get the current preference value for a dimension
    pub fn preference(&self, dimension: &str) -> Option<f64> {
        self.preferences.lock().get(dimension).map(|p| p.value)
    }

    /// Get all preferences
    pub fn all_preferences(&self) -> Vec<UserPreference> {
        self.preferences.lock().values().cloned().collect()
    }

    /// Score how well a proposed response matches user preferences.
    /// `traits` maps dimension → value for the proposed response.
    pub fn score(&self, traits: &HashMap<String, f64>) -> ResonanceScore {
        let prefs = self.preferences.lock();
        let mut dimensions = Vec::new();
        let mut total_score = 0.0;
        let mut count = 0;

        for (dim, &trait_value) in traits {
            if let Some(pref) = prefs.get(dim) {
                // Score is 1.0 when trait matches preference, 0.0 when maximally different
                let dim_score = 1.0 - (pref.value - trait_value).abs();
                dimensions.push((dim.clone(), dim_score));
                total_score += dim_score;
                count += 1;
            }
        }

        let overall = if count > 0 {
            total_score / count as f64
        } else {
            0.5 // Neutral when no preferences known
        };

        ResonanceScore {
            overall,
            dimensions,
        }
    }

    /// Number of tracked dimensions
    pub fn dimension_count(&self) -> usize {
        self.preferences.lock().len()
    }

    /// Clear all learned preferences
    pub fn reset(&self) {
        self.preferences.lock().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_observe_and_retrieve() {
        let model = ResonanceModel::with_defaults();
        model.observe("verbosity", 0.8);
        let pref = model.preference("verbosity").unwrap();
        assert!((pref - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_exponential_moving_average() {
        let model = ResonanceModel::new(0.5);
        model.observe("detail", 1.0);
        model.observe("detail", 0.0);
        // With lr=0.5: 1.0*0.5 + 0.0*0.5 = 0.5
        let pref = model.preference("detail").unwrap();
        assert!((pref - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_score_matching() {
        let model = ResonanceModel::with_defaults();
        model.observe("verbosity", 0.8);
        model.observe("formality", 0.3);

        let mut traits = HashMap::new();
        traits.insert("verbosity".into(), 0.8);
        traits.insert("formality".into(), 0.3);

        let score = model.score(&traits);
        // Perfect match
        assert!(score.overall > 0.95);
    }

    #[test]
    fn test_score_mismatch() {
        let model = ResonanceModel::with_defaults();
        model.observe("verbosity", 0.9);

        let mut traits = HashMap::new();
        traits.insert("verbosity".into(), 0.1);

        let score = model.score(&traits);
        // Big mismatch
        assert!(score.overall < 0.3);
    }

    #[test]
    fn test_score_no_preferences() {
        let model = ResonanceModel::with_defaults();
        let mut traits = HashMap::new();
        traits.insert("verbosity".into(), 0.5);
        let score = model.score(&traits);
        assert!((score.overall - 0.5).abs() < 0.01); // Neutral
    }

    #[test]
    fn test_reset() {
        let model = ResonanceModel::with_defaults();
        model.observe("test", 0.5);
        assert_eq!(model.dimension_count(), 1);
        model.reset();
        assert_eq!(model.dimension_count(), 0);
    }

    #[test]
    fn test_clamp_learning_rate_high() {
        let model = ResonanceModel::new(5.0);
        model.observe("dim", 0.5);
        // Should not panic, learning rate clamped to 1.0
        assert!(model.preference("dim").is_some());
    }

    #[test]
    fn test_clamp_learning_rate_low() {
        let model = ResonanceModel::new(-1.0);
        model.observe("dim", 0.5);
        assert!(model.preference("dim").is_some());
    }

    #[test]
    fn test_observe_clamps_value() {
        let model = ResonanceModel::with_defaults();
        model.observe("dim", 2.0);
        let v = model.preference("dim").unwrap();
        assert!(v <= 1.0);
    }

    #[test]
    fn test_observe_clamps_negative() {
        let model = ResonanceModel::with_defaults();
        model.observe("dim", -1.0);
        let v = model.preference("dim").unwrap();
        assert!(v >= 0.0);
    }

    #[test]
    fn test_all_preferences() {
        let model = ResonanceModel::with_defaults();
        model.observe("a", 0.1);
        model.observe("b", 0.9);
        let prefs = model.all_preferences();
        assert_eq!(prefs.len(), 2);
    }

    #[test]
    fn test_preference_nonexistent() {
        let model = ResonanceModel::with_defaults();
        assert!(model.preference("nope").is_none());
    }

    #[test]
    fn test_user_preference_serde() {
        let pref = UserPreference { dimension: "test".into(), value: 0.5, observations: 3 };
        let json = serde_json::to_string(&pref).unwrap();
        let restored: UserPreference = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.dimension, "test");
        assert_eq!(restored.observations, 3);
    }

    #[test]
    fn test_resonance_score_serde() {
        let score = ResonanceScore { overall: 0.8, dimensions: vec![("dim".into(), 0.9)] };
        let json = serde_json::to_string(&score).unwrap();
        let restored: ResonanceScore = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.overall, 0.8);
    }

    #[test]
    fn test_multiple_observations_increase_count() {
        let model = ResonanceModel::with_defaults();
        model.observe("dim", 0.5);
        model.observe("dim", 0.6);
        model.observe("dim", 0.7);
        let prefs = model.all_preferences();
        let pref = prefs.iter().find(|p| p.dimension == "dim").unwrap();
        assert_eq!(pref.observations, 3);
    }

    #[test]
    fn test_score_partial_dimensions() {
        let model = ResonanceModel::with_defaults();
        model.observe("a", 0.5);
        model.observe("b", 0.5);
        let mut traits = HashMap::new();
        traits.insert("a".into(), 0.5); // only score on "a", not "b"
        let score = model.score(&traits);
        assert!(score.overall > 0.95); // near perfect match on the one dimension
        assert_eq!(score.dimensions.len(), 1);
    }
}
