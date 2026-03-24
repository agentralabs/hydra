//! hydra-mcp — Model Context Protocol server and client for Hydra.
//!
//! Server: Exposes 8 Hydra tools (query, remember, recall, genome, execute,
//! browse, screenshot, status) to any MCP-compatible client.
//! Client: Connects to external MCP servers and discovers tools dynamically.
//! Transport: stdio (production) and in-memory (testing).

pub mod client;
pub mod constants;
pub mod errors;
pub mod protocol;
pub mod server;
pub mod tools;
pub mod transport;

// ── Re-exports ──

pub use client::McpClient;
pub use errors::McpError;
pub use protocol::{
    JsonRpcError, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, ToolContent,
    ToolDefinition, ToolResult,
};
pub use server::{McpServer, ToolHandler};
pub use transport::{MemoryTransport, StdioTransport, Transport};
