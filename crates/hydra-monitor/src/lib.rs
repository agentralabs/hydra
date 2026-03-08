// hydra-monitor: System health monitoring and metrics

use std::collections::HashMap;
use std::time::Instant;

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

/// System monitor
pub struct SystemMonitor {
    metrics: HashMap<String, MetricSummary>,
    alerts: Vec<Alert>,
    health_checks: HashMap<String, HealthCheck>,
    started_at: Instant,
}

impl SystemMonitor {
    pub fn new() -> Self {
        Self {
            metrics: HashMap::new(),
            alerts: Vec::new(),
            health_checks: HashMap::new(),
            started_at: Instant::now(),
        }
    }

    pub fn record_metric(&mut self, name: &str, value: f64) {
        self.metrics
            .entry(name.into())
            .or_default()
            .record(value);
    }

    pub fn get_metric(&self, name: &str) -> Option<&MetricSummary> {
        self.metrics.get(name)
    }

    pub fn fire_alert(&mut self, alert: Alert) {
        self.alerts.push(alert);
    }

    pub fn active_alerts(&self) -> Vec<&Alert> {
        self.alerts.iter().filter(|a| !a.resolved).collect()
    }

    pub fn resolve_alert(&mut self, id: &str) -> bool {
        if let Some(alert) = self.alerts.iter_mut().find(|a| a.id == id) {
            alert.resolved = true;
            return true;
        }
        false
    }

    pub fn set_health(&mut self, component: &str, status: HealthCheck) {
        self.health_checks.insert(component.into(), status);
    }

    pub fn get_health(&self, component: &str) -> HealthCheck {
        self.health_checks
            .get(component)
            .copied()
            .unwrap_or(HealthCheck::Unknown)
    }

    pub fn overall_health(&self) -> HealthCheck {
        if self.health_checks.is_empty() {
            return HealthCheck::Unknown;
        }
        if self.health_checks.values().any(|h| h.is_error()) {
            return HealthCheck::Unhealthy;
        }
        if self
            .health_checks
            .values()
            .any(|h| matches!(h, HealthCheck::Degraded))
        {
            return HealthCheck::Degraded;
        }
        HealthCheck::Healthy
    }

    pub fn uptime_secs(&self) -> u64 {
        self.started_at.elapsed().as_secs()
    }

    pub fn metric_names(&self) -> Vec<String> {
        self.metrics.keys().cloned().collect()
    }

    pub fn alert_count(&self) -> usize {
        self.alerts.len()
    }

    pub fn component_count(&self) -> usize {
        self.health_checks.len()
    }
}

