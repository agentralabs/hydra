//! MetricsCollector — counters, gauges, and histograms in Prometheus format.

use std::collections::HashMap;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Metric type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
}

/// A metric with labels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub name: String,
    pub metric_type: MetricType,
    pub help: String,
    pub labels: HashMap<String, String>,
    pub value: f64,
}

/// Histogram data
#[derive(Debug, Clone, Default)]
struct HistogramData {
    values: Vec<f64>,
    sum: f64,
    count: u64,
    buckets: Vec<(f64, u64)>, // (upper_bound, count)
}

/// Metrics collector with Prometheus-compatible export
pub struct MetricsCollector {
    counters: RwLock<HashMap<String, f64>>,
    gauges: RwLock<HashMap<String, f64>>,
    histograms: RwLock<HashMap<String, HistogramData>>,
    help_texts: RwLock<HashMap<String, String>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            counters: RwLock::new(HashMap::new()),
            gauges: RwLock::new(HashMap::new()),
            histograms: RwLock::new(HashMap::new()),
            help_texts: RwLock::new(HashMap::new()),
        }
    }

    /// Register a metric with help text
    pub fn register(&self, name: &str, help: &str) {
        self.help_texts.write().insert(name.into(), help.into());
    }

    // --- Counters ---

    /// Increment a counter by 1
    pub fn counter_inc(&self, name: &str) {
        self.counter_add(name, 1.0);
    }

    /// Add to a counter
    pub fn counter_add(&self, name: &str, value: f64) {
        let mut counters = self.counters.write();
        *counters.entry(name.into()).or_insert(0.0) += value;
    }

    /// Get counter value
    pub fn counter_get(&self, name: &str) -> f64 {
        self.counters.read().get(name).copied().unwrap_or(0.0)
    }

    // --- Gauges ---

    /// Set a gauge value
    pub fn gauge_set(&self, name: &str, value: f64) {
        self.gauges.write().insert(name.into(), value);
    }

    /// Increment a gauge by delta
    pub fn gauge_inc(&self, name: &str, delta: f64) {
        let mut gauges = self.gauges.write();
        *gauges.entry(name.into()).or_insert(0.0) += delta;
    }

    /// Get gauge value
    pub fn gauge_get(&self, name: &str) -> f64 {
        self.gauges.read().get(name).copied().unwrap_or(0.0)
    }

    // --- Histograms ---

    /// Observe a value in a histogram
    pub fn histogram_observe(&self, name: &str, value: f64) {
        let mut histograms = self.histograms.write();
        let data = histograms.entry(name.into()).or_default();
        data.values.push(value);
        data.sum += value;
        data.count += 1;

        // Update default buckets
        if data.buckets.is_empty() {
            data.buckets = default_buckets();
        }
        for bucket in &mut data.buckets {
            if value <= bucket.0 {
                bucket.1 += 1;
            }
        }
    }

    /// Get histogram summary
    pub fn histogram_summary(&self, name: &str) -> Option<HistogramSummary> {
        let histograms = self.histograms.read();
        histograms.get(name).map(|data| {
            let min = data.values.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = data
                .values
                .iter()
                .cloned()
                .fold(f64::NEG_INFINITY, f64::max);
            let avg = if data.count > 0 {
                data.sum / data.count as f64
            } else {
                0.0
            };

            HistogramSummary {
                count: data.count,
                sum: data.sum,
                min: if min.is_infinite() { 0.0 } else { min },
                max: if max.is_infinite() { 0.0 } else { max },
                avg,
            }
        })
    }

    /// Export all metrics in Prometheus text format
    pub fn export_prometheus(&self) -> String {
        let mut output = String::new();
        let help_texts = self.help_texts.read();

        // Counters
        for (name, value) in self.counters.read().iter() {
            if let Some(help) = help_texts.get(name) {
                output.push_str(&format!("# HELP {} {}\n", name, help));
            }
            output.push_str(&format!("# TYPE {} counter\n", name));
            output.push_str(&format!("{} {}\n", name, value));
        }

        // Gauges
        for (name, value) in self.gauges.read().iter() {
            if let Some(help) = help_texts.get(name) {
                output.push_str(&format!("# HELP {} {}\n", name, help));
            }
            output.push_str(&format!("# TYPE {} gauge\n", name));
            output.push_str(&format!("{} {}\n", name, value));
        }

        // Histograms
        for (name, data) in self.histograms.read().iter() {
            if let Some(help) = help_texts.get(name) {
                output.push_str(&format!("# HELP {} {}\n", name, help));
            }
            output.push_str(&format!("# TYPE {} histogram\n", name));
            for (bound, count) in &data.buckets {
                output.push_str(&format!("{}_bucket{{le=\"{}\"}} {}\n", name, bound, count));
            }
            output.push_str(&format!("{}_sum {}\n", name, data.sum));
            output.push_str(&format!("{}_count {}\n", name, data.count));
        }

        output
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary of a histogram
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramSummary {
    pub count: u64,
    pub sum: f64,
    pub min: f64,
    pub max: f64,
    pub avg: f64,
}

/// Default histogram buckets (in seconds)
fn default_buckets() -> Vec<(f64, u64)> {
    vec![
        (0.005, 0),
        (0.01, 0),
        (0.025, 0),
        (0.05, 0),
        (0.1, 0),
        (0.25, 0),
        (0.5, 0),
        (1.0, 0),
        (2.5, 0),
        (5.0, 0),
        (10.0, 0),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter() {
        let m = MetricsCollector::new();
        m.counter_inc("hydra_runs_total");
        m.counter_inc("hydra_runs_total");
        m.counter_add("hydra_runs_total", 3.0);
        assert_eq!(m.counter_get("hydra_runs_total"), 5.0);
    }

    #[test]
    fn test_gauge() {
        let m = MetricsCollector::new();
        m.gauge_set("hydra_active_runs", 3.0);
        assert_eq!(m.gauge_get("hydra_active_runs"), 3.0);
        m.gauge_inc("hydra_active_runs", -1.0);
        assert_eq!(m.gauge_get("hydra_active_runs"), 2.0);
    }

    #[test]
    fn test_histogram() {
        let m = MetricsCollector::new();
        m.histogram_observe("hydra_run_duration_seconds", 0.15);
        m.histogram_observe("hydra_run_duration_seconds", 0.25);
        m.histogram_observe("hydra_run_duration_seconds", 1.5);

        let summary = m.histogram_summary("hydra_run_duration_seconds").unwrap();
        assert_eq!(summary.count, 3);
        assert!((summary.min - 0.15).abs() < f64::EPSILON);
        assert!((summary.max - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_prometheus_export() {
        let m = MetricsCollector::new();
        m.register("hydra_runs_total", "Total cognitive runs");
        m.counter_inc("hydra_runs_total");
        m.gauge_set("hydra_active_runs", 2.0);

        let output = m.export_prometheus();
        assert!(output.contains("# TYPE hydra_runs_total counter"));
        assert!(output.contains("hydra_runs_total 1"));
        assert!(output.contains("# TYPE hydra_active_runs gauge"));
        assert!(output.contains("hydra_active_runs 2"));
    }
}
