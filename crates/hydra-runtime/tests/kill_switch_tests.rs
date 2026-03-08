use std::time::Duration;

use hydra_runtime::{
    ApprovalDecision, ApprovalError, ApprovalManager, ApprovalStatus, KillSignal, KillSwitch,
    TaskRegistry,
};

// ═══════════════════════════════════════════════════════════
// KILL SWITCH TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_instant_halt_stops_immediately() {
    let ks = KillSwitch::new();
    assert!(!ks.is_active());
    assert!(!ks.should_block());

    ks.instant_halt("test emergency");

    assert!(ks.is_active());
    assert!(ks.should_block());
    assert_eq!(ks.current_signal(), Some(KillSignal::InstantHalt));
    assert_eq!(ks.reason(), Some("test emergency".into()));
    assert!(ks.activated_at().is_some());
}

#[test]
fn test_graceful_stop_completes_phase() {
    let ks = KillSwitch::new();
    let mut rx = ks.subscribe();

    ks.graceful_stop("user requested");

    assert!(ks.is_active());
    assert!(ks.should_block());
    assert_eq!(ks.current_signal(), Some(KillSignal::GracefulStop));

    // Signal was broadcast
    let signal = rx.try_recv().unwrap();
    assert_eq!(signal, KillSignal::GracefulStop);
}

#[test]
fn test_freeze_and_resume() {
    let ks = KillSwitch::new();

    // Freeze
    ks.freeze("pausing for review");
    assert!(ks.is_frozen());
    assert!(ks.should_block());
    assert!(!ks.is_active()); // Freeze doesn't set active

    // Resume
    ks.resume();
    assert!(!ks.is_frozen());
    assert!(!ks.should_block());
}

#[test]
fn test_kill_switch_reset() {
    let ks = KillSwitch::new();
    ks.instant_halt("test");
    assert!(ks.is_active());

    ks.reset();
    assert!(!ks.is_active());
    assert!(!ks.is_frozen());
    assert!(ks.reason().is_none());
    assert!(ks.activated_at().is_none());
}

#[test]
fn test_concurrent_kills() {
    let ks = KillSwitch::new();
    let ks2 = ks.clone();

    // Both can activate
    ks.graceful_stop("first");
    ks2.instant_halt("second overrides");

    // InstantHalt overrides graceful
    assert_eq!(ks.current_signal(), Some(KillSignal::InstantHalt));
    assert!(ks.is_active());
}

#[test]
fn test_kill_signal_serialization() {
    let signal = KillSignal::InstantHalt;
    let json = serde_json::to_string(&signal).unwrap();
    assert_eq!(json, "\"instant_halt\"");

    let parsed: KillSignal = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, KillSignal::InstantHalt);
}

// ═══════════════════════════════════════════════════════════
// TASK REGISTRY TESTS
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_task_registry_tracking() {
    let registry = TaskRegistry::new();

    let token = registry.create_token("run-1");
    let handle = tokio::spawn(async move {
        token.cancelled().await;
    });
    registry.register("run-1", handle, registry.create_token("run-1-dup"));

    assert!(registry.is_active("run-1"));
    assert_eq!(registry.active_count(), 2); // run-1 + run-1-dup token

    registry.cancel("run-1");
    assert!(!registry.is_active("run-1"));
}

#[tokio::test]
async fn test_task_registry_cancel_all() {
    let registry = TaskRegistry::new();

    for i in 0..5 {
        let token = registry.create_token(&format!("run-{}", i));
        let handle = tokio::spawn(async move {
            token.cancelled().await;
        });
        registry.register(&format!("run-{}", i), handle, CancellationToken::new());
    }

    let cancelled = registry.cancel_all();
    assert_eq!(cancelled, 5);
    assert_eq!(registry.active_count(), 0);
}

// Need CancellationToken for the above test
use tokio_util::sync::CancellationToken;

// ═══════════════════════════════════════════════════════════
// APPROVAL TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_approval_request_emit() {
    let mgr = ApprovalManager::with_default_timeout();

    let (req, _rx) = mgr.request_approval(
        "run-1",
        "delete file",
        Some("/tmp/foo"),
        0.75,
        "High risk operation",
    );

    assert_eq!(req.run_id, "run-1");
    assert_eq!(req.action, "delete file");
    assert_eq!(req.risk_score, 0.75);
    assert!(mgr.is_pending(&req.id));
    assert_eq!(mgr.pending_count(), 1);
}

