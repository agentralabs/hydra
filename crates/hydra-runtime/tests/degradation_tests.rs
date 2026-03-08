use std::time::Instant;

use hydra_runtime::degradation::{
    DegradationAction, DegradationLevel, DegradationManager, DegradationPolicy, PolicyConfig,
    ResourceMonitor, ResourceSnapshot,
};

fn snap(memory: f64, cpu: f64, disk: u64) -> ResourceSnapshot {
    ResourceSnapshot {
        memory_percent: memory,
        cpu_percent: cpu,
        disk_available_mb: disk,
        taken_at: Some(Instant::now()),
    }
}

// ═══════════════════════════════════════════════════════════
// DEGRADATION LEVELS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_degradation_levels() {
    // Verify all 4 levels exist and are ordered
    assert!(DegradationLevel::Normal < DegradationLevel::Reduced);
    assert!(DegradationLevel::Reduced < DegradationLevel::Minimal);
    assert!(DegradationLevel::Minimal < DegradationLevel::Emergency);

    // Display
    assert_eq!(DegradationLevel::Normal.to_string(), "normal");
    assert_eq!(DegradationLevel::Emergency.to_string(), "emergency");

    // Step up/down
    assert_eq!(
        DegradationLevel::Normal.step_up(),
        DegradationLevel::Reduced
    );
    assert_eq!(
        DegradationLevel::Emergency.step_down(),
        DegradationLevel::Minimal
    );
}

// ═══════════════════════════════════════════════════════════
// THRESHOLD TRIGGERS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_memory_threshold_triggers() {
    let policy = DegradationPolicy::with_defaults();

    // Normal
    assert_eq!(
        policy.raw_evaluate(&snap(50.0, 20.0, 5000)),
        DegradationLevel::Normal
    );

    // Memory > 70% → Reduced
    assert_eq!(
        policy.raw_evaluate(&snap(75.0, 20.0, 5000)),
        DegradationLevel::Reduced
    );

    // Memory > 85% → Minimal
    assert_eq!(
        policy.raw_evaluate(&snap(88.0, 20.0, 5000)),
        DegradationLevel::Minimal
    );

    // Memory > 95% → Emergency
    assert_eq!(
        policy.raw_evaluate(&snap(97.0, 20.0, 5000)),
        DegradationLevel::Emergency
    );
}

#[test]
fn test_cpu_threshold_triggers() {
    let policy = DegradationPolicy::with_defaults();

    // CPU > 90% → Reduced
    assert_eq!(
        policy.raw_evaluate(&snap(30.0, 92.0, 5000)),
        DegradationLevel::Reduced
    );

    // CPU high but memory higher → takes worst
    assert_eq!(
        policy.raw_evaluate(&snap(88.0, 95.0, 5000)),
        DegradationLevel::Minimal
    );
}

#[test]
fn test_disk_threshold_triggers() {
    let policy = DegradationPolicy::with_defaults();

    // Disk < 500MB → Minimal
    assert_eq!(
        policy.raw_evaluate(&snap(30.0, 20.0, 300)),
        DegradationLevel::Minimal
    );

    // Disk OK
    assert_eq!(
        policy.raw_evaluate(&snap(30.0, 20.0, 1000)),
        DegradationLevel::Normal
    );
}

// ═══════════════════════════════════════════════════════════
// HYSTERESIS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_hysteresis_prevents_flapping() {
    let config = PolicyConfig {
        hysteresis_secs: 30,
        recovery_secs: 30,
        ..PolicyConfig::default()
    };
    let policy = DegradationPolicy::new(config);

    // First evaluation: threshold exceeded but hysteresis not met
    let result = policy.evaluate(&snap(75.0, 20.0, 5000), DegradationLevel::Normal);
    assert_eq!(
        result,
        DegradationLevel::Normal,
        "Should NOT degrade immediately"
    );

    // Second evaluation: still within hysteresis window
    let result = policy.evaluate(&snap(75.0, 20.0, 5000), DegradationLevel::Normal);
    assert_eq!(
        result,
        DegradationLevel::Normal,
        "Should NOT degrade before hysteresis expires"
    );

    // Simulate hysteresis with zero-second config
    let fast_config = PolicyConfig {
        hysteresis_secs: 0,
        recovery_secs: 0,
        ..PolicyConfig::default()
    };
    let fast_policy = DegradationPolicy::new(fast_config);

    // With 0s hysteresis, should degrade immediately on second check
    fast_policy.evaluate(&snap(75.0, 20.0, 5000), DegradationLevel::Normal);
    let result = fast_policy.evaluate(&snap(75.0, 20.0, 5000), DegradationLevel::Normal);
    assert_eq!(
        result,
        DegradationLevel::Reduced,
        "Should degrade with 0s hysteresis"
    );
}

