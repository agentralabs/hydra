//! Phase 24D: Cross-crate integration tests for Hydra.
//!
//! These tests verify the wiring between components without requiring
//! real servers, network access, or voice systems. All I/O uses temp dirs.

use std::sync::Arc;

// ══════════════════════════════════════════════════════════════════════
// Test 1: Full run flow — create task, advance through cognitive phases
// ══════════════════════════════════════════════════════════════════════

#[test]
fn test_full_run_flow() {
    use hydra_core::types::{CognitivePhase, TokenBudget};
    use hydra_kernel::state::CognitiveState;
    use hydra_runtime::tasks::TaskManager;
    use hydra_runtime::HydraTaskStatus;

    // 1. Create a task
    let mut tm = TaskManager::new();
    let task = tm.create_task("Implement REST API");
    assert_eq!(task.status, HydraTaskStatus::Pending);

    // 2. Activate the task
    tm.update_status(&task.id, HydraTaskStatus::Active);
    let active_task = tm.get_by_id(&task.id).unwrap();
    assert_eq!(active_task.status, HydraTaskStatus::Active);

    // 3. Advance through cognitive phases
    let budget = TokenBudget::new(10000);
    let mut state = CognitiveState::new(budget);
    assert_eq!(state.phase, CognitivePhase::Perceive);

    state.transition_to(CognitivePhase::Think).unwrap();
    assert_eq!(state.phase, CognitivePhase::Think);

    state.transition_to(CognitivePhase::Decide).unwrap();
    assert_eq!(state.phase, CognitivePhase::Decide);

    state.transition_to(CognitivePhase::Act).unwrap();
    assert_eq!(state.phase, CognitivePhase::Act);

    state.transition_to(CognitivePhase::Learn).unwrap();
    assert_eq!(state.phase, CognitivePhase::Learn);

    // 4. Mark task completed
    tm.complete_task(&task.id);
    let done = tm.get_by_id(&task.id).unwrap();
    assert_eq!(done.status, HydraTaskStatus::Completed);
    assert!(done.completed_at.is_some());
}

// ══════════════════════════════════════════════════════════════════════
// Test 2: Approval flow — create request, validate challenge, approve
// ══════════════════════════════════════════════════════════════════════

#[test]
fn test_approval_flow() {
    use hydra_gate::ChallengeManager;
    use hydra_runtime::approval::{ApprovalDecision, ApprovalManager, ApprovalStatus};

    // 1. Create approval request
    let mgr = ApprovalManager::with_default_timeout();
    let (req, _rx) = mgr.request_approval(
        "run-42",
        "delete production database",
        Some("/var/db/prod.sqlite"),
        0.95,
        "Critical action: permanent data loss",
    );
    assert!(mgr.is_pending(&req.id));
    assert_eq!(mgr.pending_count(), 1);

    // 2. Generate challenge for high-risk action
    let mut challenge_mgr = ChallengeManager::default();
    let challenge = challenge_mgr.generate(&req.id);
    assert!(!challenge.phrase.is_empty());
    assert!(!challenge.is_expired());

    // 3. Validate challenge (case-insensitive)
    let valid = challenge_mgr.validate(&req.id, &challenge.phrase.to_lowercase());
    assert!(valid);

    // 4. Approve the request
    mgr.submit_decision(&req.id, ApprovalDecision::Approved)
        .unwrap();
    assert_eq!(mgr.get_status(&req.id), Some(ApprovalStatus::Approved));
    assert_eq!(mgr.pending_count(), 0);
}

// ══════════════════════════════════════════════════════════════════════
// Test 3: Profile persistence — create, save, reload, verify
// ══════════════════════════════════════════════════════════════════════

