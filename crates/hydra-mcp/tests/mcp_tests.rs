//! Integration tests for hydra-mcp.

use hydra_mcp::connector::ServerConnector;
use hydra_mcp::introspect::ToolIntrospector;
use hydra_mcp::invoker::DynamicInvoker;
use hydra_mcp::protocol::{JsonRpcRequest, JsonRpcResponse, McpTool};
use hydra_mcp::registry::{ServerRegistry, ServerStatus};
use hydra_mcp::schema::SchemaValidator;
use hydra_mcp::transport::TransportConfig;

// === Registry Tests ===

#[test]
fn test_registry_add_server() {
    let registry = ServerRegistry::new();
    let id = registry.add(
        "memory-server",
        TransportConfig::stdio("agentic-memory-mcp", vec![]),
    );
    let entry = registry.get(&id).unwrap();
    assert_eq!(entry.name, "memory-server");
    assert_eq!(entry.status, ServerStatus::Registered);
    assert_eq!(registry.count(), 1);
}

#[test]
fn test_registry_remove_server() {
    let registry = ServerRegistry::new();
    let id = registry.add(
        "temp-server",
        TransportConfig::http("http://localhost:3000"),
    );
    assert_eq!(registry.count(), 1);
    assert!(registry.remove(&id));
    assert_eq!(registry.count(), 0);
    assert!(registry.get(&id).is_none());
}

// === Connector Tests ===

#[test]
fn test_connector_stdio() {
    let connector = ServerConnector::new();
    let config = TransportConfig::stdio("echo", vec![]);
    let info = connector.connect("stdio-server", &config).unwrap();
    assert!(info.connected);
    assert_eq!(info.transport_type, "stdio");
    assert_eq!(connector.connection_count(), 1);
}

#[test]
fn test_connector_http() {
    let connector = ServerConnector::new();
    let config = TransportConfig::http("http://localhost:8080");
    let info = connector.connect("http-server", &config).unwrap();
    assert!(info.connected);
    assert_eq!(info.transport_type, "http");
}

#[test]
fn test_connector_websocket() {
    let connector = ServerConnector::new();
    let config = TransportConfig::websocket("ws://localhost:9090");
    let info = connector.connect("ws-server", &config).unwrap();
    assert!(info.connected);
    assert_eq!(info.transport_type, "websocket");
}

// === Introspection Tests ===

fn setup_connected_server() -> (ServerConnector, ServerRegistry, String) {
    let connector = ServerConnector::new();
    let registry = ServerRegistry::new();
    let config = TransportConfig::stdio("test-server", vec![]);
    let server_id = registry.add("test-server", config.clone());
    connector.connect(&server_id, &config).unwrap();
    (connector, registry, server_id)
}

#[test]
fn test_introspect_tools() {
    let (connector, _registry, server_id) = setup_connected_server();
    let tools = ToolIntrospector::discover_tools(&server_id, &connector).unwrap();
    assert!(!tools.is_empty());
    assert!(tools.iter().any(|t| t.name == "test_tool"));
}

#[test]
fn test_introspect_resources() {
    let (connector, _registry, server_id) = setup_connected_server();
    let resources = ToolIntrospector::discover_resources(&server_id, &connector).unwrap();
    assert!(!resources.is_empty());
}

#[test]
fn test_introspect_prompts() {
    let (connector, _registry, server_id) = setup_connected_server();
    let prompts = ToolIntrospector::discover_prompts(&server_id, &connector).unwrap();
    assert!(!prompts.is_empty());
    assert!(prompts.iter().any(|p| p.name == "test_prompt"));
}

// === Dynamic Invocation Tests ===

fn setup_with_tools() -> (DynamicInvoker, ServerConnector, ServerRegistry, String) {
    let invoker = DynamicInvoker::new();
    let connector = ServerConnector::new();
    let registry = ServerRegistry::new();
    let config = TransportConfig::stdio("test-server", vec![]);
    let server_id = registry.add_with_id("test-srv", "test-server", config.clone());
    connector.connect(&server_id, &config).unwrap();
    registry.set_tools(
        &server_id,
        vec![McpTool {
            name: "memory_add".into(),
            description: "Add a memory".into(),
            input_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "content": { "type": "string" }
                },
                "required": ["content"]
            })),
        }],
    );
    (invoker, connector, registry, server_id)
}

