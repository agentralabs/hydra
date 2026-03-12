//! Category 10: Server Tests — RPC methods, SSE, state.

use hydra_db::HydraDb;
use hydra_runtime::sse::SseEventType;
use hydra_runtime::*;
use hydra_server::*;

// === App state ===

#[test]
fn test_app_state_creation() {
    let db = HydraDb::in_memory().expect("in-memory db");
    let state = AppState::new(db, false, None);
    assert!(state.uptime().as_millis() >= 0);
}

// === RPC method validation ===

#[test]
fn test_rpc_error_codes_unique() {
    let codes = vec![
        RpcErrorCodes::PARSE_ERROR,
        RpcErrorCodes::INVALID_REQUEST,
        RpcErrorCodes::METHOD_NOT_FOUND,
        RpcErrorCodes::INVALID_PARAMS,
        RpcErrorCodes::INTERNAL_ERROR,
        RpcErrorCodes::SISTER_UNAVAILABLE,
        RpcErrorCodes::RUN_FAILED,
        RpcErrorCodes::APPROVAL_REQUIRED,
        RpcErrorCodes::CAPABILITY_DENIED,
        RpcErrorCodes::RESOURCE_EXHAUSTED,
        RpcErrorCodes::TIMEOUT,
    ];
    let mut seen = std::collections::HashSet::new();
    for code in &codes {
        assert!(seen.insert(code), "duplicate RPC error code: {}", code);
    }
}

// === SSE event formatting ===

#[test]
fn test_sse_event_format() {
    let event = SseEvent::new(
        SseEventType::RunStarted,
        serde_json::json!({
            "run_id": "test-123",
            "intent": "hello"
        }),
    );
    let sse_str = event.to_sse_string();
    assert!(sse_str.starts_with("event:"));
    assert!(sse_str.contains("data:"));
    assert!(sse_str.contains("test-123"));
}

#[test]
fn test_sse_heartbeat_format() {
    let event = SseEvent::heartbeat();
    let sse_str = event.to_sse_string();
    assert!(sse_str.contains("heartbeat"));
}

#[test]
fn test_sse_system_ready() {
    let event = SseEvent::system_ready("1.0.0");
    let sse_str = event.to_sse_string();
    assert!(sse_str.contains("1.0.0"));
}

#[test]
fn test_sse_system_shutdown() {
    let event = SseEvent::system_shutdown("user requested");
    let sse_str = event.to_sse_string();
    assert!(sse_str.contains("user requested"));
}

// === JSON-RPC request validation ===

#[test]
fn test_rpc_request_valid() {
    let req = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: serde_json::json!(1),
        method: "hydra.run".into(),
        params: serde_json::json!({"intent": "test"}),
    };
    assert!(req.is_valid());
}

#[test]
fn test_rpc_request_empty_method() {
    let req = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: serde_json::json!(1),
        method: "".into(),
        params: serde_json::json!({}),
    };
    assert!(!req.is_valid());
}

#[test]
fn test_rpc_response_schema() {
    let resp = JsonRpcResponse::success(serde_json::json!(1), serde_json::json!({"run_id": "abc"}));
    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains("jsonrpc"));
    assert!(json.contains("2.0"));
    assert!(json.contains("result"));
}

#[test]
fn test_rpc_error_response_schema() {
    let resp = JsonRpcResponse::error(serde_json::json!(1), -32601, "method not found");
    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains("error"));
    assert!(json.contains("-32601"));
}

// === Event bus ===

#[test]
fn test_event_bus_multiple_subscribers() {
    let bus = EventBus::new(100);
    let mut rx1 = bus.subscribe();
    let mut rx2 = bus.subscribe();
    bus.publish(SseEvent::heartbeat());
    assert!(rx1.try_recv().is_ok());
    assert!(rx2.try_recv().is_ok());
}

#[test]
fn test_event_bus_overflow() {
    let bus = EventBus::new(2); // tiny buffer
    bus.publish(SseEvent::heartbeat());
    bus.publish(SseEvent::heartbeat());
    bus.publish(SseEvent::heartbeat()); // may overflow
    assert!(bus.total_published() >= 3);
}

// === Run lifecycle ===

#[test]
fn test_sse_all_event_types() {
    let types = vec![
        SseEventType::RunStarted,
        SseEventType::StepStarted,
        SseEventType::StepProgress,
        SseEventType::StepCompleted,
        SseEventType::ApprovalRequired,
        SseEventType::RunCompleted,
        SseEventType::RunError,
        SseEventType::Heartbeat,
        SseEventType::SystemReady,
        SseEventType::SystemShutdown,
    ];
    for t in types {
        let event = SseEvent::new(t, serde_json::json!({}));
        let s = event.to_sse_string();
        assert!(!s.is_empty());
    }
}
