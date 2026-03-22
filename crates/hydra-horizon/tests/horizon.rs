//! Integration tests for hydra-horizon.

use hydra_horizon::constants::{ACTION_HORIZON_INITIAL, HORIZON_MAX, PERCEPTION_HORIZON_INITIAL};
use hydra_horizon::{ActionExpansion, ActionHorizon, Horizon, PerceptionExpansion, PerceptionHorizon};

#[test]
fn perception_starts_at_initial() {
    let ph = PerceptionHorizon::new();
    assert!((ph.value() - PERCEPTION_HORIZON_INITIAL).abs() < f64::EPSILON);
}

#[test]
fn action_starts_at_initial() {
    let ah = ActionHorizon::new();
    assert!((ah.value() - ACTION_HORIZON_INITIAL).abs() < f64::EPSILON);
}

#[test]
fn perception_expands_with_genome() {
    let mut ph = PerceptionHorizon::new();
    let delta = ph
        .expand(PerceptionExpansion::GenomeEntry { count_delta: 200 })
        .expect("expand");
    // 200 * 0.0001 = 0.02
    assert!((delta - 0.02).abs() < 1e-10);
}

#[test]
fn action_expands_with_synthesis() {
    let mut ah = ActionHorizon::new();
    let delta = ah.expand(ActionExpansion::CapabilitySynthesized {
        name: "test".into(),
    });
    assert!(delta > 0.0);
}

#[test]
fn perception_never_exceeds_max() {
    let mut ph = PerceptionHorizon::new();
    for _ in 0..10_000 {
        let _ = ph.expand(PerceptionExpansion::SisterConnected {
            sister_name: "s".into(),
        });
    }
    assert!(ph.value() <= HORIZON_MAX);
    assert!((ph.value() - HORIZON_MAX).abs() < f64::EPSILON);
}

#[test]
fn action_never_exceeds_max() {
    let mut ah = ActionHorizon::new();
    for _ in 0..10_000 {
        ah.expand(ActionExpansion::CapabilitySynthesized {
            name: "x".into(),
        });
    }
    assert!(ah.value() <= HORIZON_MAX);
    assert!((ah.value() - HORIZON_MAX).abs() < f64::EPSILON);
}

#[test]
fn combined_is_geometric_mean() {
    let mut h = Horizon::new();
    h.expand_perception(PerceptionExpansion::SisterConnected {
        sister_name: "memory".into(),
    })
    .unwrap();
    h.expand_action(ActionExpansion::CapabilitySynthesized {
        name: "test".into(),
    });
    let expected = (h.perception.value * h.action.value).sqrt();
    assert!((h.combined() - expected).abs() < 1e-10);
}

#[test]
fn status_line_contains_all_fields() {
    let h = Horizon::new();
    let line = h.status_line();
    assert!(line.contains("perception="));
    assert!(line.contains("action="));
    assert!(line.contains("combined="));
}

#[test]
fn multiple_perception_expansions_accumulate() {
    let mut h = Horizon::new();
    let d1 = h
        .expand_perception(PerceptionExpansion::SystemMapped {
            system_name: "fs".into(),
        })
        .unwrap();
    let d2 = h
        .expand_perception(PerceptionExpansion::SisterConnected {
            sister_name: "forge".into(),
        })
        .unwrap();
    let expected = PERCEPTION_HORIZON_INITIAL + d1 + d2;
    assert!((h.perception.value - expected).abs() < 1e-10);
}

#[test]
fn perception_tracking_counters() {
    let mut ph = PerceptionHorizon::new();
    ph.expand(PerceptionExpansion::GenomeEntry { count_delta: 10 })
        .unwrap();
    assert_eq!(ph.genome_entries, 10);
    ph.expand(PerceptionExpansion::SystemMapped {
        system_name: "db".into(),
    })
    .unwrap();
    assert_eq!(ph.systems_mapped, 1);
    ph.expand(PerceptionExpansion::SisterConnected {
        sister_name: "memory".into(),
    })
    .unwrap();
    assert_eq!(ph.sisters_connected, 1);
    ph.expand(PerceptionExpansion::DeviceConnected {
        device_class: "mobile".into(),
    })
    .unwrap();
    assert_eq!(ph.devices_seen, 1);
}
