//! MCP protocol constants.

/// MCP protocol version.
pub const PROTOCOL_VERSION: &str = "2024-11-05";

/// Server name.
pub const SERVER_NAME: &str = "hydra";

/// Server version.
pub const SERVER_VERSION: &str = "0.1.0";

/// Maximum tools a client can discover.
pub const MAX_DISCOVERED_TOOLS: usize = 500;

/// Request timeout (ms).
pub const REQUEST_TIMEOUT_MS: u64 = 30_000;

/// Maximum concurrent tool executions.
pub const MAX_CONCURRENT_TOOLS: usize = 10;
