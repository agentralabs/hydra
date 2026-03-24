//! Tool definitions — schemas for all tools Hydra exposes via MCP.
//! Follows MCP Quality Standard: verb-first imperative descriptions, no trailing periods.

use crate::protocol::ToolDefinition;

/// Generate all Hydra tool definitions.
pub fn hydra_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "hydra_query".into(),
            description: "Ask Hydra a question using genome knowledge and memory".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The question to ask Hydra"
                    }
                },
                "required": ["query"]
            }),
        },
        ToolDefinition {
            name: "hydra_remember".into(),
            description: "Store a fact or observation in Hydra's memory".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "content": {
                        "type": "string",
                        "description": "The fact to remember"
                    },
                    "context": {
                        "type": "string",
                        "description": "Why this fact matters"
                    }
                },
                "required": ["content"]
            }),
        },
        ToolDefinition {
            name: "hydra_recall".into(),
            description: "Retrieve relevant memories matching a query".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "What to search for in memory"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum results to return",
                        "default": 5
                    }
                },
                "required": ["query"]
            }),
        },
        ToolDefinition {
            name: "hydra_genome".into(),
            description: "Query proven approaches from the genome knowledge base".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "situation": {
                        "type": "string",
                        "description": "The situation to find approaches for"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum approaches to return",
                        "default": 3
                    }
                },
                "required": ["situation"]
            }),
        },
        ToolDefinition {
            name: "hydra_execute".into(),
            description: "Execute an action through the Hydra executor".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "description": "Action identifier to execute"
                    },
                    "params": {
                        "type": "object",
                        "description": "Parameters for the action"
                    }
                },
                "required": ["action"]
            }),
        },
        ToolDefinition {
            name: "hydra_browse".into(),
            description: "Navigate to a URL and extract page content".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The URL to navigate to"
                    },
                    "extract": {
                        "type": "string",
                        "enum": ["text", "html", "elements", "screenshot"],
                        "description": "What to extract from the page",
                        "default": "text"
                    }
                },
                "required": ["url"]
            }),
        },
        ToolDefinition {
            name: "hydra_screenshot".into(),
            description: "Capture and analyze the current screen or browser page".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "source": {
                        "type": "string",
                        "enum": ["screen", "browser"],
                        "description": "Capture source",
                        "default": "screen"
                    },
                    "analyze": {
                        "type": "boolean",
                        "description": "Whether to analyze the screenshot with vision",
                        "default": false
                    }
                }
            }),
        },
        ToolDefinition {
            name: "hydra_status".into(),
            description: "Report Hydra system health and operational status".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
    ]
}

/// Find a tool definition by name.
pub fn find_tool(name: &str) -> Option<ToolDefinition> {
    hydra_tools().into_iter().find(|t| t.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_tools_have_descriptions() {
        for tool in hydra_tools() {
            assert!(!tool.description.is_empty(), "Tool {} has no description", tool.name);
            assert!(
                !tool.description.ends_with('.'),
                "Tool {} description ends with period (MCP standard violation)",
                tool.name
            );
        }
    }

    #[test]
    fn all_tools_have_schemas() {
        for tool in hydra_tools() {
            assert!(tool.input_schema.is_object(), "Tool {} has no schema", tool.name);
        }
    }

    #[test]
    fn tool_count() {
        assert_eq!(hydra_tools().len(), 8);
    }

    #[test]
    fn find_existing_tool() {
        assert!(find_tool("hydra_query").is_some());
        assert!(find_tool("hydra_status").is_some());
    }

    #[test]
    fn find_nonexistent_tool() {
        assert!(find_tool("nonexistent").is_none());
    }

    #[test]
    fn descriptions_are_verb_first() {
        for tool in hydra_tools() {
            let first_char = tool.description.chars().next().unwrap();
            assert!(
                first_char.is_uppercase(),
                "Tool {} description should start with capital verb: '{}'",
                tool.name, tool.description
            );
        }
    }
}
