//! MCP tool adapter — convert MCP tool definitions to skills.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::definition::*;

/// MCP tool definition (from tools/list response)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDefinition {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub input_schema: Option<McpSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSchema {
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(default)]
    pub properties: HashMap<String, McpProperty>,
    #[serde(default)]
    pub required: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpProperty {
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(default)]
    pub description: String,
}

/// Adapter for importing MCP tools as skills
pub struct McpAdapter;

impl McpAdapter {
    /// Convert an MCP tool definition to a skill
    pub fn from_tool(server: &str, tool: McpToolDefinition) -> SkillDefinition {
        let parameters = tool
            .input_schema
            .as_ref()
            .map(|schema| {
                let required = &schema.required;
                schema
                    .properties
                    .iter()
                    .map(|(name, prop)| SkillParam {
                        name: name.clone(),
                        param_type: map_json_type(&prop.type_),
                        required: required.contains(name),
                        description: prop.description.clone(),
                        default: None,
                        constraints: vec![],
                    })
                    .collect()
            })
            .unwrap_or_default();

        let id = format!("mcp-{}-{}", server, tool.name);

        SkillDefinition {
            id,
            name: tool.name.clone(),
            version: "1.0.0".into(),
            description: tool.description,
            triggers: vec![
                SkillTrigger::Tool(format!("{}.{}", server, tool.name)),
                SkillTrigger::Intent(tool.name.clone()),
            ],
            parameters,
            outputs: vec![],
            requirements: vec![Requirement::Sister(server.into())],
            source: SkillSource::Mcp {
                server: server.into(),
            },
            sandbox_level: SandboxLevel::Basic,
            risk_level: RiskLevel::Low,
            metadata: SkillMetadata::default(),
        }
    }

    /// Parse MCP tool definition from JSON
    pub fn parse(server: &str, json: &str) -> Result<SkillDefinition, String> {
        let tool: McpToolDefinition =
            serde_json::from_str(json).map_err(|e| format!("invalid MCP tool JSON: {}", e))?;
        Ok(Self::from_tool(server, tool))
    }
}

fn map_json_type(type_str: &str) -> ParamType {
    match type_str {
        "string" => ParamType::String,
        "number" | "integer" => ParamType::Number,
        "boolean" => ParamType::Boolean,
        "array" => ParamType::Array,
        "object" => ParamType::Object,
        _ => ParamType::String,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_tool_as_skill() {
        let json = r#"{
            "name": "memory_add",
            "description": "Add a memory entry",
            "input_schema": {
                "type": "object",
                "properties": {
                    "content": { "type": "string", "description": "Memory content" },
                    "context": { "type": "string", "description": "Context tag" }
                },
                "required": ["content"]
            }
        }"#;

        let skill = McpAdapter::parse("agentic-memory", json).unwrap();
        assert_eq!(skill.name, "memory_add");
        assert_eq!(
            skill.source,
            SkillSource::Mcp {
                server: "agentic-memory".into()
            }
        );
        assert_eq!(skill.parameters.len(), 2);
        assert!(
            skill
                .parameters
                .iter()
                .find(|p| p.name == "content")
                .unwrap()
                .required
        );
        assert!(
            !skill
                .parameters
                .iter()
                .find(|p| p.name == "context")
                .unwrap()
                .required
        );
        assert_eq!(
            skill.requirements,
            vec![Requirement::Sister("agentic-memory".into())]
        );
    }

    #[test]
    fn test_mcp_tool_no_schema() {
        let tool = McpToolDefinition {
            name: "simple_tool".into(),
            description: "A simple tool".into(),
            input_schema: None,
        };
        let skill = McpAdapter::from_tool("server", tool);
        assert!(skill.parameters.is_empty());
        assert_eq!(skill.name, "simple_tool");
    }

    #[test]
    fn test_mcp_tool_triggers() {
        let tool = McpToolDefinition {
            name: "test_tool".into(),
            description: "test".into(),
            input_schema: None,
        };
        let skill = McpAdapter::from_tool("myserver", tool);
        assert!(skill.triggers.contains(&SkillTrigger::Tool("myserver.test_tool".into())));
        assert!(skill.triggers.contains(&SkillTrigger::Intent("test_tool".into())));
    }

    #[test]
    fn test_mcp_tool_id_format() {
        let tool = McpToolDefinition {
            name: "my_tool".into(),
            description: "test".into(),
            input_schema: None,
        };
        let skill = McpAdapter::from_tool("server", tool);
        assert_eq!(skill.id, "mcp-server-my_tool");
    }

    #[test]
    fn test_mcp_tool_sandbox_level() {
        let tool = McpToolDefinition {
            name: "test".into(),
            description: "test".into(),
            input_schema: None,
        };
        let skill = McpAdapter::from_tool("s", tool);
        assert_eq!(skill.sandbox_level, SandboxLevel::Basic);
    }

    #[test]
    fn test_mcp_type_mapping() {
        assert_eq!(map_json_type("string"), ParamType::String);
        assert_eq!(map_json_type("number"), ParamType::Number);
        assert_eq!(map_json_type("integer"), ParamType::Number);
        assert_eq!(map_json_type("boolean"), ParamType::Boolean);
        assert_eq!(map_json_type("array"), ParamType::Array);
        assert_eq!(map_json_type("object"), ParamType::Object);
        assert_eq!(map_json_type("unknown"), ParamType::String);
    }

    #[test]
    fn test_parse_invalid_json() {
        let result = McpAdapter::parse("server", "not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_mcp_tool_serde() {
        let tool = McpToolDefinition {
            name: "test".into(),
            description: "desc".into(),
            input_schema: Some(McpSchema {
                type_: "object".into(),
                properties: HashMap::from([("key".into(), McpProperty { type_: "string".into(), description: "val".into() })]),
                required: vec!["key".into()],
            }),
        };
        let json = serde_json::to_string(&tool).unwrap();
        let restored: McpToolDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.name, "test");
    }
}
