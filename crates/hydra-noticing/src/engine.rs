//! NoticingEngine — the unified ambient observation coordinator.

use crate::{
    baseline::BaselineTracker,
    compound::{CompoundRiskDetector, SmallIssue},
    constants::MAX_QUEUED_SIGNALS,
    drift::{detect_drift, detect_trend, signal_from_drift},
    errors::NoticingError,
    pattern::PatternWatcher,
    signal::{NoticingKind, NoticingSignal},
};

/// The noticing engine — ambient observation without prompt.
pub struct NoticingEngine {
    pub baseline: BaselineTracker,
    pub patterns: PatternWatcher,
    pub compound: CompoundRiskDetector,
    signals:      Vec<NoticingSignal>,
    cycle_count:  u64,
}

impl NoticingEngine {
    pub fn new() -> Self {
        Self {
            baseline:    BaselineTracker::new(),
            patterns:    PatternWatcher::new(),
            compound:    CompoundRiskDetector::new(),
            signals:     Vec::new(),
            cycle_count: 0,
        }
    }

    /// Run one noticing cycle — sample all metrics and check all patterns.
    /// Call this on the NOTICING thread interval.
    pub fn cycle(&mut self) -> Vec<&NoticingSignal> {
        self.cycle_count += 1;

        // Check pattern breaks
        let pattern_signals = self.patterns.check_for_breaks();
        for s in pattern_signals {
            self.push_signal(s);
        }

        // Check compound risks
        if let Some(s) = self.compound.check_compound() {
            self.push_signal(s);
        }

        // Return all unsurfaced significant signals
        self.signals
            .iter()
            .filter(|s| !s.surfaced && s.is_significant())
            .collect()
    }

    /// Sample a metric value — triggers drift detection.
    pub fn sample_metric(
        &mut self,
        metric: &str,
        value:  f64,
    ) -> Result<(), NoticingError> {
        self.baseline.add_sample(metric, value)?;

        // Check for point drift
        if let Some(event) = detect_drift(metric, value, &self.baseline) {
            let signal = signal_from_drift(&event);
            if signal.is_significant() {
                self.push_signal(signal);
            }
        }

        // Check for sustained trend
        if let Some((direction, magnitude)) =
            detect_trend(metric, &self.baseline)
        {
            if magnitude > crate::constants::DRIFT_THRESHOLD_FRACTION {
                let significance = (magnitude
                    * crate::constants::TREND_SIGNIFICANCE_MULTIPLIER)
                    .min(1.0);
                let signal = NoticingSignal::new(
                    NoticingKind::MetricDrift {
                        metric:    metric.to_string(),
                        direction: direction.clone(),
                        magnitude,
                        weeks:     1,
                    },
                    significance,
                    format!(
                        "Noticed sustained trend: {} consistently {} from baseline ({:.0}% drift).",
                        metric,
                        direction.label(),
                        magnitude * 100.0,
                    ),
                    Some(format!(
                        "Monitor {} closely — sustained trend detected.",
                        metric
                    )),
                );
                self.push_signal(signal);
            }
        }

        Ok(())
    }

    /// Register a metric for baseline tracking.
    pub fn register_metric(&mut self, name: &str) {
        self.baseline.register(name);
    }

    /// Watch a recurring pattern.
    pub fn watch_pattern(&mut self, name: &str, expected_interval_days: f64) {
        self.patterns.watch(name, expected_interval_days);
    }

    /// Record a pattern occurrence.
    pub fn record_pattern(&mut self, name: &str) {
        self.patterns.record(name);
    }

    /// Add a small issue for compound risk detection.
    pub fn add_issue(&mut self, issue: SmallIssue) {
        self.compound.add_issue(issue);
    }

    /// Mark a signal as surfaced.
    pub fn mark_surfaced(&mut self, signal_id: &str) {
        if let Some(s) = self.signals.iter_mut().find(|s| s.id == signal_id) {
            s.mark_surfaced();
        }
    }

    /// All unsurfaced significant signals.
    pub fn pending_signals(&self) -> Vec<&NoticingSignal> {
        self.signals
            .iter()
            .filter(|s| !s.surfaced && s.is_significant())
            .collect()
    }

    fn push_signal(&mut self, signal: NoticingSignal) {
        // Dedup: skip if same kind already pending
        let already_pending = self.signals.iter().any(|s| {
            !s.surfaced
                && std::mem::discriminant(&s.kind)
                    == std::mem::discriminant(&signal.kind)
        });
        if already_pending {
            return;
        }

        if self.signals.len() < MAX_QUEUED_SIGNALS {
            self.signals.push(signal);
        }
    }

    pub fn signal_count(&self) -> usize {
        self.signals.len()
    }

    pub fn cycle_count(&self) -> u64 {
        self.cycle_count
    }

    /// TUI summary.
    pub fn summary(&self) -> String {
        let pending = self.pending_signals().len();
        format!(
            "noticing: cycles={} signals={} pending={}",
            self.cycle_count,
            self.signals.len(),
            pending,
        )
    }
}

impl Default for NoticingEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drift_generates_signal() {
        let mut engine = NoticingEngine::new();
        engine.register_metric("latency");
        // Establish baseline
        for v in [50.0, 52.0, 48.0, 51.0, 50.0] {
            engine.sample_metric("latency", v).unwrap();
        }
        // Large deviation
        engine.sample_metric("latency", 120.0).unwrap();
        assert!(engine.signal_count() > 0);
    }

    #[test]
    fn compound_risk_generates_signal() {
        let mut engine = NoticingEngine::new();
        for i in 0..crate::constants::COMPOUND_RISK_THRESHOLD {
            engine.add_issue(SmallIssue::new(
                format!("issue {}", i),
                "engineering",
                0.7,
            ));
        }
        engine.cycle();
        assert!(engine.signal_count() > 0);
    }

    #[test]
    fn cycle_count_increments() {
        let mut engine = NoticingEngine::new();
        assert_eq!(engine.cycle_count(), 0);
        engine.cycle();
        engine.cycle();
        assert_eq!(engine.cycle_count(), 2);
    }

    #[test]
    fn summary_format() {
        let engine = NoticingEngine::new();
        let s = engine.summary();
        assert!(s.contains("noticing:"));
        assert!(s.contains("cycles="));
        assert!(s.contains("pending="));
    }

    #[test]
    fn mark_surfaced_removes_from_pending() {
        let mut engine = NoticingEngine::new();
        engine.register_metric("latency");
        for v in [50.0, 51.0, 50.0, 49.0, 51.0] {
            engine.sample_metric("latency", v).unwrap();
        }
        engine.sample_metric("latency", 200.0).unwrap(); // big drift
        let pending_before = engine.pending_signals().len();
        if pending_before > 0 {
            let id = engine.pending_signals()[0].id.clone();
            engine.mark_surfaced(&id);
            assert_eq!(engine.pending_signals().len(), pending_before - 1);
        }
    }
}
