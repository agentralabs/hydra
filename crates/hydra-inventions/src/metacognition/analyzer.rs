//! CognitiveAnalyzer — analyze thinking patterns and cognitive metrics.

use serde::{Deserialize, Serialize};

/// A recognized thinking pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ThinkingPattern {
    /// Systematic step-by-step reasoning
    Systematic,
    /// Intuitive pattern matching
    Intuitive,
    /// Analogical reasoning (comparing to known cases)
    Analogical,
    /// Exploratory (trying multiple approaches)
    Exploratory,
    /// Convergent (narrowing to a single solution)
    Convergent,
}

/// Cognitive metrics for self-analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveMetrics {
    pub total_decisions: u64,
    pub successful_decisions: u64,
    pub avg_confidence: f64,
    pub avg_decision_time_ms: f64,
    pub dominant_pattern: Option<ThinkingPattern>,
    pub bias_count: u64,
}

impl CognitiveMetrics {
    pub fn success_rate(&self) -> f64 {
        if self.total_decisions == 0 {
            return 0.0;
        }
        self.successful_decisions as f64 / self.total_decisions as f64
    }
}

/// Analyzes cognitive patterns in Hydra's reasoning
pub struct CognitiveAnalyzer {
    observations: parking_lot::RwLock<Vec<CognitiveObservation>>,
}

#[derive(Debug, Clone)]
struct CognitiveObservation {
    pattern: ThinkingPattern,
    confidence: f64,
    success: bool,
    duration_ms: f64,
}

impl CognitiveAnalyzer {
    pub fn new() -> Self {
        Self {
            observations: parking_lot::RwLock::new(Vec::new()),
        }
    }

    /// Record a cognitive observation
    pub fn observe(
        &self,
        pattern: ThinkingPattern,
        confidence: f64,
        success: bool,
        duration_ms: f64,
    ) {
        self.observations.write().push(CognitiveObservation {
            pattern,
            confidence,
            success,
            duration_ms,
        });
    }

    /// Compute cognitive metrics
    pub fn metrics(&self) -> CognitiveMetrics {
        let obs = self.observations.read();

        if obs.is_empty() {
            return CognitiveMetrics {
                total_decisions: 0,
                successful_decisions: 0,
                avg_confidence: 0.0,
                avg_decision_time_ms: 0.0,
                dominant_pattern: None,
                bias_count: 0,
            };
        }

        let total = obs.len() as u64;
        let successful = obs.iter().filter(|o| o.success).count() as u64;
        let avg_conf = obs.iter().map(|o| o.confidence).sum::<f64>() / obs.len() as f64;
        let avg_time = obs.iter().map(|o| o.duration_ms).sum::<f64>() / obs.len() as f64;

        // Find dominant pattern
        let mut counts = std::collections::HashMap::new();
        for o in obs.iter() {
            *counts.entry(o.pattern).or_insert(0u64) += 1;
        }
        let dominant = counts
            .into_iter()
            .max_by_key(|(_, c)| *c)
            .map(|(p, _)| p);

        // Count potential biases (high confidence + failure)
        let biases = obs
            .iter()
            .filter(|o| o.confidence > 0.8 && !o.success)
            .count() as u64;

        CognitiveMetrics {
            total_decisions: total,
            successful_decisions: successful,
            avg_confidence: avg_conf,
            avg_decision_time_ms: avg_time,
            dominant_pattern: dominant,
            bias_count: biases,
        }
    }

    /// Get the most effective thinking pattern
    pub fn most_effective_pattern(&self) -> Option<(ThinkingPattern, f64)> {
        let obs = self.observations.read();
        let mut pattern_stats: std::collections::HashMap<ThinkingPattern, (u64, u64)> =
            std::collections::HashMap::new();

        for o in obs.iter() {
            let entry = pattern_stats.entry(o.pattern).or_insert((0, 0));
            entry.0 += 1;
            if o.success {
                entry.1 += 1;
            }
        }

        pattern_stats
            .into_iter()
            .filter(|(_, (total, _))| *total >= 2) // Need minimum observations
            .map(|(pattern, (total, successes))| (pattern, successes as f64 / total as f64))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
    }

    pub fn observation_count(&self) -> usize {
        self.observations.read().len()
    }
}

impl Default for CognitiveAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cognitive_metrics() {
        let analyzer = CognitiveAnalyzer::new();
        analyzer.observe(ThinkingPattern::Systematic, 0.8, true, 100.0);
        analyzer.observe(ThinkingPattern::Systematic, 0.7, true, 150.0);
        analyzer.observe(ThinkingPattern::Intuitive, 0.6, false, 50.0);

        let metrics = analyzer.metrics();
        assert_eq!(metrics.total_decisions, 3);
        assert_eq!(metrics.successful_decisions, 2);
        assert_eq!(metrics.dominant_pattern, Some(ThinkingPattern::Systematic));
    }

    #[test]
    fn test_most_effective_pattern() {
        let analyzer = CognitiveAnalyzer::new();
        analyzer.observe(ThinkingPattern::Systematic, 0.8, true, 100.0);
        analyzer.observe(ThinkingPattern::Systematic, 0.7, true, 120.0);
        analyzer.observe(ThinkingPattern::Intuitive, 0.6, true, 50.0);
        analyzer.observe(ThinkingPattern::Intuitive, 0.5, false, 40.0);

        let (pattern, rate) = analyzer.most_effective_pattern().unwrap();
        assert_eq!(pattern, ThinkingPattern::Systematic);
        assert!((rate - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_bias_counting() {
        let analyzer = CognitiveAnalyzer::new();
        analyzer.observe(ThinkingPattern::Intuitive, 0.95, false, 30.0);
        analyzer.observe(ThinkingPattern::Intuitive, 0.90, false, 25.0);
        analyzer.observe(ThinkingPattern::Systematic, 0.5, true, 100.0);

        let metrics = analyzer.metrics();
        assert_eq!(metrics.bias_count, 2);
    }
}
