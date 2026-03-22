//! BaselineTracker — records what is normal for each metric.
//! Drift is only meaningful relative to a baseline.

use crate::constants::DRIFT_MIN_SAMPLES;
use crate::errors::NoticingError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// One metric's baseline statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricBaseline {
    pub name:    String,
    pub samples: Vec<f64>,
    pub mean:    f64,
    pub std_dev: f64,
}

impl MetricBaseline {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name:    name.into(),
            samples: Vec::new(),
            mean:    0.0,
            std_dev: 0.0,
        }
    }

    pub fn add_sample(&mut self, value: f64) {
        self.samples.push(value);
        self.recompute();
    }

    fn recompute(&mut self) {
        if self.samples.is_empty() {
            return;
        }
        let n = self.samples.len() as f64;
        self.mean = self.samples.iter().sum::<f64>() / n;
        let var = self
            .samples
            .iter()
            .map(|x| (x - self.mean).powi(2))
            .sum::<f64>()
            / n;
        self.std_dev = var.sqrt();
    }

    pub fn has_enough_data(&self) -> bool {
        self.samples.len() >= DRIFT_MIN_SAMPLES
    }

    /// Z-score: how many std devs is this value from the mean?
    pub fn z_score(&self, value: f64) -> f64 {
        if self.std_dev < 1e-10 {
            return 0.0;
        }
        (value - self.mean) / self.std_dev
    }

    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }
}

/// Tracks baselines for multiple metrics.
#[derive(Debug, Default)]
pub struct BaselineTracker {
    metrics: HashMap<String, MetricBaseline>,
}

impl BaselineTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new metric.
    pub fn register(&mut self, name: impl Into<String>) {
        let n = name.into();
        self.metrics
            .entry(n.clone())
            .or_insert_with(|| MetricBaseline::new(n));
    }

    /// Add a sample to a metric's baseline.
    pub fn add_sample(&mut self, metric: &str, value: f64) -> Result<(), NoticingError> {
        self.metrics
            .get_mut(metric)
            .ok_or(NoticingError::MetricNotTracked {
                name: metric.to_string(),
            })?
            .add_sample(value);
        Ok(())
    }

    pub fn get(&self, metric: &str) -> Option<&MetricBaseline> {
        self.metrics.get(metric)
    }

    pub fn metric_count(&self) -> usize {
        self.metrics.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_computes_mean() {
        let mut b = MetricBaseline::new("latency");
        b.add_sample(50.0);
        b.add_sample(60.0);
        b.add_sample(55.0);
        assert!((b.mean - 55.0).abs() < 1e-10);
    }

    #[test]
    fn z_score_correct() {
        let mut b = MetricBaseline::new("latency");
        for v in [50.0, 60.0, 70.0, 80.0, 90.0] {
            b.add_sample(v);
        }
        // Mean = 70, std_dev ~ 14.14
        let z = b.z_score(100.0); // 2 std devs above
        assert!(z > 1.5);
    }

    #[test]
    fn not_enough_data() {
        let mut b = MetricBaseline::new("metric");
        b.add_sample(1.0);
        b.add_sample(2.0);
        assert!(!b.has_enough_data());
        b.add_sample(3.0);
        assert!(b.has_enough_data());
    }

    #[test]
    fn tracker_register_and_sample() {
        let mut t = BaselineTracker::new();
        t.register("latency");
        t.add_sample("latency", 50.0).unwrap();
        assert_eq!(t.get("latency").unwrap().sample_count(), 1);
    }

    #[test]
    fn unknown_metric_returns_error() {
        let mut t = BaselineTracker::new();
        let r = t.add_sample("unknown", 1.0);
        assert!(matches!(r, Err(NoticingError::MetricNotTracked { .. })));
    }
}
