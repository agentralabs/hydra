//! Memory Extended — additional memory tools not covered by memory_deep.rs.
//!
//! memory_deep.rs already has: causal_query, get_node, store_decision,
//! store_evidence, store_resolution, store_test_results, ghost_write,
//! capture_exchange, session_resume/start/end, predict_context, dejavu_check.
//!
//! extras_deep.rs already has: memory_metabolism_process, memory_meta_gaps.
//!
//! This file adds: dream_start, traverse, resolve, context_switch,
//! evidence verification, suggest, capture_file_change, longevity_stats.

use super::connection::extract_text;
use super::cognitive::Sisters;
use hydra_native_state::utils::safe_truncate;

impl Sisters {
    // ═══════════════════════════════════════════════════════════════
    // DREAM & IDLE PROCESSING
    // ═══════════════════════════════════════════════════════════════

    /// Start idle dreaming — Memory sister consolidates and connects
    /// memories in the background when the user is not active.
    pub async fn memory_dream_start(&self, context: &str) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let result = mem.call_tool("memory_dream_start", serde_json::json!({
            "context": safe_truncate(context, 300),
            "mode": "consolidate",
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ═══════════════════════════════════════════════════════════════
    // DEEP GRAPH TRAVERSAL
    // ═══════════════════════════════════════════════════════════════

    /// Deep graph traversal — follow edges through the memory graph.
    /// More thorough than causal_query: explores all edge types.
    pub async fn memory_traverse(&self, query: &str) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let result = mem.call_tool("memory_traverse", serde_json::json!({
            "query": safe_truncate(query, 300),
            "max_depth": 7,
            "include_all_edges": true,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ═══════════════════════════════════════════════════════════════
    // CONFLICT RESOLUTION
    // ═══════════════════════════════════════════════════════════════

    /// Resolve conflicting memories — when two memories contradict,
    /// Memory sister evaluates evidence and picks the most reliable.
    pub async fn memory_resolve(&self, conflict: &str) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let result = mem.call_tool("memory_resolve", serde_json::json!({
            "conflict": safe_truncate(conflict, 400),
            "strategy": "evidence_weighted",
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ═══════════════════════════════════════════════════════════════
    // CONTEXT SWITCHING
    // ═══════════════════════════════════════════════════════════════

    /// Context switch — transition memory focus from one topic to another.
    /// Saves the old context and loads relevant memories for the new one.
    pub async fn memory_context_switch(
        &self,
        from: &str,
        to: &str,
    ) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let result = mem.call_tool("memory_context_switch", serde_json::json!({
            "from_context": safe_truncate(from, 200),
            "to_context": safe_truncate(to, 200),
            "persist_old": true,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ═══════════════════════════════════════════════════════════════
    // EVIDENCE & VERIFICATION
    // ═══════════════════════════════════════════════════════════════

    /// Evidence-based memory verification — check if a claim is supported
    /// by stored memories and their confidence scores.
    pub async fn memory_evidence(&self, claim: &str) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let result = mem.call_tool("memory_evidence", serde_json::json!({
            "claim": safe_truncate(claim, 400),
            "include_confidence": true,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ═══════════════════════════════════════════════════════════════
    // PROACTIVE SUGGESTIONS
    // ═══════════════════════════════════════════════════════════════

    /// Suggest relevant context when user seems stuck or idle.
    /// Memory sister surfaces potentially useful past experiences.
    pub async fn memory_suggest(&self, context: &str) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let result = mem.call_tool("memory_suggest", serde_json::json!({
            "context": safe_truncate(context, 400),
            "max_suggestions": 3,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ═══════════════════════════════════════════════════════════════
    // FILE CHANGE TRACKING
    // ═══════════════════════════════════════════════════════════════

    /// Capture a file change in memory — links code changes to conversation.
    pub async fn memory_capture_file_change(&self, path: &str, change: &str) {
        if let Some(mem) = &self.memory {
            if let Err(e) = mem.call_tool("memory_add", serde_json::json!({
                "event_type": "file_change",
                "content": format!("File: {}\nChange: {}",
                    safe_truncate(path, 200),
                    safe_truncate(change, 300)),
                "confidence": 0.95,
                "metadata": {
                    "edge_type": "produced_by",
                    "file_path": safe_truncate(path, 200),
                }
            })).await {
                eprintln!("[hydra:memory] memory_add FAILED: {}", e);
            }
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // LONGEVITY & HEALTH
    // ═══════════════════════════════════════════════════════════════

    /// Get longevity statistics — memory health, graph size, decay rate.
    pub async fn memory_longevity_stats(&self) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let result = mem.call_tool("memory_longevity_stats", serde_json::json!({
            "include_graph_stats": true,
            "include_decay_info": true,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Query recent observations from memory for metabolism.
    pub async fn memory_query_observations(&self, max: usize) -> Option<Vec<String>> {
        let mem = self.memory.as_ref()?;
        let result = mem.call_tool("memory_search_semantic_v3", serde_json::json!({
            "query": "observation",
            "limit": max,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { return None; }
        Some(text.lines().map(|l| l.to_string()).filter(|l| !l.is_empty()).collect())
    }

    /// Query crystallized beliefs from memory.
    pub async fn memory_query_crystallizations(&self, max: usize) -> Option<Vec<String>> {
        let mem = self.memory.as_ref()?;
        let result = mem.call_tool("memory_search_semantic_v3", serde_json::json!({
            "query": "crystallized",
            "limit": max,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { return None; }
        Some(text.lines().map(|l| l.to_string()).filter(|l| !l.is_empty()).collect())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_memory_extended_compiles() {
        assert!(true);
    }
}
