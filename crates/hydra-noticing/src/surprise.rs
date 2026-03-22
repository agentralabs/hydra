//! Surprise Detection — fires when reality contradicts expectations.
//!
//! Not drift detection (change over time). Surprise detection (violation right now).
//! "You're asking about deployment but you have no rollback mechanism."
//! That is surprise. The model says rollback should exist. It does not. Signal.

use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// A single surprise event — something violated expectations.
#[derive(Debug, Clone)]
pub struct SurpriseEvent {
    pub expected: String,
    pub observed: String,
    pub magnitude: f64,
    pub domain: String,
    pub timestamp: DateTime<Utc>,
}

impl SurpriseEvent {
    pub fn summary(&self) -> String {
        format!(
            "SURPRISE (magnitude {:.1}): expected '{}', observed '{}'",
            self.magnitude, self.expected, self.observed
        )
    }
}

/// Running statistics for a single observed metric/behavior.
#[derive(Debug, Clone)]
struct RunningStats {
    count: usize,
    mean: f64,
    m2: f64, // for Welford's online variance
}

impl RunningStats {
    fn new() -> Self {
        Self {
            count: 0,
            mean: 0.0,
            m2: 0.0,
        }
    }

    /// Update with a new observation (Welford's algorithm).
    fn update(&mut self, value: f64) {
        self.count += 1;
        let delta = value - self.mean;
        self.mean += delta / self.count as f64;
        let delta2 = value - self.mean;
        self.m2 += delta * delta2;
    }

    fn std_dev(&self) -> f64 {
        if self.count < 2 {
            return f64::MAX;
        }
        (self.m2 / (self.count - 1) as f64).sqrt()
    }

    /// Z-score: how many standard deviations from the mean.
    fn z_score(&self, value: f64) -> f64 {
        let sd = self.std_dev();
        if sd < f64::EPSILON {
            return 0.0;
        }
        (value - self.mean).abs() / sd
    }
}

/// Tracks expected patterns and detects violations.
pub struct SurpriseDetector {
    /// Running statistics per domain/metric key.
    stats: HashMap<String, RunningStats>,
    /// Known expectations (categorical: what values are normal).
    categorical: HashMap<String, Vec<String>>,
    /// Detected surprise events (append-only).
    surprises: Vec<SurpriseEvent>,
}

impl SurpriseDetector {
    pub fn new() -> Self {
        Self {
            stats: HashMap::new(),
            categorical: HashMap::new(),
            surprises: Vec::new(),
        }
    }

    /// Observe a numeric value. If enough observations exist, check for surprise.
    /// Returns Some(SurpriseEvent) if the observation is surprising (z > 2.0).
    pub fn observe_numeric(
        &mut self,
        key: &str,
        value: f64,
        domain: &str,
    ) -> Option<SurpriseEvent> {
        let stats = self.stats.entry(key.to_string()).or_insert_with(RunningStats::new);
        let surprise = if stats.count >= 10 {
            let z = stats.z_score(value);
            if z > 2.0 {
                Some(SurpriseEvent {
                    expected: format!("{:.2} ± {:.2}", stats.mean, stats.std_dev()),
                    observed: format!("{:.2} (z={:.1})", value, z),
                    magnitude: z,
                    domain: domain.to_string(),
                    timestamp: Utc::now(),
                })
            } else {
                None
            }
        } else {
            None
        };
        stats.update(value);
        if let Some(ref event) = surprise {
            self.surprises.push(event.clone());
        }
        surprise
    }

    /// Observe a categorical value. Surprise if the value has never been seen before
    /// after at least 5 observations in this category.
    pub fn observe_categorical(
        &mut self,
        key: &str,
        value: &str,
        domain: &str,
    ) -> Option<SurpriseEvent> {
        let known = self
            .categorical
            .entry(key.to_string())
            .or_default();
        let is_new = !known.contains(&value.to_string());
        let surprise = if is_new && known.len() >= 5 {
            Some(SurpriseEvent {
                expected: format!("one of: {}", known.join(", ")),
                observed: value.to_string(),
                magnitude: 2.5, // categorical surprise is always significant
                domain: domain.to_string(),
                timestamp: Utc::now(),
            })
        } else {
            None
        };
        if is_new {
            known.push(value.to_string());
        }
        if let Some(ref event) = surprise {
            self.surprises.push(event.clone());
        }
        surprise
    }

    /// Observe that something expected is ABSENT.
    /// This is the most powerful form of surprise.
    /// "You are asking about deployment but have no rollback mechanism."
    pub fn observe_absence(
        &mut self,
        what_is_missing: &str,
        why_expected: &str,
        domain: &str,
    ) -> SurpriseEvent {
        let event = SurpriseEvent {
            expected: format!("{what_is_missing} (expected because: {why_expected})"),
            observed: "ABSENT".to_string(),
            magnitude: 3.0, // absence is always highly surprising
            domain: domain.to_string(),
            timestamp: Utc::now(),
        };
        self.surprises.push(event.clone());
        event
    }

    pub fn surprise_count(&self) -> usize {
        self.surprises.len()
    }

    pub fn recent_surprises(&self, n: usize) -> Vec<&SurpriseEvent> {
        self.surprises.iter().rev().take(n).collect()
    }
}

impl Default for SurpriseDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_surprise_with_few_observations() {
        let mut det = SurpriseDetector::new();
        for i in 0..5 {
            assert!(det.observe_numeric("latency", i as f64, "api").is_none());
        }
    }

    #[test]
    fn surprise_on_outlier() {
        let mut det = SurpriseDetector::new();
        // Use slightly varying values so std_dev > 0
        for i in 0..20 {
            det.observe_numeric("latency", 100.0 + (i as f64) * 0.5, "api");
        }
        let surprise = det.observe_numeric("latency", 500.0, "api");
        assert!(surprise.is_some());
        assert!(surprise.unwrap().magnitude > 2.0);
    }

    #[test]
    fn categorical_surprise() {
        let mut det = SurpriseDetector::new();
        for day in &["Mon", "Tue", "Wed", "Thu", "Fri"] {
            det.observe_categorical("deploy_day", day, "devops");
        }
        let surprise = det.observe_categorical("deploy_day", "Sun", "devops");
        assert!(surprise.is_some());
    }

    #[test]
    fn absence_is_always_surprising() {
        let mut det = SurpriseDetector::new();
        let event = det.observe_absence(
            "rollback mechanism",
            "production deployment requires rollback",
            "devops",
        );
        assert!(event.magnitude >= 3.0);
    }
}
