//! Category 7: Contract Tests — MCP protocol.

use hydra_mcp::protocol::*;

#[test]
fn test_initialize_handshake() {
    let req = JsonRpcRequest::initialize("hydra", "1.0.0");
    assert_eq!(req.method, "initialize");
    assert!(req.params.get("clientInfo").is_some());
    assert_eq!(req.params["protocolVersion"], MCP_PROTOCOL_VERSION);
}

#[test]
fn test_tools_list_response_schema() {
    let tools = vec![
        McpTool {
            name: "memory_add".into(),
            description: "Add a memory entry".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": { "content": { "type": "string" } },
                "required": ["content"]
            }),
        },
    ];
    let json = serde_json::to_value(&tools).unwrap();
    assert!(json.is_array());
    assert_eq!(json[0]["name"], "memory_add");
    assert!(json[0]["inputSchema"].is_object());
}

#[test]
fn test_tools_call_request() {
    let req = JsonRpcRequest::tools_call("memory_add", serde_json::json!({"content": "test"}));
    assert_eq!(req.method, "tools/call");
    assert_eq!(req.params["name"], "memory_add");
    assert!(req.params.get("arguments").is_some());
}

#[test]
fn test_resources_list_request() {
    let req = JsonRpcRequest::resources_list();
    assert_eq!(req.method, "resources/list");
}

#[test]
fn test_mcp_tool_result() {
    let result = McpToolResult {
        content: vec![McpContent::Text { text: "result data".into() }],
        is_error: false,
    };
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);
}
