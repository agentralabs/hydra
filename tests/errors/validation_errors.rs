//! Category 6: Error Paths — validation errors.

#[test]
fn test_invalid_json_rpc_version() {
    use hydra_runtime::JsonRpcRequest;
    let req = JsonRpcRequest {
        jsonrpc: "1.0".into(),
        id: serde_json::json!(1),
        method: "test".into(),
        params: serde_json::json!({}),
    };
    assert!(!req.is_valid());
}

#[test]
fn test_invalid_json_rpc_empty_method() {
    use hydra_runtime::JsonRpcRequest;
    let req = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: serde_json::json!(1),
        method: "".into(),
        params: serde_json::json!({}),
    };
    assert!(!req.is_valid());
}

#[test]
fn test_schema_type_mismatch() {
    use hydra_mcp::schema::SchemaValidator;
    let schema = serde_json::json!({
        "type": "object",
        "properties": { "count": { "type": "number" } },
        "required": ["count"]
    });
    let invalid = serde_json::json!({ "count": "not a number" });
    let result = SchemaValidator::validate(&invalid, &schema);
    assert!(!result.valid);
}

#[test]
fn test_missing_required_field() {
    use hydra_mcp::schema::SchemaValidator;
    let schema = serde_json::json!({
        "type": "object",
        "properties": { "name": { "type": "string" } },
        "required": ["name", "age"]
    });
    let invalid = serde_json::json!({ "name": "Alice" });
    let result = SchemaValidator::validate(&invalid, &schema);
    assert!(!result.valid);
    assert!(result.errors.iter().any(|e| e.contains("age")));
}

#[test]
fn test_serialization_error_from_json() {
    let bad_json = "not valid json at all";
    let result: Result<serde_json::Value, _> = serde_json::from_str(bad_json);
    assert!(result.is_err());

    let err: hydra_core::HydraError = result.unwrap_err().into();
    match err {
        hydra_core::HydraError::SerializationError(_) => {}
        _ => panic!("expected SerializationError"),
    }
}

#[test]
fn test_config_error() {
    let err = hydra_core::HydraError::ConfigError("missing field 'port'".into());
    assert_eq!(err.error_code(), "E501");
    assert!(!err.is_retryable());
}