// ═══════════════════════════════════════════════════════════
// ACTIONS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_actions_on_level_change() {
    let mgr = DegradationManager::with_defaults();

    // Normal → Reduced
    let actions = mgr.force_level(DegradationLevel::Reduced);
    assert!(actions.contains(&DegradationAction::DisableShadowSim));
    assert!(actions.contains(&DegradationAction::ReduceCache { percent: 50 }));
    assert_eq!(mgr.level(), DegradationLevel::Reduced);

    // Reduced → Minimal
    let actions = mgr.force_level(DegradationLevel::Minimal);
    assert!(actions.contains(&DegradationAction::UnloadLocalModels));
    assert!(actions.contains(&DegradationAction::UnloadSisters));
    assert!(actions.contains(&DegradationAction::HaikuOnly));
    assert_eq!(mgr.level(), DegradationLevel::Minimal);
}

// ═══════════════════════════════════════════════════════════
// RECOVERY
// ═══════════════════════════════════════════════════════════

#[test]
fn test_recovery_to_normal() {
    let mgr = DegradationManager::with_defaults();

    // Degrade to Emergency
    mgr.force_level(DegradationLevel::Emergency);
    assert!(mgr.runs_paused());
    assert_eq!(mgr.level(), DegradationLevel::Emergency);

    // Recover to Normal
    mgr.clear_override();
    let actions = mgr.force_level(DegradationLevel::Normal);
    assert!(!mgr.runs_paused());
    assert!(actions.contains(&DegradationAction::ResumeRuns));
    assert!(actions.contains(&DegradationAction::EnableShadowSim));
    assert!(actions.contains(&DegradationAction::ResumeNormal));
    assert_eq!(mgr.level(), DegradationLevel::Normal);
}

// ═══════════════════════════════════════════════════════════
// EMERGENCY
// ═══════════════════════════════════════════════════════════

#[test]
fn test_emergency_pauses_runs() {
    let mgr = DegradationManager::with_defaults();
    assert!(!mgr.runs_paused());

    let actions = mgr.force_level(DegradationLevel::Emergency);
    assert!(mgr.runs_paused(), "Emergency should pause new runs");
    assert!(actions.contains(&DegradationAction::PauseNewRuns));
    assert!(actions.contains(&DegradationAction::AggressiveGc));
}

// ═══════════════════════════════════════════════════════════
// SSE EVENT DATA
// ═══════════════════════════════════════════════════════════

#[test]
fn test_degradation_event_data() {
    let mgr = DegradationManager::with_defaults();
    mgr.force_level(DegradationLevel::Reduced);

    let history = mgr.history();
    assert_eq!(history.len(), 1);

    let transition = &history[0];
    assert_eq!(transition.from, DegradationLevel::Normal);
    assert_eq!(transition.to, DegradationLevel::Reduced);
    assert!(!transition.reason.is_empty());
    assert!(!transition.actions_taken.is_empty());
    assert!(!transition.timestamp.is_empty());

    // Serializable for SSE
    let json = serde_json::to_value(transition).unwrap();
    assert_eq!(json["from"], "normal");
    assert_eq!(json["to"], "reduced");
    assert!(json["actions_taken"].is_array());
}

// ═══════════════════════════════════════════════════════════
// MANUAL OVERRIDE
// ═══════════════════════════════════════════════════════════

#[test]
fn test_manual_override() {
    let mgr = DegradationManager::with_defaults();

    // Force to Minimal manually
    mgr.force_level(DegradationLevel::Minimal);
    assert_eq!(mgr.level(), DegradationLevel::Minimal);

    // Even with normal resources, override persists
    assert_eq!(mgr.policy().get_override(), Some(DegradationLevel::Minimal));

    // Clear override
    mgr.clear_override();
    assert_eq!(mgr.policy().get_override(), None);
}

// ═══════════════════════════════════════════════════════════
// RESOURCE MONITOR
// ═══════════════════════════════════════════════════════════

#[test]
fn test_resource_monitor() {
    let monitor = ResourceMonitor::new();
    let snap = monitor.snapshot();
    // Should return valid ranges
    assert!(snap.memory_percent >= 0.0 && snap.memory_percent <= 100.0);
    assert!(snap.cpu_percent >= 0.0);
    assert!(snap.disk_available_mb > 0);

    // last() should match
    let last = monitor.last();
    assert_eq!(last.memory_percent, snap.memory_percent);
}
