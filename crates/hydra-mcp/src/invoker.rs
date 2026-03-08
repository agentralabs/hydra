//! DynamicInvoker — call any MCP tool without compile-time types.

use serde::{Deserialize, Serialize};

use crate::connector::ServerConnector;
use crate::protocol::{JsonRpcRequest, McpContent, McpToolResult};
use crate::registry::ServerRegistry;
use crate::schema::SchemaValidator;

/// Result of a dynamic tool invocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvocationResult {
    pub server_id: String,
    pub tool_name: String,
    pub success: bool,
    pub content: Vec<McpContent>,
    pub raw: serde_json::Value,
}

impl InvocationResult {
    /// Get text content from the result
    pub fn text(&self) -> Option<&str> {
        self.content.iter().find_map(|c| match c {
            McpContent::Text { text } => Some(text.as_str()),
            _ => None,
        })
    }

    /// Get all text content concatenated
    pub fn all_text(&self) -> String {
        self.content
            .iter()
            .filter_map(|c| match c {
                McpContent::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Invocation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvocationError {
    pub server_id: String,
    pub tool_name: String,
    pub kind: InvocationErrorKind,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InvocationErrorKind {
    ServerNotFound,
    ToolNotFound,
    ValidationFailed,
    TransportError,
    ToolError,
}

impl std::fmt::Display for InvocationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}] {}: {}",
            self.server_id, self.tool_name, self.message
        )
    }
}

/// Dynamic invoker for calling any MCP tool
pub struct DynamicInvoker {
    validate_inputs: bool,
}

impl DynamicInvoker {
    pub fn new() -> Self {
        Self {
            validate_inputs: true,
        }
    }

    pub fn with_validation(mut self, enabled: bool) -> Self {
        self.validate_inputs = enabled;
        self
    }

    /// Invoke a tool on a server
    pub fn invoke(
        &self,
        server_id: &str,
        tool_name: &str,
        arguments: serde_json::Value,
        connector: &ServerConnector,
        registry: &ServerRegistry,
    ) -> Result<InvocationResult, InvocationError> {
        // 1. Verify server exists
        let entry = registry.get(server_id).ok_or_else(|| InvocationError {
            server_id: server_id.into(),
            tool_name: tool_name.into(),
            kind: InvocationErrorKind::ServerNotFound,
            message: "server not found in registry".into(),
        })?;

        // 2. Verify tool exists on server
        let tool = entry
            .tools
            .iter()
            .find(|t| t.name == tool_name)
            .ok_or_else(|| InvocationError {
                server_id: server_id.into(),
                tool_name: tool_name.into(),
                kind: InvocationErrorKind::ToolNotFound,
                message: format!("tool '{}' not found on server '{}'", tool_name, server_id),
            })?;

        // 3. Validate inputs against schema
        if self.validate_inputs {
            if let Some(ref schema) = tool.input_schema {
                let validation = SchemaValidator::validate(&arguments, schema);
                if !validation.valid {
                    return Err(InvocationError {
                        server_id: server_id.into(),
                        tool_name: tool_name.into(),
                        kind: InvocationErrorKind::ValidationFailed,
                        message: format!(
                            "input validation failed: {}",
                            validation.errors.join(", ")
                        ),
                    });
                }
            }
        }

        // 4. Send tools/call request
        let req = JsonRpcRequest::tools_call(5, tool_name, arguments);
        let resp = connector
            .send_request(server_id, &req)
            .map_err(|e| InvocationError {
                server_id: server_id.into(),
                tool_name: tool_name.into(),
                kind: InvocationErrorKind::TransportError,
                message: e,
            })?;

        let result = resp.into_result().map_err(|e| InvocationError {
            server_id: server_id.into(),
            tool_name: tool_name.into(),
            kind: InvocationErrorKind::ToolError,
            message: e.message,
        })?;

        // 5. Parse result
        let tool_result: McpToolResult =
            serde_json::from_value(result.clone()).unwrap_or(McpToolResult {
                content: vec![McpContent::Text {
                    text: result.to_string(),
                }],
                is_error: false,
            });

        Ok(InvocationResult {
            server_id: server_id.into(),
            tool_name: tool_name.into(),
            success: !tool_result.is_error,
            content: tool_result.content,
            raw: result,
        })
    }

    /// Invoke a tool by searching all registered servers for it
    pub fn invoke_any(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
        connector: &ServerConnector,
        registry: &ServerRegistry,
    ) -> Result<InvocationResult, InvocationError> {
        let servers = registry.find_by_tool(tool_name);
        if servers.is_empty() {
            return Err(InvocationError {
                server_id: String::new(),
                tool_name: tool_name.into(),
                kind: InvocationErrorKind::ToolNotFound,
                message: format!("no server found with tool '{}'", tool_name),
            });
        }

        // Try the first server that has the tool
        self.invoke(&servers[0].id, tool_name, arguments, connector, registry)
    }
}

impl Default for DynamicInvoker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::McpTool;
    use crate::transport::TransportConfig;

    fn setup_with_tools() -> (DynamicInvoker, ServerConnector, ServerRegistry, String) {
        let invoker = DynamicInvoker::new();
        let connector = ServerConnector::new();
        let registry = ServerRegistry::new();

        let config = TransportConfig::stdio("test-server", vec![]);
        let server_id = registry.add_with_id("test-server", "test", config.clone());
        connector.connect(&server_id, &config).unwrap();

        // Register tools
        registry.set_tools(
            &server_id,
            vec![McpTool {
                name: "test_tool".into(),
                description: "A test tool".into(),
                input_schema: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "input": { "type": "string" }
                    },
                    "required": ["input"]
                })),
            }],
        );

        (invoker, connector, registry, server_id)
    }

    #[test]
    fn test_invoke_success() {
        let (invoker, connector, registry, server_id) = setup_with_tools();
        let result = invoker
            .invoke(
                &server_id,
                "test_tool",
                serde_json::json!({"input": "hello"}),
                &connector,
                &registry,
            )
            .unwrap();

        assert!(result.success);
        assert_eq!(result.tool_name, "test_tool");
    }

    #[test]
    fn test_invoke_server_not_found() {
        let (invoker, connector, registry, _) = setup_with_tools();
        let result = invoker.invoke(
            "nonexistent",
            "tool",
            serde_json::json!({}),
            &connector,
            &registry,
        );
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().kind,
            InvocationErrorKind::ServerNotFound
        ));
    }

    #[test]
    fn test_invoke_tool_not_found() {
        let (invoker, connector, registry, server_id) = setup_with_tools();
        let result = invoker.invoke(
            &server_id,
            "nonexistent_tool",
            serde_json::json!({}),
            &connector,
            &registry,
        );
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().kind,
            InvocationErrorKind::ToolNotFound
        ));
    }

    #[test]
    fn test_invoke_validation_failure() {
        let (invoker, connector, registry, server_id) = setup_with_tools();
        // Missing required "input" field
        let result = invoker.invoke(
            &server_id,
            "test_tool",
            serde_json::json!({}),
            &connector,
            &registry,
        );
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().kind,
            InvocationErrorKind::ValidationFailed
        ));
    }
}
