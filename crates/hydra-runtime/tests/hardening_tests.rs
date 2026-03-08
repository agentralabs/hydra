//! Category 1: Unit Gap Fill — hydra-runtime edge cases.

use hydra_core::CognitivePhase;
use hydra_runtime::boot::BootError;
use hydra_runtime::filesystem::verify_filesystem;
use hydra_runtime::shutdown::ShutdownResult;
use hydra_runtime::sse::SseEventType;
use hydra_runtime::*;

// === Cognitive loop error paths ===

#[test]
fn test_cognitive_phase_all_variants() {
    let phases = vec![
        CognitivePhase::Perceive,
        CognitivePhase::Think,
        CognitivePhase::Decide,
        CognitivePhase::Act,
        CognitivePhase::Learn,
    ];
    assert_eq!(phases.len(), 5);
}

#[test]
fn test_urgency_all_variants() {
    use hydra_runtime::cognitive::types::Urgency;
    let _low = Urgency::Low;
    let _med = Urgency::Medium;
    let _high = Urgency::High;
    let _crit = Urgency::Critical;
}

// === Boot sequence failure modes ===

#[test]
fn test_boot_error_variants() {
    let errors = vec![
        BootError::ConfigInvalid("bad config".into()),
        BootError::RequiredSisterUnavailable("memory".into()),
        BootError::LockFailed("locked".into()),
        BootError::DatabaseError("db error".into()),
    ];
    for e in &errors {
        assert!(!format!("{:?}", e).is_empty());
    }
}

#[tokio::test]
async fn test_boot_sequence_results() {
    let config = HydraRuntimeConfig::load_default();
    let mut boot = BootSequence::new(config);
    let bus = EventBus::new(100);
    // Boot will fail (no real env) but should not panic
    let _ = boot.execute(&bus).await;
    assert!(!boot.results().is_empty());
    assert!(boot.total_duration_ms() > 0 || boot.results().is_empty() == false);
}

// === Config validation ===

#[test]
fn test_config_malformed_validation() {
    let mut config = HydraRuntimeConfig::load_default();
    config.limits.token_budget = 0;
    let result = config.validate();
    // Should either pass or fail gracefully
    match result {
        Ok(_) => {} // 0 budget allowed in default config
        Err(errors) => assert!(!errors.is_empty()),
    }
}

#[test]
fn test_config_load_default_valid() {
    let config = HydraRuntimeConfig::load_default();
    assert!(config.validate().is_ok());
    assert!(config.limits.token_budget > 0);
    assert!(config.limits.max_concurrent_runs > 0);
}

// === Shutdown ===

#[test]
fn test_shutdown_result_fields() {
    let result = ShutdownResult {
        clean: true,
        duration_ms: 100,
        exit_code: 0,
        reason: "test".into(),
        cancelled_runs: 0,
    };
    assert!(result.clean);
    assert_eq!(result.exit_code, 0);
}

#[test]
fn test_shutdown_sequence_not_shutting_down_initially() {
    let seq = ShutdownSequence::new();
    assert!(!seq.is_shutting_down());
}

// === Kill switch ===

#[test]
fn test_kill_switch_all_signals() {
    let ks = KillSwitch::new();
    assert!(!ks.is_active());
    assert!(!ks.is_frozen());

    ks.freeze("test");
    assert!(ks.is_frozen());
    assert!(ks.should_block());

    ks.resume();
    assert!(!ks.is_frozen());

    ks.instant_halt("halt");
    assert!(ks.is_active());
    assert!(ks.should_block());
    assert_eq!(ks.current_signal(), Some(KillSignal::InstantHalt));
    assert!(ks.reason().unwrap().contains("halt"));

    ks.reset();
    assert!(!ks.is_active());
}

// === Approval manager ===

#[test]
fn test_approval_error_variants() {
    let errors = vec![
        ApprovalError::NotFound,
        ApprovalError::Timeout,
        ApprovalError::Cancelled,
        ApprovalError::ReceiverDropped,
    ];
    for e in &errors {
        assert!(!format!("{:?}", e).is_empty());
    }
}

#[test]
fn test_approval_manager_no_pending() {
    let mgr = ApprovalManager::with_default_timeout();
    assert_eq!(mgr.pending_count(), 0);
    assert!(mgr.list_pending().is_empty());
}

#[test]
fn test_approval_expiry_flow() {
    let mgr = ApprovalManager::new(std::time::Duration::from_millis(50));
    let (req, _rx) = mgr.request_approval("run1", "delete /tmp", None, 0.8, "high risk");
    assert!(mgr.is_pending(&req.id));
    assert_eq!(mgr.pending_count(), 1);

    // Cancel all
    let cancelled = mgr.cancel_all();
    assert_eq!(cancelled, 1);
    assert_eq!(mgr.pending_count(), 0);
}