#[test]
fn test_dynamic_invoke() {
    let (invoker, connector, registry, server_id) = setup_with_tools();
    let result = invoker
        .invoke(
            &server_id,
            "memory_add",
            serde_json::json!({"content": "hello world"}),
            &connector,
            &registry,
        )
        .unwrap();
    assert!(result.success);
    assert_eq!(result.tool_name, "memory_add");
}

#[test]
fn test_invoke_validation() {
    let (invoker, connector, registry, server_id) = setup_with_tools();
    // Missing required "content" field
    let result = invoker.invoke(
        &server_id,
        "memory_add",
        serde_json::json!({}),
        &connector,
        &registry,
    );
    assert!(result.is_err());
}

// === Schema Tests ===

#[test]
fn test_schema_validator() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "query": { "type": "string", "minLength": 1 },
            "limit": { "type": "number", "minimum": 1, "maximum": 100 }
        },
        "required": ["query"]
    });

    // Valid
    let valid = serde_json::json!({"query": "test", "limit": 10});
    assert!(SchemaValidator::validate(&valid, &schema).valid);

    // Missing required
    let missing = serde_json::json!({"limit": 10});
    assert!(!SchemaValidator::validate(&missing, &schema).valid);

    // Out of range
    let out_of_range = serde_json::json!({"query": "test", "limit": 200});
    assert!(!SchemaValidator::validate(&out_of_range, &schema).valid);
}

// === Protocol Tests ===

#[test]
fn test_protocol_messages() {
    let init = JsonRpcRequest::initialize(1, "hydra", "0.1.0");
    assert_eq!(init.method, "initialize");

    let tools_list = JsonRpcRequest::tools_list(2);
    assert_eq!(tools_list.method, "tools/list");

    let tools_call =
        JsonRpcRequest::tools_call(3, "memory_add", serde_json::json!({"content": "test"}));
    assert_eq!(tools_call.method, "tools/call");

    let resources = JsonRpcRequest::resources_list(4);
    assert_eq!(resources.method, "resources/list");

    let prompts = JsonRpcRequest::prompts_list(5);
    assert_eq!(prompts.method, "prompts/list");
}

// === Error Handling Tests ===

#[test]
fn test_error_handling() {
    let json = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32803,"message":"Tool not found","data":{"tool":"unknown"}}}"#;
    let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
    assert!(resp.is_error());
    let err = resp.into_result().unwrap_err();
    assert_eq!(err.code, -32803);
    assert!(err.data.is_some());
}

// === Reconnection Tests ===

#[test]
fn test_reconnection() {
    let connector = ServerConnector::new();
    let config = TransportConfig::stdio("echo", vec![]);

    // Connect
    connector.connect("srv", &config).unwrap();
    assert!(connector.is_connected("srv"));

    // Disconnect
    connector.disconnect("srv");
    assert!(!connector.is_connected("srv"));

    // Reconnect
    connector.connect("srv", &config).unwrap();
    assert!(connector.is_connected("srv"));
}

// === Concurrent Calls Test ===

#[test]
fn test_concurrent_calls() {
    let connector = ServerConnector::new();
    let registry = ServerRegistry::new();

    // Register multiple servers
    for i in 0..5 {
        let name = format!("server-{}", i);
        let config = TransportConfig::stdio(&format!("cmd-{}", i), vec![]);
        let id = registry.add(&name, config.clone());
        connector.connect(&id, &config).unwrap();
        registry.set_status(&id, ServerStatus::Connected);
    }

    assert_eq!(registry.count(), 5);
    assert_eq!(registry.connected_count(), 5);

    // All should be independently queryable
    let connected = registry.list_by_status(ServerStatus::Connected);
    assert_eq!(connected.len(), 5);
}
