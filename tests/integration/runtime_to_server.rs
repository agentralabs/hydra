//! Category 2: Integration — runtime ↔ server data flow.

use hydra_runtime::*;
use hydra_core::*;

#[test]
fn test_rpc_to_cognitive_loop_types() {
    // Verify JSON-RPC request can carry cognitive loop parameters
    let req = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: serde_json::json!(1),
        method: "hydra.run".into(),
        params: serde_json::json!({
            "intent": "explain quantum computing",
            "context": {"project": "physics-sim"},
            "options": {"stream": true}
        }),
    };
    assert!(req.is_valid());
    assert_eq!(req.method, "hydra.run");
    assert!(req.params.get("intent").is_some());
}

#[test]
fn test_sse_events_all_types_schema() {
    // Verify all SSE event types produce valid JSON
    let events = vec![
        SseEvent::new(SseEventType::RunStarted, serde_json::json!({
            "run_id": "r1", "intent": "test"
        })),
        SseEvent::new(SseEventType::StepStarted, serde_json::json!({
            "run_id": "r1", "step": 1, "phase": "perceive"
        })),
        SseEvent::new(SseEventType::StepCompleted, serde_json::json!({
            "run_id": "r1", "step": 1, "phase": "perceive", "tokens": 150
        })),
        SseEvent::new(SseEventType::ApprovalRequired, serde_json::json!({
            "run_id": "r1", "action": "delete files", "risk": 0.8
        })),
        SseEvent::new(SseEventType::RunCompleted, serde_json::json!({
            "run_id": "r1", "response": "done"
        })),
        SseEvent::new(SseEventType::RunError, serde_json::json!({
            "run_id": "r1", "error": "something failed"
        })),
    ];
    for event in &events {
        let sse_str = event.to_sse_string();
        // Extract data field and verify it's valid JSON
        for line in sse_str.lines() {
            if line.starts_with("data:") {
                let data = line.trim_start_matches("data:").trim();
                let _: serde_json::Value = serde_json::from_str(data).expect("invalid JSON in SSE data");
            }
        }
    }
}

#[test]
fn test_kill_switch_propagation() {
    let ks = KillSwitch::new();
    let mut rx = ks.subscribe();

    ks.instant_halt("emergency");
    assert!(ks.is_active());

    let signal = rx.try_recv().unwrap();
    assert_eq!(signal, KillSignal::InstantHalt);
}

#[test]
fn test_approval_flow_complete() {
    let mgr = ApprovalManager::with_default_timeout();
    let (req, _rx) = mgr.request_approval("run1", "delete all files", None, 0.9, "high risk");
    assert!(mgr.is_pending(&req.id));

    // Submit approval
    mgr.submit_decision(&req.id, ApprovalDecision::Approved).unwrap();
    assert!(!mgr.is_pending(&req.id));
    assert_eq!(mgr.get_status(&req.id), Some(ApprovalStatus::Approved));
}

#[test]
fn test_approval_deny_flow() {
    let mgr = ApprovalManager::with_default_timeout();
    let (req, _rx) = mgr.request_approval("run1", "rm -rf /", None, 0.95, "critical");

    mgr.submit_decision(&req.id, ApprovalDecision::Denied { reason: "too dangerous".into() }).unwrap();
    assert_eq!(mgr.get_status(&req.id), Some(ApprovalStatus::Denied));
}
