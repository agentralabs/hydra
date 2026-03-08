//! ToolIntrospector — discover tools, resources, and prompts from MCP servers.

use serde::{Deserialize, Serialize};

use crate::connector::ServerConnector;
use crate::protocol::{JsonRpcRequest, McpPrompt, McpResource, McpTool};
use crate::registry::ServerRegistry;

/// Discovered capabilities of a server
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiscoveredCapabilities {
    pub tools: Vec<McpTool>,
    pub resources: Vec<McpResource>,
    pub prompts: Vec<McpPrompt>,
    pub tool_count: usize,
    pub resource_count: usize,
    pub prompt_count: usize,
}

/// Introspects MCP servers to discover their capabilities
pub struct ToolIntrospector;

impl ToolIntrospector {
    /// Discover all capabilities from a connected server
    pub fn discover(
        server_id: &str,
        connector: &ServerConnector,
        registry: &ServerRegistry,
    ) -> Result<DiscoveredCapabilities, String> {
        let tools = Self::discover_tools(server_id, connector)?;
        let resources = Self::discover_resources(server_id, connector)?;
        let prompts = Self::discover_prompts(server_id, connector)?;

        let caps = DiscoveredCapabilities {
            tool_count: tools.len(),
            resource_count: resources.len(),
            prompt_count: prompts.len(),
            tools: tools.clone(),
            resources: resources.clone(),
            prompts: prompts.clone(),
        };

        // Update registry with discovered capabilities
        registry.set_tools(server_id, tools);
        registry.set_resources(server_id, resources);
        registry.set_prompts(server_id, prompts);

        Ok(caps)
    }

    /// Discover tools from a server
    pub fn discover_tools(
        server_id: &str,
        connector: &ServerConnector,
    ) -> Result<Vec<McpTool>, String> {
        let req = JsonRpcRequest::tools_list(2);
        let resp = connector.send_request(server_id, &req)?;
        let result = resp
            .into_result()
            .map_err(|e| format!("tools/list error: {}", e.message))?;

        let tools: Vec<McpTool> = result
            .get("tools")
            .and_then(|t| serde_json::from_value(t.clone()).ok())
            .unwrap_or_default();

        Ok(tools)
    }

    /// Discover resources from a server
    pub fn discover_resources(
        server_id: &str,
        connector: &ServerConnector,
    ) -> Result<Vec<McpResource>, String> {
        let req = JsonRpcRequest::resources_list(3);
        let resp = connector.send_request(server_id, &req)?;
        let result = resp
            .into_result()
            .map_err(|e| format!("resources/list error: {}", e.message))?;

        let resources: Vec<McpResource> = result
            .get("resources")
            .and_then(|r| serde_json::from_value(r.clone()).ok())
            .unwrap_or_default();

        Ok(resources)
    }

    /// Discover prompts from a server
    pub fn discover_prompts(
        server_id: &str,
        connector: &ServerConnector,
    ) -> Result<Vec<McpPrompt>, String> {
        let req = JsonRpcRequest::prompts_list(4);
        let resp = connector.send_request(server_id, &req)?;
        let result = resp
            .into_result()
            .map_err(|e| format!("prompts/list error: {}", e.message))?;

        let prompts: Vec<McpPrompt> = result
            .get("prompts")
            .and_then(|p| serde_json::from_value(p.clone()).ok())
            .unwrap_or_default();

        Ok(prompts)
    }

    /// Get schema for a specific tool
    pub fn tool_schema(
        server_id: &str,
        tool_name: &str,
        registry: &ServerRegistry,
    ) -> Option<serde_json::Value> {
        let entry = registry.get(server_id)?;
        entry
            .tools
            .iter()
            .find(|t| t.name == tool_name)
            .and_then(|t| t.input_schema.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::TransportConfig;

    fn setup() -> (ServerConnector, ServerRegistry, String) {
        let connector = ServerConnector::new();
        let registry = ServerRegistry::new();
        let config = TransportConfig::stdio("test-server", vec![]);
        let server_id = registry.add("test-server", config.clone());
        connector.connect(&server_id, &config).unwrap();
        (connector, registry, server_id)
    }

    #[test]
    fn test_discover_tools() {
        let (connector, _registry, server_id) = setup();
        let tools = ToolIntrospector::discover_tools(&server_id, &connector).unwrap();
        assert!(!tools.is_empty());
        assert_eq!(tools[0].name, "test_tool");
    }

    #[test]
    fn test_discover_resources() {
        let (connector, _registry, server_id) = setup();
        let resources = ToolIntrospector::discover_resources(&server_id, &connector).unwrap();
        assert!(!resources.is_empty());
        assert_eq!(resources[0].name, "test.txt");
    }

    #[test]
    fn test_discover_prompts() {
        let (connector, _registry, server_id) = setup();
        let prompts = ToolIntrospector::discover_prompts(&server_id, &connector).unwrap();
        assert!(!prompts.is_empty());
        assert_eq!(prompts[0].name, "test_prompt");
    }

    #[test]
    fn test_full_discovery() {
        let (connector, registry, server_id) = setup();
        let caps = ToolIntrospector::discover(&server_id, &connector, &registry).unwrap();
        assert!(caps.tool_count > 0);
        assert!(caps.resource_count > 0);
        assert!(caps.prompt_count > 0);

        // Registry should be updated
        let entry = registry.get(&server_id).unwrap();
        assert!(!entry.tools.is_empty());
        assert!(!entry.resources.is_empty());
        assert!(!entry.prompts.is_empty());
    }
}
