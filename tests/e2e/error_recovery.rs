//! Category 3: E2E — error recovery flows.

use hydra_runtime::*;
use hydra_core::*;

#[test]
fn test_crash_resume_from_checkpoint() {
    use hydra_inventions::resurrection;

    let store = resurrection::store::CheckpointStore::new();
    // Simulate saving state before crash
    let cp = resurrection::checkpoint::Checkpoint::create(
        "pre_crash",
        serde_json::json!({
            "run_id": "run-1",
            "phase": "think",
            "completed_phases": ["perceive"],
            "tokens_used": 200,
        }),
    );
    store.save(cp.clone());

    // Simulate crash + restart
    let restored = store.restore(&cp.id).unwrap();
    assert_eq!(restored.label, "pre_crash");
    let state = restored.state;
    assert_eq!(state["phase"], "think");
}

#[test]
fn test_sister_failure_graceful() {
    // Verify error types for sister failures are retryable
    let err = HydraError::SisterUnreachable("memory".into());
    assert!(err.is_retryable());

    let err = HydraError::SisterNotFound("nonexistent".into());
    assert!(!err.is_retryable());

    // Suggested action should be helpful
    assert!(err.suggested_action().is_some());
}

#[test]
fn test_kill_switch_recovery() {
    let ks = KillSwitch::new();

    // Activate kill switch
    ks.instant_halt("critical error");
    assert!(ks.is_active());
    assert!(ks.should_block());

    // Recovery
    ks.reset();
    assert!(!ks.is_active());
    assert!(!ks.should_block());
}

#[test]
fn test_degradation_recovery_flow() {
    use hydra_runtime::degradation::manager::*;

    let mgr = DegradationManager::with_defaults();

    // Simulate degradation
    mgr.force_level(DegradationLevel::Minimal);
    assert_eq!(mgr.level(), DegradationLevel::Minimal);

    // Recovery
    mgr.force_level(DegradationLevel::Normal);
    assert_eq!(mgr.level(), DegradationLevel::Normal);
    assert!(!mgr.runs_paused());
}

#[test]
fn test_offline_queue_recovery() {
    use hydra_runtime::offline::queue::*;

    let queue = PendingSyncQueue::with_defaults();
    // Simulate offline queueing
    queue.enqueue(PendingAction::new("memory_add", serde_json::json!({"content": "test"}), SyncPriority::High));
    queue.enqueue(PendingAction::new("log_action", serde_json::json!({"action": "write"}), SyncPriority::Normal));
    assert_eq!(queue.length(), 2);

    // Simulate sync
    let action = queue.dequeue().unwrap();
    queue.mark_synced();
    assert_eq!(queue.length(), 1);
    assert_eq!(action.action_type, "memory_add"); // high priority first
}
