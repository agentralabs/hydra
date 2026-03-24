//! McpServer — exposes Hydra tools to MCP clients.
//! Handles initialize, tools/list, tools/call, and notifications.

use crate::constants::{PROTOCOL_VERSION, SERVER_NAME, SERVER_VERSION};
use crate::errors::{McpError, TOOL_NOT_FOUND};
use crate::protocol::{JsonRpcRequest, JsonRpcResponse, ToolResult};
use crate::tools;
use crate::transport::Transport;

use std::sync::Arc;

/// Callback for tool execution — the kernel provides this.
pub type ToolHandler =
    Arc<dyn Fn(&str, serde_json::Value) -> ToolResult + Send + Sync>;

/// MCP server that exposes Hydra tools.
pub struct McpServer {
    tool_handler: Option<ToolHandler>,
    initialized: bool,
}

impl McpServer {
    pub fn new() -> Self {
        Self {
            tool_handler: None,
            initialized: false,
        }
    }

    /// Set the tool execution handler (provided by kernel at boot).
    pub fn set_handler(&mut self, handler: ToolHandler) {
        self.tool_handler = Some(handler);
    }

    /// Handle a single JSON-RPC request and return a response.
    pub fn handle_request(&mut self, request: &JsonRpcRequest) -> JsonRpcResponse {
        match request.method.as_str() {
            "initialize" => self.handle_initialize(&request.id),
            "tools/list" => self.handle_tools_list(&request.id),
            "tools/call" => self.handle_tools_call(&request.id, &request.params),
            "ping" => JsonRpcResponse::success(
                request.id.clone(),
                serde_json::json!({}),
            ),
            "notifications/initialized" => {
                // Notification — no response needed, but return success
                JsonRpcResponse::success(request.id.clone(), serde_json::json!({}))
            }
            _ => JsonRpcResponse::error(
                request.id.clone(),
                crate::errors::METHOD_NOT_FOUND,
                &format!("Unknown method: {}", request.method),
            ),
        }
    }

    /// Run the server loop on a transport.
    pub async fn run<T: Transport>(&mut self, transport: &T) -> Result<(), McpError> {
        eprintln!("hydra-mcp: server starting on {}", transport.name());

        loop {
            let message = match transport.receive().await {
                Ok(m) => m,
                Err(McpError::TransportError(e)) if e.contains("EOF") => {
                    eprintln!("hydra-mcp: client disconnected");
                    break;
                }
                Err(e) => {
                    eprintln!("hydra-mcp: receive error: {e}");
                    continue;
                }
            };

            let request: JsonRpcRequest = match serde_json::from_str(&message) {
                Ok(r) => r,
                Err(e) => {
                    let error_resp = JsonRpcResponse::error(
                        serde_json::json!(null),
                        crate::errors::PARSE_ERROR,
                        &format!("Parse error: {e}"),
                    );
                    let json = serde_json::to_string(&error_resp).unwrap_or_default();
                    let _ = transport.send(&json).await;
                    continue;
                }
            };

            let response = self.handle_request(&request);
            let json = serde_json::to_string(&response).map_err(|e| {
                McpError::SerializationError(e.to_string())
            })?;
            transport.send(&json).await?;
        }

        Ok(())
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    fn handle_initialize(&mut self, id: &serde_json::Value) -> JsonRpcResponse {
        self.initialized = true;
        eprintln!("hydra-mcp: initialized");
        JsonRpcResponse::success(
            id.clone(),
            serde_json::json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": SERVER_NAME,
                    "version": SERVER_VERSION
                }
            }),
        )
    }

    fn handle_tools_list(&self, id: &serde_json::Value) -> JsonRpcResponse {
        let tool_defs = tools::hydra_tools();
        JsonRpcResponse::success(
            id.clone(),
            serde_json::json!({
                "tools": tool_defs
            }),
        )
    }

    fn handle_tools_call(
        &self,
        id: &serde_json::Value,
        params: &Option<serde_json::Value>,
    ) -> JsonRpcResponse {
        let params = match params {
            Some(p) => p,
            None => {
                return JsonRpcResponse::error(
                    id.clone(),
                    crate::errors::INVALID_PARAMS,
                    "Missing params",
                );
            }
        };

        let tool_name = params
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or(serde_json::json!({}));

        // Check tool exists
        if tools::find_tool(tool_name).is_none() {
            return JsonRpcResponse::error(
                id.clone(),
                TOOL_NOT_FOUND,
                &format!("Tool not found: {tool_name}"),
            );
        }

        // Execute via handler
        let result = if let Some(handler) = &self.tool_handler {
            handler(tool_name, arguments)
        } else {
            // No handler set — return placeholder
            ToolResult::text(format!("Tool {tool_name} called (no handler configured)"))
        };

        JsonRpcResponse::success(id.clone(), serde_json::to_value(result).unwrap_or_default())
    }
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_initialize() {
        let mut server = McpServer::new();
        let req = JsonRpcRequest::new(1, "initialize");
        let resp = server.handle_request(&req);
        assert!(!resp.is_error());
        assert!(server.is_initialized());
    }

    #[test]
    fn handle_tools_list() {
        let mut server = McpServer::new();
        let req = JsonRpcRequest::new(1, "tools/list");
        let resp = server.handle_request(&req);
        assert!(!resp.is_error());
        let result = resp.result.unwrap();
        let tools = result.get("tools").unwrap().as_array().unwrap();
        assert_eq!(tools.len(), 8);
    }

    #[test]
    fn handle_unknown_tool() {
        let mut server = McpServer::new();
        let req = JsonRpcRequest::new(1, "tools/call")
            .with_params(serde_json::json!({
                "name": "nonexistent_tool",
                "arguments": {}
            }));
        let resp = server.handle_request(&req);
        assert!(resp.is_error());
        assert_eq!(resp.error.unwrap().code, TOOL_NOT_FOUND);
    }

    #[test]
    fn handle_valid_tool_call() {
        let mut server = McpServer::new();
        server.set_handler(Arc::new(|name, _args| {
            ToolResult::text(format!("Executed {name}"))
        }));

        let req = JsonRpcRequest::new(1, "tools/call")
            .with_params(serde_json::json!({
                "name": "hydra_status",
                "arguments": {}
            }));
        let resp = server.handle_request(&req);
        assert!(!resp.is_error());
    }

    #[test]
    fn handle_unknown_method() {
        let mut server = McpServer::new();
        let req = JsonRpcRequest::new(1, "unknown/method");
        let resp = server.handle_request(&req);
        assert!(resp.is_error());
    }

    #[test]
    fn handle_ping() {
        let mut server = McpServer::new();
        let req = JsonRpcRequest::new(1, "ping");
        let resp = server.handle_request(&req);
        assert!(!resp.is_error());
    }
}
