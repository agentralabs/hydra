//! Phase 24D (extra): Cross-crate integration tests for Hydra — tests 9-15.
//!
//! These tests verify the wiring between components without requiring
//! real servers, network access, or voice systems. All I/O uses temp dirs.

use std::sync::Arc;

// ══════════════════════════════════════════════════════════════════════
// Test 9: Input validation — gate evaluates risk correctly
// ══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_input_validation() {
    use hydra_core::types::{Action, ActionType};
    use hydra_gate::risk::ActionContext;
    use hydra_gate::{ExecutionGate, GateConfig};

    let gate = ExecutionGate::new(GateConfig::default());

    // Low-risk action: read a normal file
    let read_action = Action::new(ActionType::Read, "src/main.rs");
    let ctx = ActionContext {
        in_sandbox: true,
        ..Default::default()
    };
    let decision = gate.evaluate(&read_action, &ctx, None).await;
    assert!(
        decision.is_approved(),
        "Read action should be auto-approved, got: {:?}",
        decision
    );

    // High-risk action: delete system files
    let delete_action = Action::new(ActionType::FileDelete, "/etc/passwd");
    let ctx_unsafe = ActionContext {
        in_sandbox: false,
        ..Default::default()
    };
    let decision = gate.evaluate(&delete_action, &ctx_unsafe, None).await;
    assert!(
        decision.is_blocked(),
        "System file deletion should be blocked, got: {:?}",
        decision
    );

    // Self-modification: target hydra internals
    let self_mod = Action::new(ActionType::FileModify, ".hydra/config.toml");
    let ctx_internal = ActionContext {
        is_hydra_internal: true,
        in_sandbox: true,
        ..Default::default()
    };
    let decision = gate.evaluate(&self_mod, &ctx_internal, None).await;
    assert!(
        decision.is_blocked() || decision.needs_approval(),
        "Self-modification should be blocked or need approval, got: {:?}",
        decision
    );
}

// ══════════════════════════════════════════════════════════════════════
// Test 10: Graceful degradation — missing optional components
// ══════════════════════════════════════════════════════════════════════

#[test]
fn test_graceful_degradation() {
    use hydra_runtime::degradation::{DegradationLevel, DegradationManager};
    use hydra_runtime::notifications::NotificationManager;
    use hydra_runtime::tasks::TaskManager;

    // System starts with all managers available
    let degrade = DegradationManager::with_defaults();
    let mut nm = NotificationManager::new();
    let mut tm = TaskManager::new();

    assert_eq!(degrade.level(), DegradationLevel::Normal);

    // Force degradation to Minimal — should not panic
    degrade.force_level(DegradationLevel::Minimal);
    assert_eq!(degrade.level(), DegradationLevel::Minimal);

    // Other subsystems keep working independently
    let task = tm.create_task("Background task");
    assert_eq!(task.status, hydra_runtime::HydraTaskStatus::Pending);

    let notif = hydra_runtime::notifications::Notification {
        id: "degrade-n1".into(),
        title: "System degraded".into(),
        body: "Operating in minimal mode".into(),
        urgency: hydra_runtime::notifications::NotificationUrgency::High,
        action: None,
        created_at: chrono::Utc::now(),
        read: false,
    };
    nm.send(notif);
    assert_eq!(nm.get_pending_count(), 1);

    // Recovery back to Normal
    degrade.force_level(DegradationLevel::Normal);
    assert_eq!(degrade.level(), DegradationLevel::Normal);
    assert!(!degrade.runs_paused());
}

// ══════════════════════════════════════════════════════════════════════
// Test 11: Concurrent access — multiple operations don't conflict
// ══════════════════════════════════════════════════════════════════════

