//! WebSocket transport for MCP servers.

use serde::{Deserialize, Serialize};

use super::{TransportConfig, TransportStatus};

/// WebSocket transport configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub reconnect: bool,
    pub max_reconnect_attempts: u32,
}

impl WebSocketConfig {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.into(),
            headers: Vec::new(),
            reconnect: true,
            max_reconnect_attempts: 5,
        }
    }

    pub fn from_transport(config: &TransportConfig) -> Option<Self> {
        match config {
            TransportConfig::WebSocket { url, headers } => Some(Self {
                url: url.clone(),
                headers: headers.clone(),
                reconnect: true,
                max_reconnect_attempts: 5,
            }),
            _ => None,
        }
    }
}

/// WebSocket transport handler (simulated for unit testing)
pub struct WebSocketTransport {
    config: WebSocketConfig,
    status: TransportStatus,
    messages_sent: Vec<String>,
    messages_received: Vec<String>,
    reconnect_count: u32,
}

impl WebSocketTransport {
    pub fn new(config: WebSocketConfig) -> Self {
        Self {
            config,
            status: TransportStatus::Disconnected,
            messages_sent: Vec::new(),
            messages_received: Vec::new(),
            reconnect_count: 0,
        }
    }

    pub fn connect(&mut self) -> Result<(), String> {
        if !self.config.url.starts_with("ws://") && !self.config.url.starts_with("wss://") {
            return Err("invalid WebSocket URL: must start with ws:// or wss://".into());
        }
        self.status = TransportStatus::Connected;
        Ok(())
    }

    pub fn send(&mut self, message: &str) -> Result<(), String> {
        if self.status != TransportStatus::Connected {
            return Err("not connected".into());
        }
        self.messages_sent.push(message.into());

        // Simulate echo response
        self.messages_received.push(
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": self.messages_sent.len(),
                "result": { "status": "ok" }
            })
            .to_string(),
        );

        Ok(())
    }

    pub fn receive(&mut self) -> Result<String, String> {
        self.messages_received
            .pop()
            .ok_or_else(|| "no message available".into())
    }

    pub fn reconnect(&mut self) -> Result<(), String> {
        if self.reconnect_count >= self.config.max_reconnect_attempts {
            return Err("max reconnect attempts reached".into());
        }
        self.reconnect_count += 1;
        self.status = TransportStatus::Connected;
        Ok(())
    }

    pub fn disconnect(&mut self) {
        self.status = TransportStatus::Disconnected;
    }

    pub fn status(&self) -> TransportStatus {
        self.status
    }

    pub fn reconnect_count(&self) -> u32 {
        self.reconnect_count
    }

    pub fn messages_sent(&self) -> usize {
        self.messages_sent.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_connect() {
        let config = WebSocketConfig::new("ws://localhost:8080");
        let mut transport = WebSocketTransport::new(config);
        transport.connect().unwrap();
        assert_eq!(transport.status(), TransportStatus::Connected);
    }

    #[test]
    fn test_websocket_reconnect() {
        let config = WebSocketConfig::new("ws://localhost:8080");
        let mut transport = WebSocketTransport::new(config);
        transport.connect().unwrap();
        transport.disconnect();
        transport.reconnect().unwrap();
        assert_eq!(transport.status(), TransportStatus::Connected);
        assert_eq!(transport.reconnect_count(), 1);
    }

    #[test]
    fn test_websocket_invalid_url() {
        let config = WebSocketConfig::new("http://not-websocket");
        let mut transport = WebSocketTransport::new(config);
        assert!(transport.connect().is_err());
    }
}
