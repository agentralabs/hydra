// hydra-monitor: Metric and alert types

use std::collections::HashMap;

/// A metric data point
#[derive(Debug, Clone)]
pub struct MetricPoint {
    pub name: String,
    pub value: f64,
    pub timestamp: String,
    pub tags: HashMap<String, String>,
}

/// Metric type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
    Timer,
}

impl MetricType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Counter => "counter",
            Self::Gauge => "gauge",
            Self::Histogram => "histogram",
            Self::Timer => "timer",
        }
    }
}

/// System health check result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthCheck {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

impl HealthCheck {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
            Self::Unhealthy => "unhealthy",
            Self::Unknown => "unknown",
        }
    }

    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Healthy)
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Self::Unhealthy)
    }
}

/// Alert severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

impl AlertSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Critical => "critical",
        }
    }
}

/// An alert fired by the monitor
#[derive(Debug, Clone)]
pub struct Alert {
    pub id: String,
    pub severity: AlertSeverity,
    pub message: String,
    pub source: String,
    pub timestamp: String,
    pub resolved: bool,
}

/// Metric counter that tracks min/max/avg
#[derive(Debug, Clone)]
pub struct MetricSummary {
    pub count: u64,
    pub sum: f64,
    pub min: f64,
    pub max: f64,
}

impl MetricSummary {
    pub fn new() -> Self {
        Self {
            count: 0,
            sum: 0.0,
            min: f64::MAX,
            max: f64::MIN,
        }
    }

    pub fn record(&mut self, value: f64) {
        self.count += 1;
        self.sum += value;
        if value < self.min {
            self.min = value;
        }
        if value > self.max {
            self.max = value;
        }
    }

    pub fn avg(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.sum / self.count as f64
        }
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

impl Default for MetricSummary {
    fn default() -> Self {
        Self::new()
    }
}
