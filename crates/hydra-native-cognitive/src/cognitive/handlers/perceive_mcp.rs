//! MCP skill discovery helper — extracted from phase_perceive.rs for file size.

use std::sync::Arc;
use tokio::sync::mpsc;

use hydra_db::{HydraDb, McpDiscoveredSkillRow};

use crate::sisters::SistersHandle;
use super::super::loop_runner::CognitiveUpdate;

/// Discover MCP tools from sisters, persist to DB, and return a summary string.
pub(crate) fn discover_mcp_skills(
    sisters_handle: &SistersHandle,
    db: &Option<Arc<HydraDb>>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> Option<String> {
    let tools = sisters_handle.discover_mcp_tools();
    if tools.is_empty() {
        return None;
    }

    if let Some(ref db) = db {
        let now = chrono::Utc::now().to_rfc3339();
        let mut servers_seen: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();

        for (server, tool_name) in &tools {
            servers_seen
                .entry(server.clone())
                .or_default()
                .push(tool_name.clone());
            let skill_id = format!(
                "mcp-{}-{}",
                server.to_lowercase(),
                tool_name
            );
            let _ = db.upsert_mcp_skill(&McpDiscoveredSkillRow {
                id: skill_id,
                server_name: server.clone(),
                tool_name: tool_name.clone(),
                description: None,
                input_schema: None,
                discovered_at: now.clone(),
                last_used_at: None,
                use_count: 0,
                active: true,
            });
        }

        for (server, tool_names) in &servers_seen {
            let _ = tx.send(CognitiveUpdate::McpSkillsDiscovered {
                server: server.clone(),
                tools: tool_names.clone(),
                count: tool_names.len(),
            });
        }
    }

    let mut by_server: std::collections::HashMap<&str, Vec<&str>> =
        std::collections::HashMap::new();
    for (server, tool_name) in &tools {
        by_server.entry(server).or_default().push(tool_name);
    }
    let summary: String = by_server
        .iter()
        .map(|(server, tls)| {
            format!("- {} ({} tools): {}", server, tls.len(), tls.join(", "))
        })
        .collect::<Vec<_>>()
        .join("\n");
    Some(summary)
}
