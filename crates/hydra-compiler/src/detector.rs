//! PatternDetector — scans action history for repeated sequences.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Configuration for pattern detection
#[derive(Debug, Clone)]
pub struct DetectorConfig {
    /// Minimum occurrences to consider a pattern
    pub min_occurrences: u32,
    /// Minimum success rate (0.0 - 1.0)
    pub min_success_rate: f64,
    /// Maximum age of entries to consider (days)
    pub max_age_days: u32,
}

impl Default for DetectorConfig {
    fn default() -> Self {
        Self {
            min_occurrences: 3,
            min_success_rate: 0.9,
            max_age_days: 7,
        }
    }
}

/// A recorded action for pattern analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRecord {
    pub action: String,
    pub tool: String,
    pub params_hash: String,
    pub success: bool,
    pub timestamp: String,
}

/// A detected repeated pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedPattern {
    pub signature: String,
    pub actions: Vec<String>,
    pub tools: Vec<String>,
    pub occurrences: u32,
    pub success_rate: f64,
    pub compilable: bool,
}

/// Detects repeated action sequences from execution history
pub struct PatternDetector {
    config: DetectorConfig,
    /// action_signature → occurrences
    signatures: parking_lot::Mutex<HashMap<String, PatternStats>>,
}

#[derive(Debug, Clone)]
struct PatternStats {
    actions: Vec<String>,
    tools: Vec<String>,
    total: u32,
    successes: u32,
    _first_seen: String,
    last_seen: String,
}

impl PatternDetector {
    pub fn new(config: DetectorConfig) -> Self {
        Self {
            config,
            signatures: parking_lot::Mutex::new(HashMap::new()),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(DetectorConfig::default())
    }

    /// Record an action sequence execution
    pub fn record(&self, signature: &str, actions: &[String], tools: &[String], success: bool) {
        let mut sigs = self.signatures.lock();
        let now = chrono::Utc::now().to_rfc3339();

        let entry = sigs
            .entry(signature.to_string())
            .or_insert_with(|| PatternStats {
                actions: actions.to_vec(),
                tools: tools.to_vec(),
                total: 0,
                successes: 0,
                _first_seen: now.clone(),
                last_seen: now.clone(),
            });

        entry.total += 1;
        if success {
            entry.successes += 1;
        }
        entry.last_seen = now;
    }

    /// Detect patterns that meet the compilation threshold
    pub fn detect(&self) -> Vec<DetectedPattern> {
        let sigs = self.signatures.lock();
        let mut patterns = Vec::new();

        for (sig, stats) in sigs.iter() {
            if stats.total < self.config.min_occurrences {
                continue;
            }

            let success_rate = stats.successes as f64 / stats.total as f64;
            if success_rate < self.config.min_success_rate {
                continue;
            }

            patterns.push(DetectedPattern {
                signature: sig.clone(),
                actions: stats.actions.clone(),
                tools: stats.tools.clone(),
                occurrences: stats.total,
                success_rate,
                compilable: true,
            });
        }

        // Sort by occurrences (most frequent first)
        patterns.sort_by(|a, b| b.occurrences.cmp(&a.occurrences));
        patterns
    }

    /// Number of tracked signatures
    pub fn signature_count(&self) -> usize {
        self.signatures.lock().len()
    }

    /// Clear all tracked patterns
    pub fn clear(&self) {
        self.signatures.lock().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_patterns_initially() {
        let detector = PatternDetector::with_defaults();
        assert!(detector.detect().is_empty());
    }

    #[test]
    fn test_pattern_detection() {
        let detector = PatternDetector::with_defaults();
        let actions = vec!["git add".into(), "git commit".into(), "git push".into()];
        let tools = vec!["git_add".into(), "git_commit".into(), "git_push".into()];

        // Record 5 successful executions
        for _ in 0..5 {
            detector.record("git-push-flow", &actions, &tools, true);
        }

        let patterns = detector.detect();
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].signature, "git-push-flow");
        assert_eq!(patterns[0].occurrences, 5);
        assert!((patterns[0].success_rate - 1.0).abs() < 0.01);
        assert!(patterns[0].compilable);
    }

    #[test]
    fn test_below_threshold() {
        let detector = PatternDetector::with_defaults();
        // Only 2 occurrences (threshold is 3)
        detector.record("rare", &["a".into()], &["t".into()], true);
        detector.record("rare", &["a".into()], &["t".into()], true);
        assert!(detector.detect().is_empty());
    }

    #[test]
    fn test_low_success_rate() {
        let detector = PatternDetector::with_defaults();
        // 3 occurrences but 33% success rate (threshold 90%)
        detector.record("flaky", &["a".into()], &["t".into()], true);
        detector.record("flaky", &["a".into()], &["t".into()], false);
        detector.record("flaky", &["a".into()], &["t".into()], false);
        assert!(detector.detect().is_empty());
    }

    #[test]
    fn test_signature_count() {
        let detector = PatternDetector::with_defaults();
        detector.record("a", &["x".into()], &["t".into()], true);
        detector.record("b", &["y".into()], &["t".into()], true);
        assert_eq!(detector.signature_count(), 2);
    }

    #[test]
    fn test_clear() {
        let detector = PatternDetector::with_defaults();
        detector.record("a", &["x".into()], &["t".into()], true);
        detector.clear();
        assert_eq!(detector.signature_count(), 0);
    }

    #[test]
    fn test_detector_config_default() {
        let config = DetectorConfig::default();
        assert_eq!(config.min_occurrences, 3);
        assert_eq!(config.min_success_rate, 0.9);
        assert_eq!(config.max_age_days, 7);
    }

    #[test]
    fn test_custom_config_lower_threshold() {
        let config = DetectorConfig { min_occurrences: 1, min_success_rate: 0.5, max_age_days: 30 };
        let detector = PatternDetector::new(config);
        detector.record("once", &["a".into()], &["t".into()], true);
        let patterns = detector.detect();
        assert_eq!(patterns.len(), 1);
    }

    #[test]
    fn test_multiple_patterns_sorted() {
        let config = DetectorConfig { min_occurrences: 1, min_success_rate: 0.0, max_age_days: 30 };
        let detector = PatternDetector::new(config);
        detector.record("rare", &["a".into()], &["t".into()], true);
        for _ in 0..5 {
            detector.record("frequent", &["b".into()], &["t".into()], true);
        }
        let patterns = detector.detect();
        assert_eq!(patterns[0].signature, "frequent"); // most frequent first
    }

    #[test]
    fn test_detected_pattern_serde() {
        let pattern = DetectedPattern {
            signature: "sig".into(),
            actions: vec!["a".into()],
            tools: vec!["t".into()],
            occurrences: 5,
            success_rate: 1.0,
            compilable: true,
        };
        let json = serde_json::to_string(&pattern).unwrap();
        let restored: DetectedPattern = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.signature, "sig");
        assert_eq!(restored.occurrences, 5);
    }

    #[test]
    fn test_action_record_serde() {
        let record = ActionRecord {
            action: "deploy".into(),
            tool: "deploy_tool".into(),
            params_hash: "abc123".into(),
            success: true,
            timestamp: "2026-01-01".into(),
        };
        let json = serde_json::to_string(&record).unwrap();
        let restored: ActionRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.action, "deploy");
    }
}