#[test]
fn test_concurrent_access() {
    use hydra_runtime::sse::{SseEvent, SseEventType};
    use hydra_runtime::EventBus;

    let bus = Arc::new(EventBus::new(256));
    let mut handles = vec![];

    // Spawn multiple publishers
    for i in 0..10 {
        let bus_clone = Arc::clone(&bus);
        handles.push(std::thread::spawn(move || {
            bus_clone.publish(SseEvent::new(
                SseEventType::StepProgress,
                serde_json::json!({"step": i}),
            ));
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    assert_eq!(bus.total_published(), 10);
}

// ══════════════════════════════════════════════════════════════════════
// Test 12: Error recovery — errors produce friendly messages
// ══════════════════════════════════════════════════════════════════════

#[test]
fn test_error_recovery() {
    use hydra_core::HydraError;

    let errors = vec![
        HydraError::CompilationError("parse failure".into()),
        HydraError::NoActionDetected,
        HydraError::NoProtocolsFound,
        HydraError::AllProtocolsFailed("tried 3".into()),
        HydraError::DeploymentFailed("step 2".into()),
        HydraError::ApprovalRequired,
        HydraError::Timeout,
        HydraError::SisterNotFound("memory".into()),
        HydraError::SisterUnreachable("codebase".into()),
        HydraError::PermissionDenied("no write".into()),
        HydraError::ConfigError("bad toml".into()),
        HydraError::IoError("file not found".into()),
        HydraError::ReceiptChainBroken(42),
        HydraError::TokenBudgetExceeded {
            needed: 1000,
            available: 100,
        },
        HydraError::SessionNotFound("s1".into()),
        HydraError::SerializationError("bad json".into()),
        HydraError::Internal("unexpected".into()),
    ];

    for err in &errors {
        // Every error must have a non-empty user message
        let msg = err.user_message();
        assert!(
            !msg.is_empty(),
            "Error {:?} produced empty user message",
            err
        );

        // Every error must have a suggested action
        let action = err.suggested_action();
        assert!(
            action.is_some(),
            "Error {:?} has no suggested action",
            err
        );

        // Every error must have a non-empty error code
        let code = err.error_code();
        assert!(
            code.starts_with('E'),
            "Error code should start with E, got: {}",
            code
        );
    }

    // Retryable errors should be correctly identified
    assert!(HydraError::Timeout.is_retryable());
    assert!(HydraError::SisterUnreachable("x".into()).is_retryable());
    assert!(HydraError::IoError("x".into()).is_retryable());
    assert!(!HydraError::NoActionDetected.is_retryable());
    assert!(!HydraError::PermissionDenied("x".into()).is_retryable());
}

// ══════════════════════════════════════════════════════════════════════
// Test 13: Offline mode — system works without network
// ══════════════════════════════════════════════════════════════════════

#[test]
fn test_offline_mode() {
    use hydra_runtime::offline::{
        ConnectivityMonitor, ConnectivityState, PendingSyncQueue, SyncPriority,
    };

    // 1. Start in unknown state
    let monitor = ConnectivityMonitor::with_defaults();
    assert_eq!(monitor.state(), ConnectivityState::Unknown);

    // 2. Force offline
    monitor.force_state(ConnectivityState::Offline);
    assert!(monitor.is_offline());

    // 3. Pending sync queue still works while offline
    let queue = PendingSyncQueue::new(100);
    queue.enqueue(hydra_runtime::offline::PendingAction::new(
        "save_memory",
        serde_json::json!({"key": "test"}),
        SyncPriority::Normal,
    ));
    assert_eq!(queue.len(), 1);

    // 4. Task manager works offline
    let mut tm = hydra_runtime::tasks::TaskManager::new();
    let task = tm.create_task("Offline work");
    tm.complete_task(&task.id);
    let done = tm.get_by_id(&task.id).unwrap();
    assert_eq!(done.status, hydra_runtime::HydraTaskStatus::Completed);

    // 5. Transition back to online
    monitor.force_state(ConnectivityState::Online);
    assert!(monitor.is_online());
}

// ══════════════════════════════════════════════════════════════════════
// Test 14: Version sync — all crate versions match workspace version
// ══════════════════════════════════════════════════════════════════════

#[test]
fn test_version_sync() {
    // Verify key crate versions match by checking that the workspace version
    // is used consistently. We do this by reading Cargo.toml files.
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();

    let workspace_toml = std::fs::read_to_string(workspace_root.join("Cargo.toml"))
        .expect("Cannot read workspace Cargo.toml");

    // Extract workspace version
    let version_line = workspace_toml
        .lines()
        .find(|l| l.starts_with("version") && l.contains('"'))
        .expect("No version in workspace Cargo.toml");
    let workspace_version = version_line
        .split('"')
        .nth(1)
        .expect("Cannot parse workspace version");

    // Check a selection of crate Cargo.toml files use workspace version
    let crate_names = [
        "hydra-core",
        "hydra-gate",
        "hydra-kernel",
        "hydra-runtime",
        "hydra-db",
        "hydra-cli",
    ];

    for name in &crate_names {
        let crate_toml_path = workspace_root.join("crates").join(name).join("Cargo.toml");
        let content = std::fs::read_to_string(&crate_toml_path)
            .unwrap_or_else(|_| panic!("Cannot read {}/Cargo.toml", name));

        // Crates should use `version.workspace = true` or explicit match
        let uses_workspace = content.contains("version.workspace = true");
        let has_explicit_match = content
            .lines()
            .any(|l| l.starts_with("version") && l.contains(workspace_version));

        assert!(
            uses_workspace || has_explicit_match,
            "Crate {} does not use workspace version (expected {})",
            name,
            workspace_version
        );
    }
}

// ══════════════════════════════════════════════════════════════════════
// Test 15: Smoke test — cognitive state, gate, and kernel interact
// ══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_smoke_test_cross_crate() {
    use hydra_core::types::{Action, ActionType, CognitivePhase, TokenBudget};
    use hydra_gate::risk::ActionContext;
    use hydra_gate::{ExecutionGate, GateConfig};
    use hydra_kernel::state::CognitiveState;
    use hydra_runtime::tasks::TaskManager;

    // 1. Create cognitive state
    let budget = TokenBudget::new(5000);
    let mut state = CognitiveState::new(budget);
    assert_eq!(state.run_state, hydra_kernel::KernelRunState::Idle);

    // 2. Begin perceiving (already in Perceive phase)
    assert_eq!(state.phase, CognitivePhase::Perceive);

    // 3. Create a task for the work
    let mut tm = TaskManager::new();
    let task = tm.create_task("Write integration tests");
    tm.update_status(&task.id, hydra_runtime::HydraTaskStatus::Active);

    // 4. Think about the approach
    state.transition_to(CognitivePhase::Think).unwrap();

    // 5. Decide and evaluate risk
    state.transition_to(CognitivePhase::Decide).unwrap();
    let action = Action::new(ActionType::FileCreate, "tests/new_test.rs");
    let gate = ExecutionGate::new(GateConfig::default());
    let ctx = ActionContext {
        in_sandbox: true,
        ..Default::default()
    };
    let decision = gate.evaluate(&action, &ctx, None).await;
    assert!(
        decision.is_approved(),
        "File creation in sandbox should be approved"
    );

    // 6. Act
    state.transition_to(CognitivePhase::Act).unwrap();

    // 7. Learn
    state.transition_to(CognitivePhase::Learn).unwrap();
    state.budget.record_usage(200);
    assert_eq!(state.budget.used(), 200);

    // 8. Complete the task
    tm.complete_task(&task.id);
    assert!(tm.get_by_id(&task.id).unwrap().status.is_terminal());

    // 9. Verify audit chain integrity
    assert!(gate.verify_audit_chain());
}
