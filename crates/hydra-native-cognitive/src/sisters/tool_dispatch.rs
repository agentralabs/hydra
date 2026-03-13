//! `<hydra-tool>` tag parsing and dispatch — routes LLM tool calls to sister MCP servers.
//!
//! The LLM is told about available MCP tools via `<hydra-tool>` tags in its prompt.
//! When it outputs `<hydra-tool name="tool_name">{"params": ...}</hydra-tool>`,
//! this module extracts the tag, looks up which sister owns the tool, and calls it.

use super::cognitive::Sisters;
use super::connection::extract_text;
use hydra_native_state::utils::safe_truncate;

/// A parsed `<hydra-tool>` invocation from LLM output.
#[derive(Debug, Clone)]
pub struct ToolInvocation {
    pub name: String,
    pub params: serde_json::Value,
}

/// Extract all `<hydra-tool name="X">JSON</hydra-tool>` tags from LLM output.
pub fn extract_hydra_tool_tags(text: &str) -> Vec<ToolInvocation> {
    let mut invocations = Vec::new();
    let mut remaining = text;
    let open_prefix = "<hydra-tool";
    let close_tag = "</hydra-tool>";

    while let Some(start) = remaining.find(open_prefix) {
        let after_open = &remaining[start..];
        // Find the closing `>` of the opening tag
        let Some(gt_pos) = after_open.find('>') else {
            remaining = &remaining[start + open_prefix.len()..];
            continue;
        };
        let tag_attrs = &after_open[open_prefix.len()..gt_pos];
        // Extract name="..." attribute
        let name = extract_attr(tag_attrs, "name");
        let Some(name) = name else {
            remaining = &remaining[start + gt_pos + 1..];
            continue;
        };
        // Content is between `>` and `</hydra-tool>`
        let content_start = &after_open[gt_pos + 1..];
        let Some(end_pos) = content_start.find(close_tag) else {
            remaining = &remaining[start + gt_pos + 1..];
            continue;
        };
        let body = content_start[..end_pos].trim();
        let params: serde_json::Value = if body.is_empty() {
            serde_json::json!({})
        } else {
            serde_json::from_str(body).unwrap_or_else(|_| {
                // If not valid JSON, wrap as a query parameter
                serde_json::json!({"query": body})
            })
        };
        invocations.push(ToolInvocation { name, params });
        remaining = &content_start[end_pos + close_tag.len()..];
    }
    invocations
}

/// Strip `<hydra-tool ...>...</hydra-tool>` tags from text for clean display.
pub fn strip_hydra_tool_tags(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut remaining = text;
    let open_prefix = "<hydra-tool";
    let close_tag = "</hydra-tool>";

    while let Some(start) = remaining.find(open_prefix) {
        result.push_str(&remaining[..start]);
        let after = &remaining[start..];
        if let Some(end) = after.find(close_tag) {
            remaining = &after[end + close_tag.len()..];
        } else {
            remaining = &remaining[start + open_prefix.len()..];
        }
    }
    result.push_str(remaining);
    result.trim().to_string()
}

/// Extract an attribute value from a tag fragment like ` name="value"`.
fn extract_attr(attrs: &str, key: &str) -> Option<String> {
    let pattern = format!("{}=\"", key);
    let pos = attrs.find(&pattern)?;
    let after = &attrs[pos + pattern.len()..];
    let end = after.find('"')?;
    Some(after[..end].to_string())
}

impl Sisters {
    /// Dispatch a tool call to the sister that owns the tool.
    /// Looks up the tool name in each sister's registered tool list.
    pub async fn dispatch_tool(&self, name: &str, params: serde_json::Value) -> Option<String> {
        let conn = self.find_tool_owner(name)?;
        match conn.call_tool(name, params).await {
            Ok(result) => {
                let text = extract_text(&result);
                if text.is_empty() { None } else { Some(text) }
            }
            Err(e) => {
                eprintln!("[hydra:tool-dispatch] {} failed: {}", name, safe_truncate(&e, 100));
                None
            }
        }
    }