#[tokio::test]
async fn test_approval_approve() {
    let mgr = ApprovalManager::with_default_timeout();

    let (req, rx) = mgr.request_approval("run-1", "send email", None, 0.6, "Requires confirmation");
    let approval_id = req.id.clone();

    // Submit approval in background
    mgr.submit_decision(&approval_id, ApprovalDecision::Approved)
        .unwrap();

    let decision = rx.await.unwrap();
    assert_eq!(decision, ApprovalDecision::Approved);
    assert_eq!(mgr.get_status(&approval_id), Some(ApprovalStatus::Approved));
}

#[tokio::test]
async fn test_approval_deny() {
    let mgr = ApprovalManager::with_default_timeout();

    let (req, rx) = mgr.request_approval(
        "run-1",
        "drop table",
        Some("users"),
        0.95,
        "Destructive operation",
    );

    mgr.submit_decision(
        &req.id,
        ApprovalDecision::Denied {
            reason: "Too risky".into(),
        },
    )
    .unwrap();

    let decision = rx.await.unwrap();
    assert_eq!(
        decision,
        ApprovalDecision::Denied {
            reason: "Too risky".into()
        }
    );
    assert_eq!(mgr.get_status(&req.id), Some(ApprovalStatus::Denied));
}

#[tokio::test]
async fn test_approval_timeout() {
    let mgr = ApprovalManager::new(Duration::from_millis(50)); // Very short timeout

    let (req, rx) = mgr.request_approval("run-1", "test action", None, 0.5, "test");

    let result = mgr.wait_for_approval(&req.id, rx).await;
    assert_eq!(result, Err(ApprovalError::Timeout));
    assert_eq!(mgr.get_status(&req.id), Some(ApprovalStatus::Expired));
}

#[tokio::test]
async fn test_approval_with_modification() {
    let mgr = ApprovalManager::with_default_timeout();

    let (req, rx) = mgr.request_approval("run-1", "delete /important", None, 0.8, "High risk");

    mgr.submit_decision(
        &req.id,
        ApprovalDecision::Modified {
            new_action: "move to trash".into(),
        },
    )
    .unwrap();

    let decision = rx.await.unwrap();
    assert_eq!(
        decision,
        ApprovalDecision::Modified {
            new_action: "move to trash".into()
        }
    );
    assert_eq!(mgr.get_status(&req.id), Some(ApprovalStatus::Modified));
}

#[tokio::test]
async fn test_kill_switch_cancels_pending_approval() {
    let mgr = ApprovalManager::with_default_timeout();

    // Create 3 pending approvals
    let (_req1, rx1) = mgr.request_approval("run-1", "action1", None, 0.5, "test");
    let (_req2, rx2) = mgr.request_approval("run-2", "action2", None, 0.6, "test");
    let (_req3, rx3) = mgr.request_approval("run-3", "action3", None, 0.7, "test");

    assert_eq!(mgr.pending_count(), 3);

    // Kill switch cancels all
    let cancelled = mgr.cancel_all();
    assert_eq!(cancelled, 3);
    assert_eq!(mgr.pending_count(), 0);

    // All receivers get Denied
    let d1 = rx1.await.unwrap();
    let d2 = rx2.await.unwrap();
    let d3 = rx3.await.unwrap();
    assert_eq!(
        d1,
        ApprovalDecision::Denied {
            reason: "Kill switch activated".into()
        }
    );
    assert_eq!(
        d2,
        ApprovalDecision::Denied {
            reason: "Kill switch activated".into()
        }
    );
    assert_eq!(
        d3,
        ApprovalDecision::Denied {
            reason: "Kill switch activated".into()
        }
    );
}

#[test]
fn test_approval_not_found() {
    let mgr = ApprovalManager::with_default_timeout();
    let result = mgr.submit_decision("nonexistent", ApprovalDecision::Approved);
    assert_eq!(result, Err(ApprovalError::NotFound));
}

#[test]
fn test_sse_events_emitted() {
    // Kill switch signals are serializable for SSE
    let signals = vec![
        KillSignal::InstantHalt,
        KillSignal::GracefulStop,
        KillSignal::Freeze,
        KillSignal::Resume,
    ];

    for signal in signals {
        let json = serde_json::to_value(&signal).unwrap();
        assert!(json.is_string());
    }

    // Approval requests are serializable for SSE
    let mgr = ApprovalManager::with_default_timeout();
    let (req, _rx) = mgr.request_approval("run-1", "test", None, 0.5, "test");
    let json = serde_json::to_value(&req).unwrap();
    assert!(json.get("id").is_some());
    assert!(json.get("expires_at").is_some());
}
