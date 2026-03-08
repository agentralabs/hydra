//! Category 7: Contract Tests — JSON-RPC protocol.

use hydra_runtime::*;

#[test]
fn test_response_schema_valid() {
    let resp = JsonRpcResponse::success(serde_json::json!(1), serde_json::json!({"ok": true}));
    let json = serde_json::to_value(&resp).unwrap();
    assert_eq!(json["jsonrpc"], "2.0");
    assert!(json.get("result").is_some());
    assert!(json.get("error").is_none());
}

#[test]
fn test_error_codes_consistent() {
    // All standard JSON-RPC error codes should be negative
    assert!(RpcErrorCodes::PARSE_ERROR < 0);
    assert!(RpcErrorCodes::INVALID_REQUEST < 0);
    assert!(RpcErrorCodes::METHOD_NOT_FOUND < 0);
    assert!(RpcErrorCodes::INVALID_PARAMS < 0);
    assert!(RpcErrorCodes::INTERNAL_ERROR < 0);
    // Custom codes also negative
    assert!(RpcErrorCodes::SISTER_UNAVAILABLE < 0);
    assert!(RpcErrorCodes::RUN_FAILED < 0);
}

#[test]
fn test_error_response_has_error_field() {
    let resp = JsonRpcResponse::error(serde_json::json!(1), -32601, "method not found".into());
    let json = serde_json::to_value(&resp).unwrap();
    assert!(json.get("error").is_some());
    assert!(json.get("result").is_none());
    assert_eq!(json["error"]["code"], -32601);
}

#[test]
fn test_request_roundtrip() {
    let req = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: serde_json::json!("abc-123"),
        method: "hydra.run".into(),
        params: serde_json::json!({"intent": "test"}),
    };
    let json = serde_json::to_string(&req).unwrap();
    let restored: JsonRpcRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.method, "hydra.run");
    assert_eq!(restored.id, serde_json::json!("abc-123"));
}

#[test]
fn test_all_hydra_methods_prefixed() {
    let methods = vec![
        "hydra.run",
        "hydra.status",
        "hydra.kill",
        "hydra.approve",
        "hydra.deny",
        "hydra.config",
    ];
    for method in methods {
        assert!(method.starts_with("hydra."), "{} should be prefixed with hydra.", method);
    }
}
