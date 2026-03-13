//! V3 Immortal Architecture tools — capture, retrieve, search, verify,
//! stats, and session resume for the immortal memory graph.
//!
//! Sister-first, local-fallback pattern for all V3 memory operations.

use super::connection::extract_text;
use super::cognitive::Sisters;
use hydra_native_state::utils::safe_truncate;

impl Sisters {
    /// CAPTURE: Store a user or assistant message in the immortal log.
    pub async fn memory_capture_message(&self, content: &str) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let result = mem.call_tool("memory_capture_message", serde_json::json!({
            "content": safe_truncate(content, 500),
        })).await.ok()?;
        let extracted = extract_text(&result);
        if extracted.is_empty() { None } else { Some(extracted) }
    }

    /// CAPTURE: Store a tool invocation and its result.
    pub async fn memory_capture_tool(
        &self,
        tool: &str,
        result: &str,
    ) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let res = mem.call_tool("memory_capture_tool", serde_json::json!({
            "tool": tool,
            "result": safe_truncate(result, 500),
        })).await.ok()?;
        let extracted = extract_text(&res);
        if extracted.is_empty() { None } else { Some(extracted) }
    }

    /// CAPTURE: Store a file operation (read, write, delete, etc.).
    pub async fn memory_capture_file(
        &self,
        path: &str,
        operation: &str,
    ) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let res = mem.call_tool("memory_capture_file", serde_json::json!({
            "path": path,
            "operation": operation,
        })).await.ok()?;
        let extracted = extract_text(&res);
        if extracted.is_empty() { None } else { Some(extracted) }
    }

    /// CAPTURE: Store a decision with its reasoning chain.
    pub async fn memory_capture_decision(
        &self,
        decision: &str,
        reasoning: &str,
    ) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let res = mem.call_tool("memory_capture_decision", serde_json::json!({
            "decision": safe_truncate(decision, 300),
            "reasoning": safe_truncate(reasoning, 300),
        })).await.ok()?;
        let extracted = extract_text(&res);
        if extracted.is_empty() { None } else { Some(extracted) }
    }

    /// CAPTURE: Store a boundary event (session start/end, context switch).
    pub async fn memory_capture_boundary(
        &self,
        boundary_type: &str,
    ) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let res = mem.call_tool("memory_capture_boundary", serde_json::json!({
            "boundary_type": boundary_type,
        })).await.ok()?;
        let extracted = extract_text(&res);
        if extracted.is_empty() { None } else { Some(extracted) }
    }

    /// RETRIEVE: General-purpose memory retrieval by query.
    pub async fn memory_retrieve(&self, query: &str) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let result = mem.call_tool("memory_retrieve", serde_json::json!({
            "query": query,
        })).await.ok()?;
        let extracted = extract_text(&result);
        if extracted.is_empty() { None } else { Some(extracted) }
    }

    /// RETRIEVE: Resurrect a memory from the immortal graph.
    /// Used when a memory may have been compacted or archived.
    pub async fn memory_resurrect_memory(
        &self,
        query: &str,
    ) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let result = mem.call_tool("memory_resurrect", serde_json::json!({
            "query": query,
        })).await.ok()?;
        let extracted = extract_text(&result);
        if extracted.is_empty() { None } else { Some(extracted) }
    }

    /// SEARCH: Temporal search — find memories by time proximity.
    pub async fn memory_search_temporal_v3(
        &self,
        query: &str,
    ) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let result = mem.call_tool("memory_search_temporal", serde_json::json!({
            "query": query,
        })).await.ok()?;
        let extracted = extract_text(&result);
        if extracted.is_empty() { None } else { Some(extracted) }
    }

    /// SEARCH: Semantic search — find memories by meaning similarity.
    pub async fn memory_search_semantic_v3(
        &self,
        query: &str,
    ) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let result = mem.call_tool("memory_search_semantic", serde_json::json!({
            "query": query,
        })).await.ok()?;
        let extracted = extract_text(&result);
        if extracted.is_empty() { None } else { Some(extracted) }
    }

    /// SEARCH: Entity search — find memories related to a named entity.
    pub async fn memory_search_entity(
        &self,
        entity: &str,
    ) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let result = mem.call_tool("memory_search_entity", serde_json::json!({
            "entity": entity,
        })).await.ok()?;
        let extracted = extract_text(&result);
        if extracted.is_empty() { None } else { Some(extracted) }
    }

    /// VERIFY: Check the integrity of the immortal memory graph.
    /// Returns diagnostics about orphaned nodes, broken edges, etc.
    pub async fn memory_verify_integrity(&self) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let result = mem.call_tool("memory_verify_integrity", serde_json::json!({}))
            .await.ok()?;
        let extracted = extract_text(&result);
        if extracted.is_empty() { None } else { Some(extracted) }
    }

    /// STATS: Get V3 memory graph statistics (node count, edge count, etc.).
    pub async fn memory_v3_stats(&self) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let result = mem.call_tool("memory_v3_stats", serde_json::json!({}))
            .await.ok()?;
        let extracted = extract_text(&result);
        if extracted.is_empty() { None } else { Some(extracted) }
    }

    /// SESSION: Resume from the V3 immortal session log.
    /// Returns the last session context for continuity across restarts.
    pub async fn memory_v3_session_resume(&self) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let result = mem.call_tool("memory_v3_session_resume", serde_json::json!({}))
            .await.ok()?;
        let extracted = extract_text(&result);
        if extracted.is_empty() { None } else { Some(extracted) }
    }
}

#[cfg(test)]
mod tests {
    /// Compile-time check: ensure this module builds and imports resolve.
    #[test]
    fn memory_v3_compiles() {
        // Compilation of this module is the test.
        assert!(true);
    }
}
