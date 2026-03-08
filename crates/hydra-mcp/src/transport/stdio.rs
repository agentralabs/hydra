//! Stdio transport — spawn a child process and communicate via stdin/stdout.

use serde::{Deserialize, Serialize};

use super::{TransportConfig, TransportStatus};

/// Stdio transport configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StdioConfig {
    pub command: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub timeout_ms: u64,
}

impl StdioConfig {
    pub fn new(command: &str, args: Vec<String>) -> Self {
        Self {
            command: command.into(),
            args,
            env: Vec::new(),
            timeout_ms: 5000,
        }
    }

    pub fn from_transport(config: &TransportConfig) -> Option<Self> {
        match config {
            TransportConfig::Stdio { command, args, env } => Some(Self {
                command: command.clone(),
                args: args.clone(),
                env: env.clone(),
                timeout_ms: 5000,
            }),
            _ => None,
        }
    }
}

/// Stdio transport handler (simulated for unit testing)
pub struct StdioTransport {
    config: StdioConfig,
    status: TransportStatus,
    request_log: Vec<String>,
    response_queue: Vec<String>,
}

impl StdioTransport {
    pub fn new(config: StdioConfig) -> Self {
        Self {
            config,
            status: TransportStatus::Disconnected,
            request_log: Vec::new(),
            response_queue: Vec::new(),
        }
    }

    /// Connect to the child process
    pub fn connect(&mut self) -> Result<(), String> {
        self.status = TransportStatus::Connected;
        Ok(())
    }

    /// Send a JSON-RPC message
    pub fn send(&mut self, message: &str) -> Result<(), String> {
        if self.status != TransportStatus::Connected {
            return Err("not connected".into());
        }
        self.request_log.push(message.to_string());

        // Generate simulated response
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(message) {
            let id = parsed.get("id").cloned();
            let method = parsed.get("method").and_then(|m| m.as_str()).unwrap_or("");
            let response = generate_simulated_response(id, method);
            self.response_queue.push(response);
        }
        Ok(())
    }

    /// Receive a JSON-RPC response
    pub fn receive(&mut self) -> Result<String, String> {
        if self.status != TransportStatus::Connected {
            return Err("not connected".into());
        }
        self.response_queue
            .pop()
            .ok_or_else(|| "no response available".into())
    }

    /// Disconnect the transport
    pub fn disconnect(&mut self) {
        self.status = TransportStatus::Disconnected;
    }

    pub fn status(&self) -> TransportStatus {
        self.status
    }

    pub fn command(&self) -> &str {
        &self.config.command
    }

    pub fn request_count(&self) -> usize {
        self.request_log.len()
    }
}

/// Generate a simulated MCP response for testing
fn generate_simulated_response(id: Option<serde_json::Value>, method: &str) -> String {
    let result = match method {
        "initialize" => serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "test-server", "version": "1.0.0" }
        }),
        "tools/list" => serde_json::json!({
            "tools": [
                { "name": "test_tool", "description": "A test tool", "inputSchema": {
                    "type": "object",
                    "properties": { "input": { "type": "string" } },
                    "required": ["input"]
                }}
            ]
        }),
        "tools/call" => serde_json::json!({
            "content": [{ "type": "text", "text": "tool result" }]
        }),
        "resources/list" => serde_json::json!({
            "resources": [
                { "uri": "file:///test.txt", "name": "test.txt", "description": "Test file" }
            ]
        }),
        "resources/read" => serde_json::json!({
            "contents": [{ "uri": "file:///test.txt", "text": "file content" }]
        }),
        "prompts/list" => serde_json::json!({
            "prompts": [
                { "name": "test_prompt", "description": "A test prompt" }
            ]
        }),
        "prompts/get" => serde_json::json!({
            "messages": [{ "role": "user", "content": { "type": "text", "text": "Hello" } }]
        }),
        _ => serde_json::json!({}),
    };

    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stdio_connect_send_receive() {
        let config = StdioConfig::new("echo", vec![]);
        let mut transport = StdioTransport::new(config);

        assert_eq!(transport.status(), TransportStatus::Disconnected);
        transport.connect().unwrap();
        assert_eq!(transport.status(), TransportStatus::Connected);

        let req = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
        transport.send(req).unwrap();
        assert_eq!(transport.request_count(), 1);

        let resp = transport.receive().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(parsed["jsonrpc"], "2.0");
    }

    #[test]
    fn test_stdio_disconnect() {
        let config = StdioConfig::new("echo", vec![]);
        let mut transport = StdioTransport::new(config);
        transport.connect().unwrap();
        transport.disconnect();

        assert!(transport.send("test").is_err());
    }
}
