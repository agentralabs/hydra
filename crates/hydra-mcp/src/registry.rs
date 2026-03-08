//! ServerRegistry — track and manage known MCP servers.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::protocol::{McpCapabilities, McpPrompt, McpResource, McpServerInfo, McpTool};
use crate::transport::TransportConfig;

/// Unique server identifier
pub type ServerId = String;

/// A registered MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerEntry {
    pub id: ServerId,
    pub name: String,
    pub transport: TransportConfig,
    pub status: ServerStatus,
    #[serde(default)]
    pub server_info: Option<McpServerInfo>,
    #[serde(default)]
    pub capabilities: McpCapabilities,
    #[serde(default)]
    pub tools: Vec<McpTool>,
    #[serde(default)]
    pub resources: Vec<McpResource>,
    #[serde(default)]
    pub prompts: Vec<McpPrompt>,
    pub registered_at: String,
    #[serde(default)]
    pub last_connected: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Server connection status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerStatus {
    Registered,
    Connecting,
    Connected,
    Disconnected,
    Error,
}

/// Registry for tracking MCP servers
pub struct ServerRegistry {
    servers: parking_lot::RwLock<HashMap<ServerId, ServerEntry>>,
}

impl ServerRegistry {
    pub fn new() -> Self {
        Self {
            servers: parking_lot::RwLock::new(HashMap::new()),
        }
    }

    /// Register a new server
    pub fn add(&self, name: &str, transport: TransportConfig) -> ServerId {
        let id = format!("mcp-{}", uuid::Uuid::new_v4());
        let entry = ServerEntry {
            id: id.clone(),
            name: name.into(),
            transport,
            status: ServerStatus::Registered,
            server_info: None,
            capabilities: McpCapabilities::default(),
            tools: Vec::new(),
            resources: Vec::new(),
            prompts: Vec::new(),
            registered_at: chrono::Utc::now().to_rfc3339(),
            last_connected: None,
            tags: Vec::new(),
        };
        self.servers.write().insert(id.clone(), entry);
        id
    }

    /// Register a server with a specific ID
    pub fn add_with_id(&self, id: &str, name: &str, transport: TransportConfig) -> ServerId {
        let entry = ServerEntry {
            id: id.into(),
            name: name.into(),
            transport,
            status: ServerStatus::Registered,
            server_info: None,
            capabilities: McpCapabilities::default(),
            tools: Vec::new(),
            resources: Vec::new(),
            prompts: Vec::new(),
            registered_at: chrono::Utc::now().to_rfc3339(),
            last_connected: None,
            tags: Vec::new(),
        };
        self.servers.write().insert(id.into(), entry);
        id.into()
    }

    /// Remove a server
    pub fn remove(&self, id: &str) -> bool {
        self.servers.write().remove(id).is_some()
    }

    /// Get a server by ID
    pub fn get(&self, id: &str) -> Option<ServerEntry> {
        self.servers.read().get(id).cloned()
    }

    /// List all servers
    pub fn list(&self) -> Vec<ServerEntry> {
        self.servers.read().values().cloned().collect()
    }

    /// List servers by status
    pub fn list_by_status(&self, status: ServerStatus) -> Vec<ServerEntry> {
        self.servers
            .read()
            .values()
            .filter(|s| s.status == status)
            .cloned()
            .collect()
    }

    /// Find servers by tag
    pub fn find_by_tag(&self, tag: &str) -> Vec<ServerEntry> {
        self.servers
            .read()
            .values()
            .filter(|s| s.tags.contains(&tag.to_string()))
            .cloned()
            .collect()
    }

    /// Find servers that have a specific tool
    pub fn find_by_tool(&self, tool_name: &str) -> Vec<ServerEntry> {
        self.servers
            .read()
            .values()
            .filter(|s| s.tools.iter().any(|t| t.name == tool_name))
            .cloned()
            .collect()
    }

    /// Update server status
    pub fn set_status(&self, id: &str, status: ServerStatus) -> bool {
        if let Some(entry) = self.servers.write().get_mut(id) {
            entry.status = status;
            if status == ServerStatus::Connected {
                entry.last_connected = Some(chrono::Utc::now().to_rfc3339());
            }
            true
        } else {
            false
        }
    }

    /// Update discovered tools for a server
    pub fn set_tools(&self, id: &str, tools: Vec<McpTool>) {
        if let Some(entry) = self.servers.write().get_mut(id) {
            entry.tools = tools;
        }
    }

    /// Update discovered resources for a server
    pub fn set_resources(&self, id: &str, resources: Vec<McpResource>) {
        if let Some(entry) = self.servers.write().get_mut(id) {
            entry.resources = resources;
        }
    }

    /// Update discovered prompts for a server
    pub fn set_prompts(&self, id: &str, prompts: Vec<McpPrompt>) {
        if let Some(entry) = self.servers.write().get_mut(id) {
            entry.prompts = prompts;
        }
    }

    /// Add tags to a server
    pub fn add_tags(&self, id: &str, tags: Vec<String>) {
        if let Some(entry) = self.servers.write().get_mut(id) {
            for tag in tags {
                if !entry.tags.contains(&tag) {
                    entry.tags.push(tag);
                }
            }
        }
    }

    /// Count registered servers
    pub fn count(&self) -> usize {
        self.servers.read().len()
    }

    /// Count connected servers
    pub fn connected_count(&self) -> usize {
        self.servers
            .read()
            .values()
            .filter(|s| s.status == ServerStatus::Connected)
            .count()
    }
}

impl Default for ServerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_get() {
        let registry = ServerRegistry::new();
        let id = registry.add("test-server", TransportConfig::stdio("echo", vec![]));
        let entry = registry.get(&id).unwrap();
        assert_eq!(entry.name, "test-server");
        assert_eq!(entry.status, ServerStatus::Registered);
    }

    #[test]
    fn test_remove() {
        let registry = ServerRegistry::new();
        let id = registry.add("server", TransportConfig::http("http://localhost:3000"));
        assert_eq!(registry.count(), 1);
        assert!(registry.remove(&id));
        assert_eq!(registry.count(), 0);
        assert!(!registry.remove("nonexistent"));
    }

    #[test]
    fn test_status_update() {
        let registry = ServerRegistry::new();
        let id = registry.add("server", TransportConfig::stdio("cmd", vec![]));
        registry.set_status(&id, ServerStatus::Connected);
        let entry = registry.get(&id).unwrap();
        assert_eq!(entry.status, ServerStatus::Connected);
        assert!(entry.last_connected.is_some());
    }

    #[test]
    fn test_find_by_tool() {
        let registry = ServerRegistry::new();
        let id = registry.add("memory", TransportConfig::stdio("memory-mcp", vec![]));
        registry.set_tools(
            &id,
            vec![
                McpTool {
                    name: "memory_add".into(),
                    description: "Add memory".into(),
                    input_schema: None,
                },
                McpTool {
                    name: "memory_query".into(),
                    description: "Query memory".into(),
                    input_schema: None,
                },
            ],
        );

        let found = registry.find_by_tool("memory_add");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "memory");

        let not_found = registry.find_by_tool("nonexistent_tool");
        assert!(not_found.is_empty());
    }

    #[test]
    fn test_tags() {
        let registry = ServerRegistry::new();
        let id = registry.add("server", TransportConfig::stdio("cmd", vec![]));
        registry.add_tags(&id, vec!["sister".into(), "memory".into()]);

        let found = registry.find_by_tag("sister");
        assert_eq!(found.len(), 1);

        let not_found = registry.find_by_tag("unknown");
        assert!(not_found.is_empty());
    }
}
