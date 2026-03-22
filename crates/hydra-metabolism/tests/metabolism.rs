//! Integration tests for hydra-metabolism.

use hydra_metabolism::intervention::InterventionLevel;
use hydra_metabolism::lyapunov::{classify, LyapunovTracker, StabilityClass};
use hydra_metabolism::monitor::MetabolismMonitor;
use hydra_metabolism::report::MetabolismReport;

#[test]
fn full_lifecycle_healthy() {
    let mut m = MetabolismMonitor::new();
    for i in 0..50 {
        let v = 0.5 + (i as f64 * 0.001);
        let level = m.tick(v, 0.01).expect("tick");
        assert_eq!(level, InterventionLevel::None);
    }
    assert_eq!(m.tick_count(), 50);
    assert!(m.interventions().is_empty());
    assert!(m.tracker().is_consistently_stable(50));
}

#[test]
fn degradation_and_recovery() {
    let mut m = MetabolismMonitor::new();
    // Start healthy
    m.tick(0.5, 0.01).expect("tick");
    assert_eq!(m.current_level(), InterventionLevel::None);

    // Degrade to alert
    m.tick(-0.1, 0.01).expect("tick");
    assert_eq!(m.current_level(), InterventionLevel::Level1Alert);

    // Degrade further to critical
    m.tick(-0.6, 0.01).expect("tick");
    assert_eq!(m.current_level(), InterventionLevel::Level2Critical);

    // Recover
    m.tick(0.5, 0.01).expect("tick");
    assert_eq!(m.current_level(), InterventionLevel::None);
}

#[test]
fn growth_invariant_always_checked_first() {
    let mut m = MetabolismMonitor::new();
    // Even with a great Lyapunov value, negative gamma-hat is rejected
    let result = m.tick(1.0, -0.001);
    assert!(result.is_err());
}

#[test]
fn report_reflects_state() {
    let mut m = MetabolismMonitor::new();
    m.tick(0.5, 0.01).expect("tick");
    m.tick(-0.1, 0.01).expect("tick");

    let r = MetabolismReport::capture(&m);
    assert_eq!(r.tick_count, 2);
    assert_eq!(r.lyapunov_value, Some(-0.1));
    assert_eq!(r.stability, Some(StabilityClass::Alert));
    assert_eq!(r.intervention_count, 1);
}

#[test]
fn classify_all_ranges() {
    assert_eq!(classify(1.0), StabilityClass::Optimal);
    assert_eq!(classify(0.3), StabilityClass::Optimal);
    assert_eq!(classify(0.15), StabilityClass::Stable);
    assert_eq!(classify(0.0), StabilityClass::Stable);
    assert_eq!(classify(-0.25), StabilityClass::Alert);
    assert_eq!(classify(-0.5), StabilityClass::Alert);
    assert_eq!(classify(-0.51), StabilityClass::Critical);
    assert_eq!(classify(-0.75), StabilityClass::Critical);
    assert_eq!(classify(-1.0), StabilityClass::Critical);
    assert_eq!(classify(-2.0), StabilityClass::Emergency);
}

#[test]
fn tracker_mean_and_trend() {
    let mut t = LyapunovTracker::new();
    t.record(0.1);
    t.record(0.2);
    t.record(0.3);
    let mean = t.mean().expect("mean");
    assert!((mean - 0.2).abs() < 0.001);
    let trend = t.trend().expect("trend");
    assert!(trend > 0.0);
}
