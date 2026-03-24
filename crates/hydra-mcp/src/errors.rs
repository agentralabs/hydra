//! MCP error types.

use thiserror::Error;

/// All MCP protocol errors.
#[derive(Debug, Error, Clone)]
pub enum McpError {
    /// Tool not found.
    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    /// Tool execution failed.
    #[error("Tool execution failed: {tool} — {reason}")]
    ToolFailed { tool: String, reason: String },

    /// Transport error (connection, I/O).
    #[error("Transport error: {0}")]
    TransportError(String),

    /// Protocol error (invalid JSON-RPC).
    #[error("Protocol error: {0}")]
    ProtocolError(String),

    /// Serialization error.
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Server not started.
    #[error("Server not started")]
    ServerNotStarted,

    /// Client not connected.
    #[error("Client not connected to {0}")]
    NotConnected(String),

    /// Timeout waiting for response.
    #[error("Timeout after {ms}ms")]
    Timeout { ms: u64 },
}

/// MCP error codes per the MCP Quality Standard.
pub const TOOL_NOT_FOUND: i64 = -32803;
pub const INVALID_PARAMS: i64 = -32602;
pub const METHOD_NOT_FOUND: i64 = -32601;
pub const PARSE_ERROR: i64 = -32700;
pub const INTERNAL_ERROR: i64 = -32603;