#[test]
fn test_profile_persistence() {
    use hydra_runtime::profile::{InterfaceMode, Theme, UserProfile};

    let dir = std::env::temp_dir().join(format!("hydra-test-profile-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("profile.json");

    // 1. Create and configure a profile
    let mut profile = UserProfile::default();
    profile.set_name("Ada Lovelace");
    profile.onboarding_complete = true;
    profile.preferences.theme = Theme::Dark;
    profile.preferences.default_mode = InterfaceMode::Workspace;
    profile.preferences.voice_enabled = true;
    profile.preferences.language = "es".into();

    // 2. Save
    profile.save_to(&path).unwrap();

    // 3. Reload
    let loaded = UserProfile::load_from(&path).unwrap();

    // 4. Verify fields match
    assert_eq!(loaded.name.as_deref(), Some("Ada Lovelace"));
    assert!(loaded.onboarding_complete);
    assert_eq!(loaded.preferences.theme, Theme::Dark);
    assert_eq!(loaded.preferences.default_mode, InterfaceMode::Workspace);
    assert!(loaded.preferences.voice_enabled);
    assert_eq!(loaded.preferences.language, "es");
    assert_eq!(loaded.get_greeting(), "Hi Ada Lovelace!");

    // Cleanup
    let _ = std::fs::remove_dir_all(&dir);
}

// ══════════════════════════════════════════════════════════════════════
// Test 4: Message persistence — store, query by conversation
// ══════════════════════════════════════════════════════════════════════

#[test]
fn test_message_persistence() {
    use hydra_db::{Conversation, Message, MessageRole, MessageStore};

    let store = MessageStore::in_memory().unwrap();

    // 1. Create a conversation
    let conv = Conversation {
        id: "conv-1".into(),
        title: Some("Architecture discussion".into()),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    store.create_conversation(&conv).unwrap();

    // 2. Add messages
    let msg1 = Message {
        id: "msg-1".into(),
        conversation_id: "conv-1".into(),
        role: MessageRole::User,
        content: "How should we structure the API?".into(),
        created_at: chrono::Utc::now().to_rfc3339(),
        run_id: None,
        metadata: None,
    };
    let msg2 = Message {
        id: "msg-2".into(),
        conversation_id: "conv-1".into(),
        role: MessageRole::Hydra,
        content: "I recommend a layered architecture with clear boundaries.".into(),
        created_at: chrono::Utc::now().to_rfc3339(),
        run_id: Some("run-1".into()),
        metadata: None,
    };
    store.add_message(&msg1).unwrap();
    store.add_message(&msg2).unwrap();

    // 3. Query by conversation
    let messages = store.get_conversation("conv-1", None).unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].role, MessageRole::User);
    assert_eq!(messages[1].role, MessageRole::Hydra);

    // 4. Search
    let results = store.search("layered").unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].content.contains("layered"));

    // 5. Verify conversation info round-trips
    let info = store.get_conversation_info("conv-1").unwrap();
    assert_eq!(info.title.as_deref(), Some("Architecture discussion"));
}

// ══════════════════════════════════════════════════════════════════════
// Test 5: Task auto-creation — message triggers task linkage
// ══════════════════════════════════════════════════════════════════════