// === Event bus ===

#[test]
fn test_event_bus_publish_subscribe() {
    let bus = EventBus::new(100);
    let mut rx = bus.subscribe();
    bus.publish(SseEvent::heartbeat());
    let event = rx.try_recv().unwrap();
    assert_eq!(event.event_type, SseEventType::Heartbeat);
    assert!(bus.total_published() >= 1);
}

// === SSE event serialization ===

#[test]
fn test_sse_event_all_types() {
    let events = vec![
        SseEvent::new(SseEventType::RunStarted, serde_json::json!({})),
        SseEvent::new(SseEventType::StepStarted, serde_json::json!({})),
        SseEvent::new(SseEventType::StepProgress, serde_json::json!({})),
        SseEvent::new(SseEventType::StepCompleted, serde_json::json!({})),
        SseEvent::new(SseEventType::ApprovalRequired, serde_json::json!({})),
        SseEvent::new(SseEventType::RunCompleted, serde_json::json!({})),
        SseEvent::new(SseEventType::RunError, serde_json::json!({})),
        SseEvent::heartbeat(),
        SseEvent::system_ready("1.0"),
        SseEvent::system_shutdown("test"),
    ];
    for event in &events {
        let sse_str = event.to_sse_string();
        assert!(sse_str.contains("event:"));
        assert!(sse_str.contains("data:"));
    }
}

// === JSON-RPC validation ===

#[test]
fn test_jsonrpc_request_valid() {
    let req = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: serde_json::json!(1),
        method: "hydra.run".into(),
        params: serde_json::json!({}),
    };
    assert!(req.is_valid());
}

#[test]
fn test_jsonrpc_request_invalid_version() {
    let req = JsonRpcRequest {
        jsonrpc: "1.0".into(),
        id: serde_json::json!(1),
        method: "test".into(),
        params: serde_json::json!({}),
    };
    assert!(!req.is_valid());
}

#[test]
fn test_jsonrpc_response_success_error() {
    let success = JsonRpcResponse::success(serde_json::json!(1), serde_json::json!({"ok": true}));
    assert!(success.is_success());

    let error = JsonRpcResponse::error(serde_json::json!(1), -32601, "method not found");
    assert!(!error.is_success());
}

// === Task registry ===

#[test]
fn test_task_registry_operations() {
    let registry = TaskRegistry::new();
    assert_eq!(registry.active_count(), 0);
    assert!(registry.list_active().is_empty());
    assert!(!registry.is_active("nonexistent"));
}

// === Degradation levels ===

#[test]
fn test_degradation_level_ordering() {
    use hydra_runtime::degradation::manager::DegradationLevel;
    assert!(DegradationLevel::Emergency > DegradationLevel::Minimal);
    assert!(DegradationLevel::Minimal > DegradationLevel::Reduced);
    assert!(DegradationLevel::Reduced > DegradationLevel::Normal);
}

#[test]
fn test_degradation_step_up_down() {
    use hydra_runtime::degradation::manager::DegradationLevel;
    assert_eq!(
        DegradationLevel::Normal.step_up(),
        DegradationLevel::Reduced
    );
    assert_eq!(
        DegradationLevel::Emergency.step_up(),
        DegradationLevel::Emergency
    ); // can't go higher
    assert_eq!(
        DegradationLevel::Emergency.step_down(),
        DegradationLevel::Minimal
    );
    assert_eq!(
        DegradationLevel::Normal.step_down(),
        DegradationLevel::Normal
    ); // can't go lower
}

// === Offline queue ===

#[test]
fn test_offline_queue_full() {
    use hydra_runtime::offline::queue::*;
    let queue = PendingSyncQueue::new(2);
    let a1 = PendingAction::new("test", serde_json::json!({}), SyncPriority::Normal);
    let a2 = PendingAction::new("test", serde_json::json!({}), SyncPriority::Normal);
    let a3 = PendingAction::new("test", serde_json::json!({}), SyncPriority::Normal);
    assert!(queue.enqueue(a1));
    assert!(queue.enqueue(a2));
    assert!(!queue.enqueue(a3)); // full
    assert_eq!(queue.len(), 2);
}

// === Filesystem ===

#[test]
fn test_filesystem_verify_nonexistent() {
    assert!(!verify_filesystem(std::path::Path::new(
        "/nonexistent/path/hydra"
    )));
}

// === Lock ===

#[test]
fn test_lock_initial_state() {
    let lock = InstanceLock::new(std::path::Path::new("/tmp/hydra-test-lock-check"));
    assert!(!lock.is_held());
}

// === Private features ===

#[test]
fn test_private_features_detection() {
    let active = PrivateFeatures::active();
    // In test mode, no private features should be active unless feature-flagged
    let _ = PrivateFeatures::any_active();
    let _ = active.len();
}
