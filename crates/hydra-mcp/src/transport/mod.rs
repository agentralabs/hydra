//! Transport layer for MCP connections.

pub mod http;
pub mod stdio;
pub mod websocket;

use serde::{Deserialize, Serialize};

/// Transport type for connecting to an MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransportConfig {
    /// Stdio transport — spawn a child process
    Stdio {
        command: String,
        args: Vec<String>,
        #[serde(default)]
        env: Vec<(String, String)>,
    },
    /// HTTP+SSE transport — connect to an HTTP endpoint
    Http {
        url: String,
        #[serde(default)]
        headers: Vec<(String, String)>,
    },
    /// WebSocket transport
    WebSocket {
        url: String,
        #[serde(default)]
        headers: Vec<(String, String)>,
    },
}

impl TransportConfig {
    pub fn stdio(command: &str, args: Vec<String>) -> Self {
        Self::Stdio {
            command: command.into(),
            args,
            env: Vec::new(),
        }
    }

    pub fn http(url: &str) -> Self {
        Self::Http {
            url: url.into(),
            headers: Vec::new(),
        }
    }

    pub fn websocket(url: &str) -> Self {
        Self::WebSocket {
            url: url.into(),
            headers: Vec::new(),
        }
    }

    pub fn transport_type(&self) -> &'static str {
        match self {
            Self::Stdio { .. } => "stdio",
            Self::Http { .. } => "http",
            Self::WebSocket { .. } => "websocket",
        }
    }
}

/// Transport status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportStatus {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

/// Transport message (request or response line)
#[derive(Debug, Clone)]
pub struct TransportMessage {
    pub data: String,
}

impl TransportMessage {
    pub fn new(data: String) -> Self {
        Self { data }
    }
}
