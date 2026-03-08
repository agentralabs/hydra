//! ServerConnector — connect to MCP servers via any transport.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::protocol::{JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};
use crate::transport::http::{HttpConfig, HttpTransport};
use crate::transport::stdio::{StdioConfig, StdioTransport};
use crate::transport::websocket::{WebSocketConfig, WebSocketTransport};
use crate::transport::TransportConfig;

/// Connection state for a server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub server_id: String,
    pub transport_type: String,
    pub connected: bool,
    pub initialized: bool,
    pub request_count: u64,
}

/// Manages connections to MCP servers
pub struct ServerConnector {
    connections: parking_lot::RwLock<HashMap<String, Connection>>,
}

/// Internal connection state
struct Connection {
    transport_type: String,
    stdio: Option<StdioTransport>,
    http: Option<HttpTransport>,
    websocket: Option<WebSocketTransport>,
    initialized: bool,
    request_counter: u64,
}

impl ServerConnector {
    pub fn new() -> Self {
        Self {
            connections: parking_lot::RwLock::new(HashMap::new()),
        }
    }

    /// Connect to a server using the provided transport config
    pub fn connect(
        &self,
        server_id: &str,
        config: &TransportConfig,
    ) -> Result<ConnectionInfo, String> {
        let transport_type = config.transport_type().to_string();

        let connection = match config {
            TransportConfig::Stdio { command, args, env } => {
                let stdio_config = StdioConfig {
                    command: command.clone(),
                    args: args.clone(),
                    env: env.clone(),
                    timeout_ms: 5000,
                };
                let mut transport = StdioTransport::new(stdio_config);
                transport
                    .connect()
                    .map_err(|e| format!("stdio connect failed: {}", e))?;
                Connection {
                    transport_type: "stdio".into(),
                    stdio: Some(transport),
                    http: None,
                    websocket: None,
                    initialized: false,
                    request_counter: 0,
                }
            }
            TransportConfig::Http { url, headers } => {
                let http_config = HttpConfig {
                    url: url.clone(),
                    headers: headers.clone(),
                    timeout_ms: 30000,
                };
                let mut transport = HttpTransport::new(http_config);
                transport
                    .connect()
                    .map_err(|e| format!("http connect failed: {}", e))?;
                Connection {
                    transport_type: "http".into(),
                    stdio: None,
                    http: Some(transport),
                    websocket: None,
                    initialized: false,
                    request_counter: 0,
                }
            }
            TransportConfig::WebSocket { url, headers } => {
                let ws_config = WebSocketConfig {
                    url: url.clone(),
                    headers: headers.clone(),
                    reconnect: true,
                    max_reconnect_attempts: 5,
                };
                let mut transport = WebSocketTransport::new(ws_config);
                transport
                    .connect()
                    .map_err(|e| format!("websocket connect failed: {}", e))?;
                Connection {
                    transport_type: "websocket".into(),
                    stdio: None,
                    http: None,
                    websocket: Some(transport),
                    initialized: false,
                    request_counter: 0,
                }
            }
        };

        self.connections
            .write()
            .insert(server_id.into(), connection);

        Ok(ConnectionInfo {
            server_id: server_id.into(),
            transport_type,
            connected: true,
            initialized: false,
            request_count: 0,
        })
    }

    /// Initialize the MCP connection (handshake)
    pub fn initialize(&self, server_id: &str) -> Result<JsonRpcResponse, String> {
        let req = JsonRpcRequest::initialize(1, "hydra", "0.1.0");
        let response = self.send_request(server_id, &req)?;

        // Send initialized notification
        let _notif = JsonRpcNotification::initialized();
        // In production: send notification over transport

        let mut connections = self.connections.write();
        if let Some(conn) = connections.get_mut(server_id) {
            conn.initialized = true;
        }

        Ok(response)
    }

    /// Send a JSON-RPC request and get response
    pub fn send_request(
        &self,
        server_id: &str,
        request: &JsonRpcRequest,
    ) -> Result<JsonRpcResponse, String> {
        let serialized =
            serde_json::to_string(request).map_err(|e| format!("serialization error: {}", e))?;

        let mut connections = self.connections.write();
        let conn = connections
            .get_mut(server_id)
            .ok_or_else(|| format!("server not connected: {}", server_id))?;

        conn.request_counter += 1;

        let response_str = if let Some(ref mut stdio) = conn.stdio {
            stdio.send(&serialized)?;
            stdio.receive()?
        } else if let Some(ref mut http) = conn.http {
            http.send(&request.method, &serialized)?
        } else if let Some(ref mut ws) = conn.websocket {
            ws.send(&serialized)?;
            ws.receive()?
        } else {
            return Err("no transport available".into());
        };

        serde_json::from_str(&response_str).map_err(|e| format!("response parse error: {}", e))
    }

    /// Disconnect from a server
    pub fn disconnect(&self, server_id: &str) -> bool {
        let mut connections = self.connections.write();
        if let Some(mut conn) = connections.remove(server_id) {
            if let Some(ref mut stdio) = conn.stdio {
                stdio.disconnect();
            }
            if let Some(ref mut http) = conn.http {
                http.disconnect();
            }
            if let Some(ref mut ws) = conn.websocket {
                ws.disconnect();
            }
            true
        } else {
            false
        }
    }

    /// Check if a server is connected
    pub fn is_connected(&self, server_id: &str) -> bool {
        self.connections.read().contains_key(server_id)
    }

    /// Get connection info
    pub fn info(&self, server_id: &str) -> Option<ConnectionInfo> {
        self.connections
            .read()
            .get(server_id)
            .map(|conn| ConnectionInfo {
                server_id: server_id.into(),
                transport_type: conn.transport_type.clone(),
                connected: true,
                initialized: conn.initialized,
                request_count: conn.request_counter,
            })
    }

    /// Count active connections
    pub fn connection_count(&self) -> usize {
        self.connections.read().len()
    }
}

impl Default for ServerConnector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stdio_connection() {
        let connector = ServerConnector::new();
        let config = TransportConfig::stdio("echo", vec![]);
        let info = connector.connect("test", &config).unwrap();
        assert!(info.connected);
        assert_eq!(info.transport_type, "stdio");
        assert!(connector.is_connected("test"));
    }

    #[test]
    fn test_http_connection() {
        let connector = ServerConnector::new();
        let config = TransportConfig::http("http://localhost:3000");
        let info = connector.connect("test-http", &config).unwrap();
        assert!(info.connected);
        assert_eq!(info.transport_type, "http");
    }

    #[test]
    fn test_websocket_connection() {
        let connector = ServerConnector::new();
        let config = TransportConfig::websocket("ws://localhost:8080");
        let info = connector.connect("test-ws", &config).unwrap();
        assert!(info.connected);
        assert_eq!(info.transport_type, "websocket");
    }

    #[test]
    fn test_disconnect() {
        let connector = ServerConnector::new();
        let config = TransportConfig::stdio("echo", vec![]);
        connector.connect("test", &config).unwrap();
        assert!(connector.disconnect("test"));
        assert!(!connector.is_connected("test"));
    }

    #[test]
    fn test_initialize() {
        let connector = ServerConnector::new();
        let config = TransportConfig::stdio("echo", vec![]);
        connector.connect("test", &config).unwrap();
        let resp = connector.initialize("test").unwrap();
        assert!(!resp.is_error());
        let info = connector.info("test").unwrap();
        assert!(info.initialized);
    }
}
