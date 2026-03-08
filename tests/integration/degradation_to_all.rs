//! Category 2: Integration — degradation affects all systems.

use hydra_runtime::degradation::*;

#[test]
fn test_degradation_affects_scheduling() {
    use hydra_runtime::daemon::tasks::TaskId;

    // At Normal level, all tasks should run
    for task in TaskId::all() {
        assert!(task.allowed_at(manager::DegradationLevel::Normal));
    }

    // At Emergency, only critical tasks should run
    let emergency_tasks: Vec<_> = TaskId::all().iter()
        .filter(|t| t.allowed_at(manager::DegradationLevel::Emergency))
        .collect();
    assert!(emergency_tasks.len() < TaskId::all().len());
}

#[test]
fn test_degradation_level_transitions() {
    let actions = actions::actions_for_transition(
        manager::DegradationLevel::Normal,
        manager::DegradationLevel::Reduced,
    );
    assert!(!actions.is_empty());
    // Should include DisableShadowSim at Reduced
    assert!(actions.iter().any(|a| matches!(a, actions::DegradationAction::DisableShadowSim)));
}

#[test]
fn test_degradation_recovery_actions() {
    let actions = actions::actions_for_transition(
        manager::DegradationLevel::Reduced,
        manager::DegradationLevel::Normal,
    );
    assert!(!actions.is_empty());
    assert!(actions.iter().any(|a| matches!(a, actions::DegradationAction::ResumeNormal | actions::DegradationAction::EnableShadowSim)));
}

#[test]
fn test_degradation_emergency_pauses_runs() {
    let mgr = manager::DegradationManager::with_defaults();
    mgr.force_level(manager::DegradationLevel::Emergency);
    assert_eq!(mgr.level(), manager::DegradationLevel::Emergency);
    assert!(mgr.runs_paused());
}

#[test]
fn test_degradation_normal_no_pause() {
    let mgr = manager::DegradationManager::with_defaults();
    assert_eq!(mgr.level(), manager::DegradationLevel::Normal);
    assert!(!mgr.runs_paused());
}

#[test]
fn test_degradation_policy_hysteresis() {
    let policy = policy::DegradationPolicy::with_defaults();
    let snap = monitor::ResourceSnapshot {
        memory_percent: 75.0, // above reduced threshold
        cpu_percent: 50.0,
        disk_available_mb: 10000,
        taken_at: None,
    };
    let level = policy.raw_evaluate(&snap);
    assert!(level >= manager::DegradationLevel::Reduced);
}
