//! Category 1: Unit Gap Fill — hydra-mcp edge cases.

use hydra_mcp::*;

// === Transport reconnection all types ===

#[test]
fn test_stdio_transport_status() {
    let config = transport::TransportConfig::stdio("echo", vec!["test".into()]);
    let transport = transport::stdio::StdioTransport::new(
        transport::stdio::StdioConfig::from_transport(&config).unwrap(),
    );
    assert_eq!(transport.status(), transport::TransportStatus::Disconnected);
}

#[test]
fn test_http_transport_invalid_url() {
    let config = transport::TransportConfig::http("not-a-url");
    let t = transport::http::HttpTransport::new(
        transport::http::HttpConfig::from_transport(&config).unwrap(),
    );
    assert_eq!(t.status(), transport::TransportStatus::Disconnected);
}

#[test]
fn test_websocket_transport_reconnect_count() {
    let config = transport::TransportConfig::websocket("ws://localhost:9999");
    let t = transport::websocket::WebSocketTransport::new(
        transport::websocket::WebSocketConfig::from_transport(&config).unwrap(),
    );
    assert_eq!(t.reconnect_count(), 0);
    assert_eq!(t.messages_sent(), 0);
}

// === Schema validation all types ===

#[test]
fn test_schema_validate_string() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": { "name": { "type": "string", "minLength": 1 } },
        "required": ["name"]
    });
    let valid = serde_json::json!({ "name": "Alice" });
    let result = schema::SchemaValidator::validate(&valid, &schema);
    assert!(result.valid);

    let invalid = serde_json::json!({ "name": "" });
    let result = schema::SchemaValidator::validate(&invalid, &schema);
    assert!(!result.valid);
}

#[test]
fn test_schema_validate_number() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": { "age": { "type": "number", "minimum": 0, "maximum": 150 } },
        "required": ["age"]
    });
    let valid = serde_json::json!({ "age": 25 });
    assert!(schema::SchemaValidator::validate(&valid, &schema).valid);

    let invalid = serde_json::json!({ "age": -1 });
    assert!(!schema::SchemaValidator::validate(&invalid, &schema).valid);
}

#[test]
fn test_schema_validate_missing_required() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": { "name": { "type": "string" } },
        "required": ["name"]
    });
    let invalid = serde_json::json!({});
    assert!(!schema::SchemaValidator::validate(&invalid, &schema).valid);
}

#[test]
fn test_schema_validate_nested_object() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "address": {
                "type": "object",
                "properties": { "city": { "type": "string" } },
                "required": ["city"]
            }
        },
        "required": ["address"]
    });
    let valid = serde_json::json!({ "address": { "city": "NYC" } });
    assert!(schema::SchemaValidator::validate(&valid, &schema).valid);
}

// === Concurrent invocations ===

#[test]
fn test_invoker_server_not_found() {
    let registry = registry::ServerRegistry::new();
    let connector = connector::ServerConnector::new();
    let invoker = invoker::DynamicInvoker::new();
    let result = invoker.invoke(
        "nonexistent",
        "tool",
        serde_json::json!({}),
        &connector,
        &registry,
    );
    assert!(result.is_err());
}

// === Registry operations ===

#[test]
fn test_registry_add_remove() {
    let registry = registry::ServerRegistry::new();
    let id = registry.add_with_id(
        "s1",
        "Test Server",
        transport::TransportConfig::stdio("test", vec![]),
    );
    registry.add_tags(&id, vec!["test".into()]);
    assert_eq!(registry.count(), 1);
    assert!(registry.find_by_tag("test").len() == 1);

    registry.remove("s1");
    assert_eq!(registry.count(), 0);
}

#[test]
fn test_registry_find_by_tool() {
    let registry = registry::ServerRegistry::new();
    let id = registry.add_with_id(
        "s1",
        "Server",
        transport::TransportConfig::stdio("test", vec![]),
    );
    registry.set_tools(
        &id,
        vec![protocol::McpTool {
            name: "my_tool".into(),
            description: "A tool".into(),
            input_schema: Some(serde_json::json!({})),
        }],
    );
    let found = registry.find_by_tool("my_tool");
    assert_eq!(found.len(), 1);
}

// === Protocol ===

#[test]
fn test_jsonrpc_request_factory_methods() {
    let init = protocol::JsonRpcRequest::initialize(1, "client", "1.0");
    assert_eq!(init.method, "initialize");

    let tools = protocol::JsonRpcRequest::tools_list(2);
    assert_eq!(tools.method, "tools/list");

    let call = protocol::JsonRpcRequest::tools_call(3, "my_tool", serde_json::json!({}));
    assert_eq!(call.method, "tools/call");
}
