//! Metabolism report — captures a snapshot for display.

use crate::intervention::InterventionLevel;
use crate::lyapunov::StabilityClass;
use crate::monitor::MetabolismMonitor;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A snapshot report of the metabolism monitor state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetabolismReport {
    /// When this report was captured.
    pub captured_at: DateTime<Utc>,
    /// Current Lyapunov value.
    pub lyapunov_value: Option<f64>,
    /// Current stability classification.
    pub stability: Option<StabilityClass>,
    /// Current intervention level.
    pub intervention_level: InterventionLevel,
    /// Trend direction (positive = improving).
    pub trend: Option<f64>,
    /// Mean Lyapunov over the history window.
    pub mean: Option<f64>,
    /// Total ticks processed.
    pub tick_count: u64,
    /// Total interventions triggered.
    pub intervention_count: usize,
}

impl MetabolismReport {
    /// Capture a report from the current monitor state.
    pub fn capture(monitor: &MetabolismMonitor) -> Self {
        let tracker = monitor.tracker();
        Self {
            captured_at: Utc::now(),
            lyapunov_value: tracker.current(),
            stability: tracker.stability(),
            intervention_level: monitor.current_level(),
            trend: tracker.trend(),
            mean: tracker.mean(),
            tick_count: monitor.tick_count(),
            intervention_count: monitor.interventions().len(),
        }
    }

    /// Produce a single-line status for TUI display.
    pub fn status_line(&self) -> String {
        let stability = self
            .stability
            .map(|s| s.to_string())
            .unwrap_or_else(|| "UNKNOWN".to_string());
        let lyapunov = self
            .lyapunov_value
            .map(|v| format!("{v:.4}"))
            .unwrap_or_else(|| "N/A".to_string());
        let trend_str = self
            .trend
            .map(|t| {
                if t > 0.0 {
                    "UP".to_string()
                } else if t < 0.0 {
                    "DOWN".to_string()
                } else {
                    "FLAT".to_string()
                }
            })
            .unwrap_or_else(|| "N/A".to_string());

        format!(
            "[metabolism] {} V(Psi)={} trend={} ticks={} interventions={}",
            stability, lyapunov, trend_str, self.tick_count, self.intervention_count
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitor::MetabolismMonitor;

    #[test]
    fn report_empty_monitor() {
        let m = MetabolismMonitor::new();
        let r = MetabolismReport::capture(&m);
        assert!(r.lyapunov_value.is_none());
        assert_eq!(r.tick_count, 0);
    }

    #[test]
    fn report_after_ticks() {
        let mut m = MetabolismMonitor::new();
        m.tick(0.5, 0.01).expect("tick");
        m.tick(0.4, 0.01).expect("tick");
        let r = MetabolismReport::capture(&m);
        assert_eq!(r.lyapunov_value, Some(0.4));
        assert_eq!(r.tick_count, 2);
        assert_eq!(r.stability, Some(StabilityClass::Optimal));
    }

    #[test]
    fn status_line_contains_fields() {
        let mut m = MetabolismMonitor::new();
        m.tick(0.5, 0.01).expect("tick");
        let r = MetabolismReport::capture(&m);
        let line = r.status_line();
        assert!(line.contains("metabolism"));
        assert!(line.contains("OPTIMAL"));
    }
}