impl Default for SystemMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── MetricType tests ───────────────────────────────────

    #[test]
    fn test_metric_type_as_str() {
        assert_eq!(MetricType::Counter.as_str(), "counter");
        assert_eq!(MetricType::Gauge.as_str(), "gauge");
        assert_eq!(MetricType::Histogram.as_str(), "histogram");
        assert_eq!(MetricType::Timer.as_str(), "timer");
    }

    // ── HealthCheck tests ──────────────────────────────────

    #[test]
    fn test_health_check_as_str() {
        assert_eq!(HealthCheck::Healthy.as_str(), "healthy");
        assert_eq!(HealthCheck::Degraded.as_str(), "degraded");
        assert_eq!(HealthCheck::Unhealthy.as_str(), "unhealthy");
        assert_eq!(HealthCheck::Unknown.as_str(), "unknown");
    }

    #[test]
    fn test_health_check_is_ok() {
        assert!(HealthCheck::Healthy.is_ok());
        assert!(!HealthCheck::Degraded.is_ok());
        assert!(!HealthCheck::Unhealthy.is_ok());
        assert!(!HealthCheck::Unknown.is_ok());
    }

    #[test]
    fn test_health_check_is_error() {
        assert!(!HealthCheck::Healthy.is_error());
        assert!(!HealthCheck::Degraded.is_error());
        assert!(HealthCheck::Unhealthy.is_error());
        assert!(!HealthCheck::Unknown.is_error());
    }

    // ── AlertSeverity tests ────────────────────────────────

    #[test]
    fn test_alert_severity_as_str() {
        assert_eq!(AlertSeverity::Info.as_str(), "info");
        assert_eq!(AlertSeverity::Warning.as_str(), "warning");
        assert_eq!(AlertSeverity::Error.as_str(), "error");
        assert_eq!(AlertSeverity::Critical.as_str(), "critical");
    }

    #[test]
    fn test_alert_severity_ordering() {
        assert!(AlertSeverity::Critical > AlertSeverity::Error);
        assert!(AlertSeverity::Error > AlertSeverity::Warning);
        assert!(AlertSeverity::Warning > AlertSeverity::Info);
    }

    // ── MetricSummary tests ────────────────────────────────

    #[test]
    fn test_metric_summary_new() {
        let s = MetricSummary::new();
        assert_eq!(s.count, 0);
        assert!(s.is_empty());
        assert_eq!(s.avg(), 0.0);
    }

    #[test]
    fn test_metric_summary_default() {
        let s = MetricSummary::default();
        assert!(s.is_empty());
    }

    #[test]
    fn test_metric_summary_record() {
        let mut s = MetricSummary::new();
        s.record(10.0);
        s.record(20.0);
        s.record(30.0);
        assert_eq!(s.count, 3);
        assert_eq!(s.sum, 60.0);
        assert_eq!(s.min, 10.0);
        assert_eq!(s.max, 30.0);
        assert!((s.avg() - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_metric_summary_single_value() {
        let mut s = MetricSummary::new();
        s.record(42.0);
        assert_eq!(s.min, 42.0);
        assert_eq!(s.max, 42.0);
        assert_eq!(s.avg(), 42.0);
    }

    // ── SystemMonitor tests ────────────────────────────────

    #[test]
    fn test_monitor_new() {
        let m = SystemMonitor::new();
        assert_eq!(m.alert_count(), 0);
        assert_eq!(m.component_count(), 0);
    }

    #[test]
    fn test_monitor_default() {
        let m = SystemMonitor::default();
        assert_eq!(m.alert_count(), 0);
    }

    #[test]
    fn test_monitor_record_metric() {
        let mut m = SystemMonitor::new();
        m.record_metric("cpu_usage", 45.0);
        m.record_metric("cpu_usage", 55.0);
        let metric = m.get_metric("cpu_usage").unwrap();
        assert_eq!(metric.count, 2);
        assert_eq!(metric.avg(), 50.0);
    }

    #[test]
    fn test_monitor_get_metric_nonexistent() {
        let m = SystemMonitor::new();
        assert!(m.get_metric("nonexistent").is_none());
    }

    #[test]
    fn test_monitor_fire_alert() {
        let mut m = SystemMonitor::new();
        m.fire_alert(Alert {
            id: "a1".into(),
            severity: AlertSeverity::Warning,
            message: "High CPU".into(),
            source: "monitor".into(),
            timestamp: "now".into(),
            resolved: false,
        });
        assert_eq!(m.alert_count(), 1);
        assert_eq!(m.active_alerts().len(), 1);
    }

    #[test]
    fn test_monitor_resolve_alert() {
        let mut m = SystemMonitor::new();
        m.fire_alert(Alert {
            id: "a1".into(),
            severity: AlertSeverity::Error,
            message: "Disk full".into(),
            source: "disk".into(),
            timestamp: "now".into(),
            resolved: false,
        });
        assert!(m.resolve_alert("a1"));
        assert!(m.active_alerts().is_empty());
    }

    #[test]
    fn test_monitor_resolve_nonexistent() {
        let mut m = SystemMonitor::new();
        assert!(!m.resolve_alert("none"));
    }

    #[test]
    fn test_monitor_set_health() {
        let mut m = SystemMonitor::new();
        m.set_health("db", HealthCheck::Healthy);
        assert_eq!(m.get_health("db"), HealthCheck::Healthy);
    }

    #[test]
    fn test_monitor_get_health_unknown() {
        let m = SystemMonitor::new();
        assert_eq!(m.get_health("anything"), HealthCheck::Unknown);
    }

    #[test]
    fn test_monitor_overall_health_empty() {
        let m = SystemMonitor::new();
        assert_eq!(m.overall_health(), HealthCheck::Unknown);
    }

    #[test]
    fn test_monitor_overall_health_all_healthy() {
        let mut m = SystemMonitor::new();
        m.set_health("db", HealthCheck::Healthy);
        m.set_health("cache", HealthCheck::Healthy);
        assert_eq!(m.overall_health(), HealthCheck::Healthy);
    }

    #[test]
    fn test_monitor_overall_health_degraded() {
        let mut m = SystemMonitor::new();
        m.set_health("db", HealthCheck::Healthy);
        m.set_health("cache", HealthCheck::Degraded);
        assert_eq!(m.overall_health(), HealthCheck::Degraded);
    }

    #[test]
    fn test_monitor_overall_health_unhealthy() {
        let mut m = SystemMonitor::new();
        m.set_health("db", HealthCheck::Unhealthy);
        m.set_health("cache", HealthCheck::Healthy);
        assert_eq!(m.overall_health(), HealthCheck::Unhealthy);
    }

    #[test]
    fn test_monitor_metric_names() {
        let mut m = SystemMonitor::new();
        m.record_metric("a", 1.0);
        m.record_metric("b", 2.0);
        let names = m.metric_names();
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn test_monitor_component_count() {
        let mut m = SystemMonitor::new();
        m.set_health("a", HealthCheck::Healthy);
        m.set_health("b", HealthCheck::Healthy);
        assert_eq!(m.component_count(), 2);
    }

    #[test]
    fn test_monitor_uptime() {
        let m = SystemMonitor::new();
        // Just verify it doesn't panic
        let _ = m.uptime_secs();
    }

    #[test]
    fn test_multiple_alerts_mixed_resolved() {
        let mut m = SystemMonitor::new();
        m.fire_alert(Alert {
            id: "a1".into(),
            severity: AlertSeverity::Info,
            message: "msg1".into(),
            source: "s".into(),
            timestamp: "now".into(),
            resolved: false,
        });
        m.fire_alert(Alert {
            id: "a2".into(),
            severity: AlertSeverity::Critical,
            message: "msg2".into(),
            source: "s".into(),
            timestamp: "now".into(),
            resolved: false,
        });
        m.resolve_alert("a1");
        assert_eq!(m.active_alerts().len(), 1);
        assert_eq!(m.active_alerts()[0].id, "a2");
    }
}