    /// Find which sister connection owns a tool by name.
    fn find_tool_owner(&self, tool_name: &str) -> Option<&super::connection::SisterConnection> {
        let sisters: [(&Option<super::connection::SisterConnection>, &str); 14] = [
            (&self.memory, "memory"),
            (&self.identity, "identity"),
            (&self.codebase, "codebase"),
            (&self.vision, "vision"),
            (&self.comm, "comm"),
            (&self.contract, "contract"),
            (&self.time, "time"),
            (&self.planning, "planning"),
            (&self.cognition, "cognition"),
            (&self.reality, "reality"),
            (&self.forge, "forge"),
            (&self.aegis, "aegis"),
            (&self.veritas, "veritas"),
            (&self.evolve, "evolve"),
        ];
        for (opt, _name) in &sisters {
            if let Some(conn) = opt.as_ref() {
                if conn.tools.iter().any(|t| t == tool_name) {
                    return Some(conn);
                }
            }
        }
        None
    }

    /// Execute all `<hydra-tool>` invocations from LLM output.
    /// Returns (tool_name, result_text) pairs.
    pub async fn execute_tool_tags(&self, text: &str) -> Vec<(String, String)> {
        let invocations = extract_hydra_tool_tags(text);
        let mut results = Vec::with_capacity(invocations.len());
        for inv in invocations {
            eprintln!("[hydra:tool] Dispatching: {}", inv.name);
            match self.dispatch_tool(&inv.name, inv.params).await {
                Some(output) => {
                    eprintln!("[hydra:tool] {} → {} chars", inv.name, output.len());
                    results.push((inv.name, output));
                }
                None => {
                    eprintln!("[hydra:tool] {} → no result", inv.name);
                    results.push((inv.name, "Tool returned no result".to_string()));
                }
            }
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_hydra_tool_tags_basic() {
        let text = r#"Here is my analysis. <hydra-tool name="memory_query">{"query": "test"}</hydra-tool> Done."#;
        let tags = extract_hydra_tool_tags(text);
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "memory_query");
        assert_eq!(tags[0].params["query"], "test");
    }

    #[test]
    fn test_extract_multiple_tags() {
        let text = r#"<hydra-tool name="memory_query">{"query": "a"}</hydra-tool> text <hydra-tool name="codebase_search">{"pattern": "b"}</hydra-tool>"#;
        let tags = extract_hydra_tool_tags(text);
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].name, "memory_query");
        assert_eq!(tags[1].name, "codebase_search");
    }

    #[test]
    fn test_extract_empty_body() {
        let text = r#"<hydra-tool name="identity_show"></hydra-tool>"#;
        let tags = extract_hydra_tool_tags(text);
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "identity_show");
        assert!(tags[0].params.is_object());
    }

    #[test]
    fn test_extract_no_tags() {
        let tags = extract_hydra_tool_tags("no tags here");
        assert!(tags.is_empty());
    }

    #[test]
    fn test_strip_hydra_tool_tags() {
        let text = r#"Before <hydra-tool name="x">{"a":1}</hydra-tool> after"#;
        let stripped = strip_hydra_tool_tags(text);
        assert_eq!(stripped, "Before  after");
    }

    #[test]
    fn test_extract_attr() {
        assert_eq!(extract_attr(r#" name="hello" "#, "name"), Some("hello".into()));
        assert_eq!(extract_attr(r#" id="42" name="test""#, "name"), Some("test".into()));
        assert_eq!(extract_attr(r#" foo="bar""#, "name"), None);
    }

    #[test]
    fn test_non_json_body_wrapped_as_query() {
        let text = r#"<hydra-tool name="memory_query">just plain text</hydra-tool>"#;
        let tags = extract_hydra_tool_tags(text);
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].params["query"], "just plain text");
    }
}