#[test]
fn test_task_auto_creation() {
    use hydra_db::{Conversation, Message, MessageRole, MessageStore};
    use hydra_runtime::tasks::TaskManager;

    let store = MessageStore::in_memory().unwrap();
    let mut tm = TaskManager::new();

    // User sends a message
    let conv = Conversation {
        id: "conv-auto".into(),
        title: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    store.create_conversation(&conv).unwrap();

    let msg = Message {
        id: "msg-auto-1".into(),
        conversation_id: "conv-auto".into(),
        role: MessageRole::User,
        content: "Refactor the authentication module".into(),
        created_at: chrono::Utc::now().to_rfc3339(),
        run_id: Some("run-auto".into()),
        metadata: None,
    };
    store.add_message(&msg).unwrap();

    // System creates a task based on the message
    let task = tm.create_task(&msg.content);
    tm.link_to_run(&task.id, msg.run_id.as_deref().unwrap());

    // Verify linkage
    let linked = tm.get_by_id(&task.id).unwrap();
    assert_eq!(linked.run_id.as_deref(), Some("run-auto"));
    assert_eq!(linked.title, "Refactor the authentication module");
}

// ══════════════════════════════════════════════════════════════════════
// Test 6: Notification on events — event bus publishes, notification created
// ══════════════════════════════════════════════════════════════════════

#[test]
fn test_notification_on_events() {
    use hydra_runtime::notifications::{Notification, NotificationManager, NotificationUrgency};
    use hydra_runtime::sse::{SseEvent, SseEventType};
    use hydra_runtime::EventBus;

    // 1. Publish an event via the bus
    let bus = EventBus::new(16);
    let mut rx = bus.subscribe();
    bus.publish(SseEvent::new(
        SseEventType::ApprovalRequired,
        serde_json::json!({"run_id": "r1", "risk": 0.8}),
    ));
    assert_eq!(bus.total_published(), 1);

    // 2. Subscriber receives it
    let event = rx.try_recv().unwrap();
    assert_eq!(event.event_type, SseEventType::ApprovalRequired);

    // 3. Create a notification in response
    let mut nm = NotificationManager::new();
    let notif = Notification {
        id: "n1".into(),
        title: "Approval needed".into(),
        body: "Run r1 requires your approval (risk: 0.8)".into(),
        urgency: NotificationUrgency::High,
        action: None,
        created_at: chrono::Utc::now(),
        read: false,
    };
    nm.send(notif);

    assert_eq!(nm.get_pending_count(), 1);
    assert_eq!(nm.get_unread().len(), 1);

    // 4. Mark read
    nm.mark_read("n1");
    assert_eq!(nm.get_pending_count(), 0);
}

// ══════════════════════════════════════════════════════════════════════
// Test 7: Undo action — perform, undo, verify state reverted
// ══════════════════════════════════════════════════════════════════════

#[test]
fn test_undo_action() {
    use hydra_runtime::undo::{GenericAction, UndoStack};

    let mut stack = UndoStack::new(10);

    // 1. Push actions
    let action1 = GenericAction::new("a1", "Renamed variable foo to bar");
    let action2 = GenericAction::new("a2", "Added error handling");
    stack.push(Box::new(action1));
    stack.push(Box::new(action2));

    assert_eq!(stack.undo_count(), 2);
    assert_eq!(stack.redo_count(), 0);
    assert_eq!(
        stack.last_action_description(),
        Some("Added error handling")
    );

    // 2. Undo last action
    stack.undo().unwrap();
    assert_eq!(stack.undo_count(), 1);
    assert_eq!(stack.redo_count(), 1);
    assert_eq!(
        stack.last_action_description(),
        Some("Renamed variable foo to bar")
    );

    // 3. Redo
    stack.redo().unwrap();
    assert_eq!(stack.undo_count(), 2);
    assert_eq!(stack.redo_count(), 0);

    // 4. Undo both
    stack.undo().unwrap();
    stack.undo().unwrap();
    assert!(!stack.can_undo());
    assert!(stack.can_redo());
}

// ══════════════════════════════════════════════════════════════════════
// Test 8: Challenge for critical action — generate and validate phrase
// ══════════════════════════════════════════════════════════════════════

#[test]
fn test_challenge_critical() {
    use hydra_gate::ChallengeManager;

    let mut mgr = ChallengeManager::new(120);

    // Generate a challenge
    let challenge = mgr.generate("delete-prod-db");
    assert!(!challenge.phrase.is_empty());
    assert_eq!(challenge.action_id, "delete-prod-db");
    assert!(!challenge.is_expired());
    assert_eq!(mgr.active_count(), 1);

    // Wrong phrase should fail
    let invalid = mgr.validate("delete-prod-db", "WRONG PHRASE");
    assert!(!invalid);
    // Challenge should still be active (not consumed on failure)
    assert_eq!(mgr.active_count(), 1);

    // Correct phrase (case-insensitive) should succeed and consume
    let valid = mgr.validate("delete-prod-db", &challenge.phrase.to_lowercase());
    assert!(valid);
    assert_eq!(mgr.active_count(), 0);

    // Second validation with same phrase should fail (one-time use)
    let reused = mgr.validate("delete-prod-db", &challenge.phrase);
    assert!(!reused);
}

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
