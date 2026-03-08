//! Category 7: Contract Tests — SSE event schema.

use hydra_runtime::*;

#[test]
fn test_all_event_types_have_schema() {
    let events = vec![
        ("run_started", SseEvent::new(SseEventType::RunStarted, serde_json::json!({"run_id": "r1"}))),
        ("step_started", SseEvent::new(SseEventType::StepStarted, serde_json::json!({"step": 1}))),
        ("step_completed", SseEvent::new(SseEventType::StepCompleted, serde_json::json!({"step": 1}))),
        ("approval_required", SseEvent::new(SseEventType::ApprovalRequired, serde_json::json!({"action": "test"}))),
        ("run_completed", SseEvent::new(SseEventType::RunCompleted, serde_json::json!({"response": "done"}))),
        ("run_error", SseEvent::new(SseEventType::RunError, serde_json::json!({"error": "fail"}))),
    ];
    for (name, event) in &events {
        let sse_str = event.to_sse_string();
        assert!(sse_str.contains("event:"), "missing event field for {}", name);
        assert!(sse_str.contains("data:"), "missing data field for {}", name);
    }
}

#[test]
fn test_event_ordering() {
    // Events have timestamps and should be ordered
    let e1 = SseEvent::new(SseEventType::RunStarted, serde_json::json!({}));
    std::thread::sleep(std::time::Duration::from_millis(1));
    let e2 = SseEvent::new(SseEventType::StepStarted, serde_json::json!({}));
    assert!(e2.timestamp >= e1.timestamp);
}

#[test]
fn test_heartbeat_minimal() {
    let hb = SseEvent::heartbeat();
    let s = hb.to_sse_string();
    assert!(s.contains("heartbeat"));
}

#[test]
fn test_sse_data_is_valid_json() {
    let event = SseEvent::new(SseEventType::RunCompleted, serde_json::json!({
        "run_id": "r1",
        "response": "Here's the result",
        "tokens_used": 500,
    }));
    let sse_str = event.to_sse_string();
    for line in sse_str.lines() {
        if line.starts_with("data:") {
            let data = line.trim_start_matches("data:").trim();
            let parsed: Result<serde_json::Value, _> = serde_json::from_str(data);
            assert!(parsed.is_ok(), "invalid JSON in SSE data: {}", data);
        }
    }
}
