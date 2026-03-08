//! hydra-mcp — MCP Universal Adapter for Hydra.
//!
//! Connect to any MCP server dynamically:
//! - **Registry**: Track and manage MCP server connections
//! - **Connector**: Connect via stdio, HTTP, or WebSocket
//! - **Introspector**: Discover tools, resources, and prompts
//! - **Invoker**: Call any tool without compile-time types
//! - **Schema**: Validate inputs/outputs against JSON Schema

pub mod connector;
pub mod introspect;
pub mod invoker;
pub mod protocol;
pub mod registry;
pub mod schema;
pub mod transport;
