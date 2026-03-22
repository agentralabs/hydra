//! DriftDetector — detects deviation from baseline.
//! Both point drift (single large deviation) and trend drift (sustained).

use crate::{
    baseline::BaselineTracker,
    constants::*,
    signal::{DriftDirection, NoticingKind, NoticingSignal},
};

/// A detected drift event.
#[derive(Debug, Clone)]
pub struct DriftEvent {
    pub metric:    String,
    pub current:   f64,
    pub baseline:  f64,
    pub z_score:   f64,
    pub direction: DriftDirection,
    pub magnitude: f64, // fractional change from baseline
}

/// Detect drift for a metric value against its baseline.
pub fn detect_drift(
    metric:   &str,
    current:  f64,
    baseline: &BaselineTracker,
) -> Option<DriftEvent> {
    let b = baseline.get(metric)?;
    if !b.has_enough_data() {
        return None;
    }

    let z = b.z_score(current);
    let magnitude = if b.mean.abs() < 1e-10 {
        0.0
    } else {
        ((current - b.mean) / b.mean).abs()
    };

    // Only report if magnitude exceeds threshold
    if magnitude < DRIFT_THRESHOLD_FRACTION {
        return None;
    }

    let direction = if current > b.mean {
        DriftDirection::Increasing
    } else {
        DriftDirection::Decreasing
    };

    Some(DriftEvent {
        metric: metric.to_string(),
        current,
        baseline: b.mean,
        z_score: z,
        direction,
        magnitude,
    })
}

/// Detect a sustained trend across the sample window.
/// Returns true if all recent samples are consistently above or below baseline.
pub fn detect_trend(
    metric:   &str,
    baseline: &BaselineTracker,
) -> Option<(DriftDirection, f64)> {
    let b = baseline.get(metric)?;
    if b.samples.len() < TREND_WINDOW_SIZE {
        return None;
    }

    let window = &b.samples[b.samples.len() - TREND_WINDOW_SIZE..];
    let all_above = window.iter().all(|&v| v > b.mean);
    let all_below = window.iter().all(|&v| v < b.mean);

    if all_above {
        let avg_above = window.iter().sum::<f64>() / window.len() as f64;
        let magnitude = if b.mean.abs() < 1e-10 {
            0.0
        } else {
            (avg_above - b.mean) / b.mean
        };
        Some((DriftDirection::Increasing, magnitude))
    } else if all_below {
        let avg_below = window.iter().sum::<f64>() / window.len() as f64;
        let magnitude = if b.mean.abs() < 1e-10 {
            0.0
        } else {
            (b.mean - avg_below) / b.mean
        };
        Some((DriftDirection::Decreasing, magnitude))
    } else {
        None
    }
}

/// Generate a NoticingSignal from a drift event.
pub fn signal_from_drift(event: &DriftEvent) -> NoticingSignal {
    let significance = (event.magnitude * 2.0).min(1.0);
    let narrative = format!(
        "Noticed: {} is {} {:.0}% from baseline (z={:.1}). \
         Current: {:.2}, baseline: {:.2}.",
        event.metric,
        event.direction.label(),
        event.magnitude * 100.0,
        event.z_score,
        event.current,
        event.baseline,
    );
    let action_hint = Some(format!(
        "Investigate {} — drifting {} from expected range.",
        event.metric,
        event.direction.label(),
    ));
    NoticingSignal::new(
        NoticingKind::MetricDrift {
            metric:    event.metric.clone(),
            direction: event.direction.clone(),
            magnitude: event.magnitude,
            weeks:     0, // set by caller if trend
        },
        significance,
        narrative,
        action_hint,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_baseline(name: &str, samples: &[f64]) -> BaselineTracker {
        let mut t = BaselineTracker::new();
        t.register(name);
        for &v in samples {
            t.add_sample(name, v).unwrap();
        }
        t
    }

    #[test]
    fn large_deviation_detected() {
        let t = build_baseline("latency", &[50.0, 55.0, 52.0, 48.0, 53.0]);
        let event = detect_drift("latency", 100.0, &t);
        assert!(event.is_some());
        let e = event.unwrap();
        assert_eq!(e.direction, DriftDirection::Increasing);
        assert!(e.magnitude > DRIFT_THRESHOLD_FRACTION);
    }

    #[test]
    fn small_deviation_not_detected() {
        let t = build_baseline("latency", &[50.0, 55.0, 52.0, 48.0, 53.0]);
        let event = detect_drift("latency", 52.0, &t);
        // Within 10% threshold — not detected
        assert!(event.is_none());
    }

    #[test]
    fn sustained_trend_detected() {
        let mut t = BaselineTracker::new();
        t.register("memory");
        // First 5 samples for baseline
        for v in [100.0, 100.0, 100.0, 100.0, 100.0] {
            t.add_sample("memory", v).unwrap();
        }
        // Then 5 consistently high samples
        for v in [150.0, 155.0, 160.0, 158.0, 162.0] {
            t.add_sample("memory", v).unwrap();
        }
        let trend = detect_trend("memory", &t);
        assert!(trend.is_some());
        assert_eq!(trend.unwrap().0, DriftDirection::Increasing);
    }

    #[test]
    fn insufficient_baseline_no_drift() {
        let t = build_baseline("latency", &[50.0, 55.0]); // below DRIFT_MIN_SAMPLES
        let event = detect_drift("latency", 100.0, &t);
        assert!(event.is_none());
    }
}
