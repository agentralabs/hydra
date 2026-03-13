//! Communication Agent — advanced multi-channel messaging, inter-agent
//! coordination, emotional tracking, and federated sync.
//!
//! Extends comm_deep.rs (which has: register, deregister, send, broadcast,
//! inbox, swarm_health, session start/log/context/end) with forensics,
//! affect tracking, trust verification, semantic/temporal search,
//! collaboration, consent, federation, hive coordination, and workspace.

use super::connection::extract_text;
use super::cognitive::Sisters;
use hydra_native_state::utils::safe_truncate;

impl Sisters {
    // ═══════════════════════════════════════════════════════════════
    // FORENSICS & DIAGNOSTICS
    // ═══════════════════════════════════════════════════════════════

    /// Debug communication issues — trace message delivery, find drops.
    pub async fn comm_forensics(&self, query: &str) -> Option<String> {
        let comm = self.comm.as_ref()?;
        let result = comm.call_tool("comm_forensics", serde_json::json!({
            "query": safe_truncate(query, 300),
            "include_trace": true,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ═══════════════════════════════════════════════════════════════
    // AFFECT & EMOTIONAL TRACKING
    // ═══════════════════════════════════════════════════════════════

    /// Track emotional context in a message — sentiment, urgency, tone.
    /// Used to adapt communication style and detect frustration early.
    pub async fn comm_affect(&self, message: &str) -> Option<String> {
        let comm = self.comm.as_ref()?;
        let result = comm.call_tool("comm_affect", serde_json::json!({
            "message": safe_truncate(message, 500),
            "track_sentiment": true,
            "track_urgency": true,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ═══════════════════════════════════════════════════════════════
    // TRUST & VERIFICATION
    // ═══════════════════════════════════════════════════════════════

    /// Verify trust level for an agent before allowing sensitive operations.
    pub async fn comm_trust_verify(&self, agent: &str) -> Option<String> {
        let comm = self.comm.as_ref()?;
        let result = comm.call_tool("comm_trust", serde_json::json!({
            "agent_id": agent,
            "operation": "verify",
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ═══════════════════════════════════════════════════════════════
    // SEARCH: SEMANTIC & TEMPORAL
    // ═══════════════════════════════════════════════════════════════

    /// Semantic search across all communication history.
    /// Finds messages by meaning, not just keywords.
    pub async fn comm_semantic_search(&self, query: &str) -> Option<String> {
        let comm = self.comm.as_ref()?;
        let result = comm.call_tool("comm_semantic", serde_json::json!({
            "query": safe_truncate(query, 300),
            "max_results": 10,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Time-based message search — "messages from yesterday", "last hour".
    pub async fn comm_temporal_search(&self, query: &str) -> Option<String> {
        let comm = self.comm.as_ref()?;
        let result = comm.call_tool("comm_temporal", serde_json::json!({
            "query": safe_truncate(query, 300),
            "max_results": 10,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// General message search across communication history.
    pub async fn comm_message_search(&self, query: &str) -> Option<String> {
        let comm = self.comm.as_ref()?;
        let result = comm.call_tool("comm_message_search", serde_json::json!({
            "query": safe_truncate(query, 300),
            "max_results": 10,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ═══════════════════════════════════════════════════════════════
    // COLLABORATION & MULTI-AGENT
    // ═══════════════════════════════════════════════════════════════

    /// Initiate a collaborative task across multiple agents.
    /// Comm sister handles message routing, turn-taking, and result aggregation.
    pub async fn comm_collaboration(
        &self,
        task: &str,
        agents: &[String],
    ) -> Option<String> {
        let comm = self.comm.as_ref()?;
        let result = comm.call_tool("comm_collaboration", serde_json::json!({
            "task": safe_truncate(task, 300),
            "agents": agents,
            "mode": "collaborative",
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ═══════════════════════════════════════════════════════════════
    // CONSENT & DATA SHARING
    // ═══════════════════════════════════════════════════════════════

    /// Check or grant data sharing consent for a given scope.
    /// Ensures privacy compliance before sharing context across agents.
    pub async fn comm_consent(&self, scope: &str) -> Option<String> {
        let comm = self.comm.as_ref()?;
        let result = comm.call_tool("comm_consent", serde_json::json!({
            "scope": scope,
            "operation": "check",
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ═══════════════════════════════════════════════════════════════
    // FEDERATION & HIVE
    // ═══════════════════════════════════════════════════════════════

    /// Sync communication state across federated instances.
    pub async fn comm_federation_sync(&self) -> Option<String> {
        let comm = self.comm.as_ref()?;
        let result = comm.call_tool("comm_federation", serde_json::json!({
            "operation": "sync",
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Hive/swarm coordination — distribute a task across the collective.
    pub async fn comm_hive_coordinate(&self, task: &str) -> Option<String> {
        let comm = self.comm.as_ref()?;
        let result = comm.call_tool("comm_hive", serde_json::json!({
            "task": safe_truncate(task, 300),
            "mode": "coordinate",
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ═══════════════════════════════════════════════════════════════
    // WORKSPACE & SESSION CONTEXT
    // ═══════════════════════════════════════════════════════════════

    /// Query workspace communication context — project-scoped messages.
    pub async fn comm_workspace(&self, query: &str) -> Option<String> {
        let comm = self.comm.as_ref()?;
        let result = comm.call_tool("comm_workspace", serde_json::json!({
            "query": safe_truncate(query, 300),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Query session context with a specific question.
    /// More targeted than comm_session_context() — asks a specific question.
    pub async fn comm_session_context_query(&self, query: &str) -> Option<String> {
        let comm = self.comm.as_ref()?;
        let result = comm.call_tool("comm_session", serde_json::json!({
            "operation": "query_context",
            "params": {
                "query": safe_truncate(query, 300),
            }
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_comm_agent_compiles() {
        assert!(true);
    }
}
