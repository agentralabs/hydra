//! MCP JSON-RPC protocol messages.

use serde::{Deserialize, Serialize};

/// JSON-RPC 2.0 request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC error object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// JSON-RPC notification (no id)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// MCP protocol version
pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";

/// Standard MCP methods
pub mod methods {
    pub const INITIALIZE: &str = "initialize";
    pub const INITIALIZED: &str = "notifications/initialized";
    pub const TOOLS_LIST: &str = "tools/list";
    pub const TOOLS_CALL: &str = "tools/call";
    pub const RESOURCES_LIST: &str = "resources/list";
    pub const RESOURCES_READ: &str = "resources/read";
    pub const PROMPTS_LIST: &str = "prompts/list";
    pub const PROMPTS_GET: &str = "prompts/get";
}

/// MCP tool definition from tools/list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default, rename = "inputSchema")]
    pub input_schema: Option<serde_json::Value>,
}

/// MCP resource definition from resources/list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default, rename = "mimeType")]
    pub mime_type: Option<String>,
}

/// MCP prompt definition from prompts/list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPrompt {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub arguments: Vec<McpPromptArgument>,
}

/// Prompt argument definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptArgument {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub required: bool,
}

/// MCP server capabilities from initialize response
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpCapabilities {
    #[serde(default)]
    pub tools: Option<serde_json::Value>,
    #[serde(default)]
    pub resources: Option<serde_json::Value>,
    #[serde(default)]
    pub prompts: Option<serde_json::Value>,
}

/// MCP server info from initialize response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerInfo {
    pub name: String,
    pub version: String,
}

/// Result of an MCP tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolResult {
    pub content: Vec<McpContent>,
    #[serde(default, rename = "isError")]
    pub is_error: bool,
}

/// Content item from tool result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
    #[serde(rename = "resource")]
    Resource { resource: serde_json::Value },
}

impl JsonRpcRequest {
    pub fn new(id: u64, method: &str, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::json!(id)),
            method: method.into(),
            params,
        }
    }

    /// Create an initialize request
    pub fn initialize(id: u64, client_name: &str, client_version: &str) -> Self {
        Self::new(
            id,
            methods::INITIALIZE,
            Some(serde_json::json!({
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {
                    "name": client_name,
                    "version": client_version,
                }
            })),
        )
    }

    /// Create a tools/list request
    pub fn tools_list(id: u64) -> Self {
        Self::new(id, methods::TOOLS_LIST, None)
    }

    /// Create a tools/call request
    pub fn tools_call(id: u64, name: &str, arguments: serde_json::Value) -> Self {
        Self::new(
            id,
            methods::TOOLS_CALL,
            Some(serde_json::json!({
                "name": name,
                "arguments": arguments,
            })),
        )
    }

    /// Create a resources/list request
    pub fn resources_list(id: u64) -> Self {
        Self::new(id, methods::RESOURCES_LIST, None)
    }

    /// Create a resources/read request
    pub fn resources_read(id: u64, uri: &str) -> Self {
        Self::new(
            id,
            methods::RESOURCES_READ,
            Some(serde_json::json!({
                "uri": uri,
            })),
        )
    }

    /// Create a prompts/list request
    pub fn prompts_list(id: u64) -> Self {
        Self::new(id, methods::PROMPTS_LIST, None)
    }

    /// Create a prompts/get request
    pub fn prompts_get(id: u64, name: &str, arguments: Option<serde_json::Value>) -> Self {
        let mut params = serde_json::json!({ "name": name });
        if let Some(args) = arguments {
            params["arguments"] = args;
        }
        Self::new(id, methods::PROMPTS_GET, Some(params))
    }
}

impl JsonRpcNotification {
    pub fn initialized() -> Self {
        Self {
            jsonrpc: "2.0".into(),
            method: methods::INITIALIZED.into(),
            params: None,
        }
    }
}

impl JsonRpcResponse {
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }

    pub fn into_result(self) -> Result<serde_json::Value, JsonRpcError> {
        if let Some(err) = self.error {
            Err(err)
        } else {
            Ok(self.result.unwrap_or(serde_json::Value::Null))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialization() {
        let req = JsonRpcRequest::initialize(1, "hydra", "0.1.0");
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"initialize\""));
        assert!(json.contains("\"protocolVersion\""));
    }

    #[test]
    fn test_tools_call_request() {
        let req =
            JsonRpcRequest::tools_call(2, "memory_add", serde_json::json!({"content": "hello"}));
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"method\":\"tools/call\""));
        assert!(json.contains("\"memory_add\""));
    }

    #[test]
    fn test_response_parsing() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[{"name":"test","description":"A test tool"}]}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert!(!resp.is_error());
        let result = resp.into_result().unwrap();
        assert!(result["tools"].is_array());
    }

    #[test]
    fn test_error_response() {
        let json =
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert!(resp.is_error());
        let err = resp.into_result().unwrap_err();
        assert_eq!(err.code, -32601);
    }

    #[test]
    fn test_notification() {
        let notif = JsonRpcNotification::initialized();
        let json = serde_json::to_string(&notif).unwrap();
        assert!(json.contains("notifications/initialized"));
        assert!(!json.contains("\"id\""));
    }
}
