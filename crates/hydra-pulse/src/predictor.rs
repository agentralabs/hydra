//! ResponsePredictor — prefetch responses based on partial input and history.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Result of a prediction attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionResult {
    pub matched: bool,
    pub response: Option<String>,
    pub confidence: f64,
    pub pattern: Option<String>,
}

impl PredictionResult {
    pub fn miss() -> Self {
        Self {
            matched: false,
            response: None,
            confidence: 0.0,
            pattern: None,
        }
    }

    pub fn hit(response: String, confidence: f64, pattern: String) -> Self {
        Self {
            matched: true,
            response: Some(response),
            confidence,
            pattern: Some(pattern),
        }
    }
}

/// A cached pattern with its response
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PatternEntry {
    pattern: String,
    response: String,
    hit_count: u64,
    confidence: f64,
}

/// Predicts responses based on partial input matching against known patterns.
///
/// Learns from completed responses to prefetch likely answers.
pub struct ResponsePredictor {
    /// Pattern -> response cache
    patterns: parking_lot::Mutex<HashMap<String, PatternEntry>>,
    /// Maximum patterns to cache
    max_patterns: usize,
    /// Minimum prefix length before attempting prediction
    min_prefix_len: usize,
}

impl ResponsePredictor {
    pub fn new(max_patterns: usize, min_prefix_len: usize) -> Self {
        Self {
            patterns: parking_lot::Mutex::new(HashMap::new()),
            max_patterns,
            min_prefix_len,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(500, 3)
    }

    /// Try to predict a response from partial input
    pub fn predict(&self, input: &str) -> PredictionResult {
        if input.len() < self.min_prefix_len {
            return PredictionResult::miss();
        }

        let normalized = input.to_lowercase().trim().to_string();
        let patterns = self.patterns.lock();

        // Exact match first
        if let Some(entry) = patterns.get(&normalized) {
            return PredictionResult::hit(
                entry.response.clone(),
                entry.confidence,
                entry.pattern.clone(),
            );
        }

        // Prefix match — find best matching pattern
        let mut best: Option<&PatternEntry> = None;
        let mut best_len = 0;

        for entry in patterns.values() {
            let pattern_lower = entry.pattern.to_lowercase();
            if pattern_lower.starts_with(&normalized) || normalized.starts_with(&pattern_lower) {
                let match_len = normalized.len().min(pattern_lower.len());
                if match_len > best_len {
                    best = Some(entry);
                    best_len = match_len;
                }
            }
        }

        match best {
            Some(entry) => {
                // Scale confidence by match quality
                let quality = best_len as f64 / entry.pattern.len().max(1) as f64;
                let adjusted_confidence = entry.confidence * quality;
                PredictionResult::hit(
                    entry.response.clone(),
                    adjusted_confidence,
                    entry.pattern.clone(),
                )
            }
            None => PredictionResult::miss(),
        }
    }

    /// Record a completed input→response pair for future prediction
    pub fn learn(&self, input: &str, response: &str) {
        let normalized = input.to_lowercase().trim().to_string();
        if normalized.is_empty() {
            return;
        }

        let mut patterns = self.patterns.lock();

        if let Some(entry) = patterns.get_mut(&normalized) {
            entry.hit_count += 1;
            // Confidence increases with repeated patterns
            entry.confidence = (entry.confidence + 0.1).min(1.0);
            entry.response = response.to_string();
        } else {
            // Evict least-used if at capacity
            if patterns.len() >= self.max_patterns {
                if let Some(least_key) = patterns
                    .iter()
                    .min_by_key(|(_, v)| v.hit_count)
                    .map(|(k, _)| k.clone())
                {
                    patterns.remove(&least_key);
                }
            }

            patterns.insert(
                normalized.clone(),
                PatternEntry {
                    pattern: input.to_string(),
                    response: response.to_string(),
                    hit_count: 1,
                    confidence: 0.5,
                },
            );
        }
    }

    /// Number of cached patterns
    pub fn pattern_count(&self) -> usize {
        self.patterns.lock().len()
    }

    /// Clear all patterns
    pub fn clear(&self) {
        self.patterns.lock().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_predictor_miss_on_empty() {
        let predictor = ResponsePredictor::with_defaults();
        let result = predictor.predict("hi");
        assert!(!result.matched);
    }

    #[test]
    fn test_predictor_learn_and_match() {
        let predictor = ResponsePredictor::with_defaults();
        predictor.learn("what is the weather", "I can check the weather for you");
        let result = predictor.predict("what is the weather");
        assert!(result.matched);
        assert_eq!(result.response.unwrap(), "I can check the weather for you");
    }

    #[test]
    fn test_predictor_partial_match() {
        let predictor = ResponsePredictor::with_defaults();
        predictor.learn(
            "how do I create a function",
            "You can create a function using...",
        );
        let result = predictor.predict("how do I create");
        assert!(result.matched);
        assert!(result.confidence > 0.0);
    }

    #[test]
    fn test_predictor_confidence_increases() {
        let predictor = ResponsePredictor::with_defaults();
        predictor.learn("hello", "Hi there!");
        predictor.learn("hello", "Hi there!");
        predictor.learn("hello", "Hi there!");
        let result = predictor.predict("hello");
        assert!(result.confidence > 0.5); // Increased from default 0.5
    }

    #[test]
    fn test_predictor_eviction() {
        let predictor = ResponsePredictor::new(2, 3);
        predictor.learn("first query here", "response 1");
        predictor.learn("second query here", "response 2");
        assert_eq!(predictor.pattern_count(), 2);
        // Third should evict least used
        predictor.learn("third query here", "response 3");
        assert_eq!(predictor.pattern_count(), 2);
    }

    #[test]
    fn test_prediction_result_miss() {
        let result = PredictionResult::miss();
        assert!(!result.matched);
        assert!(result.response.is_none());
        assert_eq!(result.confidence, 0.0);
        assert!(result.pattern.is_none());
    }

    #[test]
    fn test_prediction_result_hit() {
        let result = PredictionResult::hit("answer".into(), 0.9, "pattern".into());
        assert!(result.matched);
        assert_eq!(result.response, Some("answer".into()));
        assert_eq!(result.confidence, 0.9);
        assert_eq!(result.pattern, Some("pattern".into()));
    }

    #[test]
    fn test_predictor_short_input() {
        let predictor = ResponsePredictor::with_defaults();
        predictor.learn("hi", "hello");
        let result = predictor.predict("hi");
        assert!(!result.matched); // Below min_prefix_len of 3
    }

    #[test]
    fn test_predictor_clear() {
        let predictor = ResponsePredictor::with_defaults();
        predictor.learn("test input", "test response");
        assert_eq!(predictor.pattern_count(), 1);
        predictor.clear();
        assert_eq!(predictor.pattern_count(), 0);
    }

    #[test]
    fn test_predictor_learn_empty_input() {
        let predictor = ResponsePredictor::with_defaults();
        predictor.learn("", "response");
        assert_eq!(predictor.pattern_count(), 0);
    }

    #[test]
    fn test_predictor_case_insensitive() {
        let predictor = ResponsePredictor::with_defaults();
        predictor.learn("Hello World", "greeting");
        let result = predictor.predict("hello world");
        assert!(result.matched);
    }

    #[test]
    fn test_predictor_learn_updates_response() {
        let predictor = ResponsePredictor::with_defaults();
        predictor.learn("test query", "old response");
        predictor.learn("test query", "new response");
        let result = predictor.predict("test query");
        assert_eq!(result.response, Some("new response".into()));
    }

    #[test]
    fn test_prediction_result_serde() {
        let result = PredictionResult::hit("answer".into(), 0.9, "pat".into());
        let json = serde_json::to_string(&result).unwrap();
        let restored: PredictionResult = serde_json::from_str(&json).unwrap();
        assert!(restored.matched);
        assert_eq!(restored.confidence, 0.9);
    }

    #[test]
    fn test_predictor_with_defaults_settings() {
        let predictor = ResponsePredictor::with_defaults();
        // Should accept patterns of length >= 3
        predictor.learn("abc", "response");
        let result = predictor.predict("abc");
        assert!(result.matched);
    }

    #[test]
    fn test_predictor_multiple_patterns() {
        let predictor = ResponsePredictor::with_defaults();
        predictor.learn("how do I create a file", "Use touch command");
        predictor.learn("how do I delete a file", "Use rm command");
        assert_eq!(predictor.pattern_count(), 2);
        let r1 = predictor.predict("how do I create a file");
        assert!(r1.matched);
        let r2 = predictor.predict("how do I delete a file");
        assert!(r2.matched);
    }
}
