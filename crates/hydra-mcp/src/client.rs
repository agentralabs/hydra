//! McpClient — connects to external MCP servers and discovers/calls tools.

use crate::constants::MAX_DISCOVERED_TOOLS;
use crate::errors::McpError;
use crate::protocol::{JsonRpcRequest, JsonRpcResponse, ToolDefinition, ToolResult};
use crate::transport::Transport;

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// MCP client for connecting to external tool servers.
pub struct McpClient {
    tools_cache: HashMap<String, ToolDefinition>,
    next_id: AtomicU64,
    initialized: bool,
    server_name: String,
}

impl McpClient {
    pub fn new() -> Self {
        Self {
            tools_cache: HashMap::new(),
            next_id: AtomicU64::new(1),
            initialized: false,
            server_name: String::new(),
        }
    }

    /// Initialize the connection with a server via a transport.
    pub async fn initialize<T: Transport>(&mut self, transport: &T) -> Result<(), McpError> {
        let req = self.make_request("initialize")
            .with_params(serde_json::json!({
                "protocolVersion": crate::constants::PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {
                    "name": "hydra",
                    "version": crate::constants::SERVER_VERSION
                }
            }));

        let resp = self.send_request(transport, &req).await?;

        if resp.is_error() {
            return Err(McpError::ProtocolError(
                resp.error.map(|e| e.message).unwrap_or_default(),
            ));
        }

        if let Some(result) = &resp.result {
            self.server_name = result
                .get("serverInfo")
                .and_then(|s| s.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or("unknown")
                .to_string();
        }

        self.initialized = true;
        eprintln!("hydra-mcp: client connected to '{}'", self.server_name);

        // Send initialized notification
        let notif = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });
        let json = serde_json::to_string(&notif).map_err(|e| {
            McpError::SerializationError(e.to_string())
        })?;
        transport.send(&json).await?;

        Ok(())
    }

    /// Discover tools from the connected server.
    pub async fn discover_tools<T: Transport>(
        &mut self,
        transport: &T,
    ) -> Result<Vec<ToolDefinition>, McpError> {
        if !self.initialized {
            return Err(McpError::NotConnected("not initialized".into()));
        }

        let req = self.make_request("tools/list");
        let resp = self.send_request(transport, &req).await?;

        if resp.is_error() {
            return Err(McpError::ProtocolError(
                resp.error.map(|e| e.message).unwrap_or_default(),
            ));
        }

        let tools: Vec<ToolDefinition> = resp
            .result
            .and_then(|r| r.get("tools").cloned())
            .and_then(|t| serde_json::from_value(t).ok())
            .unwrap_or_default();

        // Cache the tools
        for tool in &tools {
            if self.tools_cache.len() < MAX_DISCOVERED_TOOLS {
                self.tools_cache.insert(tool.name.clone(), tool.clone());
            }
        }

        eprintln!(
            "hydra-mcp: discovered {} tools from '{}'",
            tools.len(),
            self.server_name
        );

        Ok(tools)
    }

    /// Call a tool on the connected server.
    pub async fn call_tool<T: Transport>(
        &self,
        transport: &T,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<ToolResult, McpError> {
        if !self.initialized {
            return Err(McpError::NotConnected("not initialized".into()));
        }

        if !self.tools_cache.contains_key(tool_name) {
            return Err(McpError::ToolNotFound(tool_name.into()));
        }

        let req = self.make_request("tools/call").with_params(serde_json::json!({
            "name": tool_name,
            "arguments": arguments
        }));

        let resp = self.send_request(transport, &req).await?;

        if resp.is_error() {
            let msg = resp.error.map(|e| e.message).unwrap_or_default();
            return Err(McpError::ToolFailed {
                tool: tool_name.into(),
                reason: msg,
            });
        }

        let result: ToolResult = resp
            .result
            .and_then(|r| serde_json::from_value(r).ok())
            .unwrap_or_else(|| ToolResult::text("(empty result)"));

        Ok(result)
    }

    /// Get cached tool definitions.
    pub fn cached_tools(&self) -> Vec<&ToolDefinition> {
        self.tools_cache.values().collect()
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn server_name(&self) -> &str {
        &self.server_name
    }

    fn make_request(&self, method: &str) -> JsonRpcRequest {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        JsonRpcRequest::new(id, method)
    }

    async fn send_request<T: Transport>(
        &self,
        transport: &T,
        request: &JsonRpcRequest,
    ) -> Result<JsonRpcResponse, McpError> {
        let json = serde_json::to_string(request).map_err(|e| {
            McpError::SerializationError(e.to_string())
        })?;

        transport.send(&json).await?;
        let response_str = transport.receive().await?;

        serde_json::from_str(&response_str).map_err(|e| {
            McpError::ProtocolError(format!("Invalid response: {e}"))
        })
    }
}

impl Default for McpClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::McpServer;
    use crate::transport::MemoryTransport;
    use std::sync::Arc;

    async fn setup_connected_pair() -> (McpClient, McpServer, MemoryTransport, MemoryTransport) {
        let (client_transport, server_transport) = MemoryTransport::pair();
        let client = McpClient::new();
        let server = McpServer::new();
        (client, server, client_transport, server_transport)
    }

    #[tokio::test]
    async fn initialize_handshake() {
        let (mut client, mut server, ct, st) = setup_connected_pair().await;

        // Client sends initialize
        let init_task = tokio::spawn(async move {
            client.initialize(&ct).await.unwrap();
            assert!(client.is_initialized());
            (client, ct)
        });

        // Server handles it
        let msg = st.receive().await.unwrap();
        let req: JsonRpcRequest = serde_json::from_str(&msg).unwrap();
        let resp = server.handle_request(&req);
        let resp_json = serde_json::to_string(&resp).unwrap();
        st.send(&resp_json).await.unwrap();

        // Handle the initialized notification
        let _notif = st.receive().await.unwrap();

        let (_client, _ct) = init_task.await.unwrap();
    }

    #[test]
    fn client_starts_uninitialized() {
        let client = McpClient::new();
        assert!(!client.is_initialized());
        assert!(client.cached_tools().is_empty());
    }
}
