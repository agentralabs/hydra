//! Memory Facades & Longevity — compact facade routers, longevity engine, core gaps.

use super::connection::extract_text;
use super::cognitive::Sisters;
use hydra_native_state::utils::safe_truncate;

/// Helper: call a facade tool with operation + params on memory sister.
async fn mem_facade(
    sisters: &Sisters,
    tool: &str,
    operation: &str,
    params: &str,
) -> Option<String> {
    let mem = sisters.memory.as_ref()?;
    let pj: serde_json::Value =
        serde_json::from_str(params).unwrap_or(serde_json::json!({}));
    let result = mem.call_tool(tool, serde_json::json!({
        "operation": operation, "params": pj,
    })).await.ok()?;
    let text = extract_text(&result);
    if text.is_empty() { None } else { Some(text) }
}

impl Sisters {
    // ═══════════════════════════════════════════════════════════════
    // FACADE ROUTERS — generic operation + params dispatch
    // ═══════════════════════════════════════════════════════════════

    /// Route an operation to the `memory_core` facade tool.
    pub async fn memory_core_facade(&self, operation: &str, params: &str) -> Option<String> {
        mem_facade(self, "memory_core", operation, params).await
    }

    /// Route an operation to the `memory_grounding` facade tool.
    pub async fn memory_grounding_facade(&self, operation: &str, params: &str) -> Option<String> {
        mem_facade(self, "memory_grounding", operation, params).await
    }

    /// Route an operation to the `memory_workspace` facade tool.
    /// Named `memory_workspace_facade` to avoid collision with memory_workspace.rs methods.
    pub async fn memory_workspace_facade(&self, operation: &str, params: &str) -> Option<String> {
        mem_facade(self, "memory_workspace", operation, params).await
    }

    /// Route an operation to the `memory_session` facade tool.
    pub async fn memory_session_facade(&self, operation: &str, params: &str) -> Option<String> {
        mem_facade(self, "memory_session", operation, params).await
    }

    /// Route an operation to the `memory_infinite` facade tool.
    pub async fn memory_infinite_facade(&self, operation: &str, params: &str) -> Option<String> {
        mem_facade(self, "memory_infinite", operation, params).await
    }

    /// Route an operation to the `memory_prophetic` facade tool.
    pub async fn memory_prophetic_facade(&self, operation: &str, params: &str) -> Option<String> {
        mem_facade(self, "memory_prophetic", operation, params).await
    }

    /// Route an operation to the `memory_collective` facade tool.
    pub async fn memory_collective_facade(&self, operation: &str, params: &str) -> Option<String> {
        mem_facade(self, "memory_collective", operation, params).await
    }

    /// Route an operation to the `memory_resurrection` facade tool.
    pub async fn memory_resurrection_facade(
        &self, operation: &str, params: &str,
    ) -> Option<String> {
        mem_facade(self, "memory_resurrection", operation, params).await
    }

    /// Route an operation to the `memory_metamemory` facade tool.
    pub async fn memory_metamemory_facade(&self, operation: &str, params: &str) -> Option<String> {
        mem_facade(self, "memory_metamemory", operation, params).await
    }

    /// Route an operation to the `memory_transcendent` facade tool.
    pub async fn memory_transcendent_facade(
        &self, operation: &str, params: &str,
    ) -> Option<String> {
        mem_facade(self, "memory_transcendent", operation, params).await
    }

    // ═══════════════════════════════════════════════════════════════
    // CORE TOOLS — memory_stats, context, temporal, conversation_log
    // ═══════════════════════════════════════════════════════════════

    /// Retrieve overall memory statistics.
    pub async fn memory_stats(&self) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let r = mem.call_tool("memory_stats", serde_json::json!({})).await.ok()?;
        let text = extract_text(&r);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Get context around a specific memory node.
    pub async fn memory_context(&self, node_id: i64, depth: u32) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let r = mem.call_tool("memory_context", serde_json::json!({
            "node_id": node_id, "depth": depth,
        })).await.ok()?;
        let text = extract_text(&r);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Query memories within a temporal range.
    pub async fn memory_temporal(&self, range_a: &str, range_b: &str) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let r = mem.call_tool("memory_temporal", serde_json::json!({
            "range_a": safe_truncate(range_a, 100),
            "range_b": safe_truncate(range_b, 100),
        })).await.ok()?;
        let text = extract_text(&r);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Log a conversation exchange into memory.
    pub async fn memory_conversation_log(
        &self, user_message: &str, agent_response: &str, topic: &str,
    ) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let r = mem.call_tool("conversation_log", serde_json::json!({
            "user_message": safe_truncate(user_message, 500),
            "agent_response": safe_truncate(agent_response, 500),
            "topic": safe_truncate(topic, 200),
        })).await.ok()?;
        let text = extract_text(&r);
        if text.is_empty() { None } else { Some(text) }
    }

    /// End the current memory session (no-summary variant).
    pub async fn memory_session_end_quiet(&self) {
        if let Some(mem) = self.memory.as_ref() {
            if let Err(e) = mem.call_tool("session_end", serde_json::json!({})).await {
                eprintln!("[hydra:memory] session_end FAILED: {}", e);
            }
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // LONGEVITY TOOLS — stats, hierarchy, consolidation, health
    // ═══════════════════════════════════════════════════════════════

    /// Get longevity statistics for a specific project.
    pub async fn memory_longevity_stats_project(&self, project_id: &str) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let r = mem.call_tool("memory_longevity_stats", serde_json::json!({
            "project_id": safe_truncate(project_id, 200),
        })).await.ok()?;
        let text = extract_text(&r);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Query the memory hierarchy for a specific layer.
    pub async fn memory_hierarchy_query(
        &self, project_id: &str, layer: &str,
    ) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let r = mem.call_tool("memory_hierarchy_query", serde_json::json!({
            "project_id": safe_truncate(project_id, 200),
            "layer": safe_truncate(layer, 100),
        })).await.ok()?;
        let text = extract_text(&r);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Navigate to a specific memory in the hierarchy.
    pub async fn memory_hierarchy_navigate(&self, memory_id: &str) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let r = mem.call_tool("memory_hierarchy_navigate", serde_json::json!({
            "memory_id": safe_truncate(memory_id, 200),
        })).await.ok()?;
        let text = extract_text(&r);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Trigger longevity consolidation for a project.
    pub async fn memory_longevity_consolidate(&self, project_id: &str) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let r = mem.call_tool("memory_longevity_consolidate", serde_json::json!({
            "project_id": safe_truncate(project_id, 200),
        })).await.ok()?;
        let text = extract_text(&r);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Check longevity health for a project.
    pub async fn memory_longevity_health(&self, project_id: &str) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let r = mem.call_tool("memory_longevity_health", serde_json::json!({
            "project_id": safe_truncate(project_id, 200),
        })).await.ok()?;
        let text = extract_text(&r);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Get or set significance for a memory in the hierarchy.
    pub async fn memory_hierarchy_significance(
        &self, memory_id: &str, set_significance: Option<f64>,
    ) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let mut args = serde_json::json!({ "memory_id": safe_truncate(memory_id, 200) });
        if let Some(sig) = set_significance {
            args["set_significance"] = serde_json::json!(sig);
        }
        let r = mem.call_tool("memory_hierarchy_significance", args).await.ok()?;
        let text = extract_text(&r);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Check embedding processing status.
    pub async fn memory_embedding_status(&self) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let r = mem.call_tool("memory_embedding_status", serde_json::json!({})).await.ok()?;
        let text = extract_text(&r);
        if text.is_empty() { None } else { Some(text) }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_memory_facades_compiles() {
        assert!(true);
    }
}
