//! Priority 1: Deep Memory Integration — causal chains, decision storage,
//! structured episodes with edges, Ghost Writer summaries.
//!
//! Sister-first, local-fallback pattern for all memory operations.

use super::connection::extract_text;
use super::cognitive::Sisters;
use hydra_native_state::utils::safe_truncate;

impl Sisters {
    /// PERCEIVE: Causal chain query for "why" questions.
    /// Queries decisions and their reasoning context for causal understanding.
    pub async fn memory_causal_query(&self, text: &str) -> Option<String> {
        let mem = self.memory.as_ref()?;
        // No event_type filter — let Memory sister rank by relevance
        let result = mem.call_tool("memory_query", serde_json::json!({
            "query": text,
            "max_results": 20,
            "sort_by": "highest_confidence",
            "include_edges": true,
        })).await.ok()?;
        let extracted = extract_text(&result);
        if extracted.is_empty() || extracted.contains("No memories") || extracted.contains("Invalid params") {
            None
        } else {
            Some(extracted)
        }
    }

    /// PERCEIVE: Get a specific memory node by reference.
    /// Used when user references a past event: "that thing we did with X".
    pub async fn memory_get_node(&self, reference: &str) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let result = mem.call_tool("memory_get", serde_json::json!({
            "query": reference,
            "include_edges": true,
        })).await.ok()?;
        let extracted = extract_text(&result);
        if extracted.is_empty() { None } else { Some(extracted) }
    }

    /// LEARN: Store a decision with proper edge types.
    /// Connects decisions to their reasoning context via caused_by edges.
    pub async fn memory_store_decision(&self, decision: &str, reasoning: &str, context: &str) {
        if let Some(mem) = &self.memory {
            match mem.call_tool("memory_add", serde_json::json!({
                "event_type": "decision", "content": decision, "confidence": 0.9,
                "metadata": { "reasoning": safe_truncate(reasoning, 300),
                    "context": safe_truncate(context, 200), "edge_type": "caused_by" }
            })).await {
                Ok(_) => eprintln!("[hydra:memory] store_decision OK"),
                Err(e) => eprintln!("[hydra:memory] store_decision FAILED: {}", e),
            }
        }
    }

    /// LEARN: Store tool output as evidence linked to the action.
    /// Creates an evidence node with edges to the command that produced it.
    pub async fn memory_store_evidence(&self, action: &str, output: &str, success: bool) {
        if let Some(mem) = &self.memory {
            match mem.call_tool("memory_add", serde_json::json!({
                "event_type": "evidence",
                "content": format!("Action: {}\nResult: {}\nSuccess: {}", safe_truncate(action, 100), safe_truncate(output, 300), success),
                "confidence": if success { 0.85 } else { 0.7 },
                "metadata": { "edge_type": "produced_by", "action": safe_truncate(action, 100) }
            })).await {
                Ok(_) => eprintln!("[hydra:memory] store_evidence OK"),
                Err(e) => eprintln!("[hydra:memory] store_evidence FAILED: {}", e),
            }
        }
    }

    /// LEARN: Store an obstacle resolution with caused_by edge to the error.
    pub async fn memory_store_resolution(&self, error: &str, solution: &str) {
        if let Some(mem) = &self.memory {
            match mem.call_tool("memory_add", serde_json::json!({
                "event_type": "resolution",
                "content": format!("Error: {}\nSolution: {}", safe_truncate(error, 200), safe_truncate(solution, 200)),
                "confidence": 0.9,
                "metadata": { "edge_type": "caused_by", "error_context": safe_truncate(error, 100) }
            })).await {
                Ok(_) => eprintln!("[hydra:memory] store_resolution OK"),
                Err(e) => eprintln!("[hydra:memory] store_resolution FAILED: {}", e),
            }
        }
    }

    /// LEARN: Store test results as structured episode with project edges.
    pub async fn memory_store_test_results(&self, project: &str, language: &str, passed: u32, failed: u32, total: u32) {
        if let Some(mem) = &self.memory {
            match mem.call_tool("memory_add", serde_json::json!({
                "event_type": "episode",
                "content": format!("Test results for {}: {}/{} passed ({} failed)", project, passed, total, failed),
                "confidence": 0.95,
                "metadata": { "edge_type": "related_to", "project": project, "language": language,
                    "test_passed": passed, "test_failed": failed, "test_total": total }
            })).await {
                Ok(_) => eprintln!("[hydra:memory] store_test_results OK"),
                Err(e) => eprintln!("[hydra:memory] store_test_results FAILED: {}", e),
            }
        }
    }

    /// LEARN: Request Ghost Writer summary after long sessions.
    /// Ghost Writer consolidates conversation into a coherent narrative.
    pub async fn memory_ghost_write(&self, messages: &[(String, String)]) -> Option<String> {
        let mem = self.memory.as_ref()?;
        if messages.len() < 10 {
            return None; // Only summarize long sessions
        }
        let conversation: Vec<serde_json::Value> = messages.iter()
            .take(50) // cap at 50 messages
            .map(|(role, content)| serde_json::json!({
                "role": role,
                "content": safe_truncate(content, 200),
            }))
            .collect();
        let result = mem.call_tool("memory_ghost_write", serde_json::json!({
            "conversation": conversation,
            "style": "summary",
        })).await.ok()?;
        let extracted = extract_text(&result);
        if extracted.is_empty() { None } else { Some(extracted) }
    }

    /// SESSION: Resume previous session — returns last session context for continuity.
    /// Called once at session start. Returns what happened last time for "where did we stop?" queries.
    pub async fn memory_session_resume(&self) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let result = mem.call_tool("memory_session_resume", serde_json::json!({
            "include_summary": true,
            "include_last_topics": true,
            "max_context_tokens": 500,
        })).await.ok()?;
        let extracted = extract_text(&result);
        if extracted.is_empty() || extracted.contains("No previous session") {
            None
        } else {
            eprintln!("[hydra:session] Resumed previous session context ({} chars)", extracted.len());
            Some(extracted)
        }
    }

    /// SESSION: Start a new memory session for tracking this conversation.
    pub async fn memory_session_start(&self, user_name: &str) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let result = mem.call_tool("session_start", serde_json::json!({
            "agent_id": user_name,
            "session_type": "conversation",
        })).await.ok()?;
        result.get("session_id").and_then(|v| v.as_str()).map(|s| s.to_string())
    }

    /// SESSION: End current session — persists state for future resume.
    pub async fn memory_session_end(&self, summary: &str) {
        if let Some(mem) = &self.memory {
            match mem.call_tool("session_end", serde_json::json!({
                "summary": safe_truncate(summary, 500), "persist": true,
            })).await {
                Ok(_) => eprintln!("[hydra:memory] session_end OK"),
                Err(e) => eprintln!("[hydra:memory] session_end FAILED: {}", e),
            }
        }
    }

    /// LEARN: Capture user+hydra exchange in the immortal V3 log.
    /// This is what enables "where did we stop?" — every exchange is captured.
    pub async fn memory_capture_exchange(&self, user_msg: &str, hydra_response: &str) {
        if let Some(mem) = &self.memory {
            match mem.call_tool("memory_capture_message", serde_json::json!({
                "role": "user",
                "content": safe_truncate(user_msg, 500),
                "importance": "normal",
            })).await {
                Ok(_) => eprintln!("[hydra:memory] capture user msg OK"),
                Err(e) => eprintln!("[hydra:memory] capture user msg FAILED: {}", e),
            }
            match mem.call_tool("memory_capture_message", serde_json::json!({
                "role": "assistant",
                "content": safe_truncate(hydra_response, 500),
                "importance": "normal",
            })).await {
                Ok(_) => eprintln!("[hydra:memory] capture hydra response OK"),
                Err(e) => eprintln!("[hydra:memory] capture hydra response FAILED: {}", e),
            }
        } else {
            eprintln!("[hydra:memory] capture skipped — memory sister not connected");
        }
    }

    /// Store an episode in memory — used by capability handlers that bypass LEARN phase.
    pub async fn memory_store_episode(&self, content: &str, metadata: &str) {
        if let Some(mem) = &self.memory {
            match mem.call_tool("memory_add", serde_json::json!({
                "event_type": "episode",
                "content": safe_truncate(content, 500),
                "confidence": 0.9,
                "metadata": metadata,
            })).await {
                Ok(_) => eprintln!("[hydra:memory] episode stored OK"),
                Err(e) => eprintln!("[hydra:memory] episode store FAILED: {}", e),
            }
        }
    }

    /// PERCEIVE: Predict what memories will be needed based on current input.
    /// Returns pre-loaded relevant context — smarter than generic memory_query.
    pub async fn memory_predict_context(&self, text: &str) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let result = mem.call_tool("memory_predict", serde_json::json!({
            "context": text,
            "max_results": 20,
            "include_confidence": true,
        })).await.ok()?;
        let extracted = extract_text(&result);
        if extracted.is_empty() || extracted.contains("No predictions") {
            None
        } else {
            Some(extracted)
        }
    }

    /// PERCEIVE: Check for déjà vu — has user visited this topic before?
    /// Detects returning conversations for context continuity.
    pub async fn memory_dejavu_check(&self, text: &str) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let result = mem.call_tool("memory_dejavu_check", serde_json::json!({
            "context": text,
        })).await.ok()?;
        let extracted = extract_text(&result);
        if extracted.is_empty() || extracted.contains("No déjà vu") || extracted.contains("No deja vu") {
            None
        } else {
            eprintln!("[hydra:dejavu] Detected returning topic: {}", safe_truncate(&extracted, 80));
            Some(extracted)
        }
    }
}

/// Detect if user is asking a "why" question (triggers causal chain query).
pub fn is_why_question(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.starts_with("why ")
        || lower.starts_with("why?")
        || lower.contains("why did")
        || lower.contains("why do")
        || lower.contains("what was the reason")
        || lower.contains("what's the reason")
        || lower.contains("how come")
        || lower.contains("why was")
        || lower.contains("why is")
        || lower.contains("explain the decision")
        || lower.contains("why we chose")
        || lower.contains("why did we")
}

/// Detect if user is referencing a specific past event.
pub fn is_past_reference(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("that thing")
        || lower.contains("remember when")
        || lower.contains("that time")
        || lower.contains("the one where")
        || lower.contains("back when")
        || lower.contains("that error")
        || lower.contains("that bug")
        || lower.contains("that issue")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_why_question_detection() {
        assert!(is_why_question("why did we choose PostgreSQL?"));
        assert!(is_why_question("Why is this failing?"));
        assert!(is_why_question("how come this doesn't work?"));
        assert!(is_why_question("explain the decision to use Rust"));
        assert!(is_why_question("why did we go with this approach?"));
        assert!(!is_why_question("help me with code"));
        assert!(!is_why_question("what is PostgreSQL?"));
    }

    #[test]
    fn test_past_reference_detection() {
        assert!(is_past_reference("remember when we fixed the auth bug?"));
        assert!(is_past_reference("that thing we did with the database"));
        assert!(is_past_reference("that error from yesterday"));
        assert!(!is_past_reference("fix this bug"));
        assert!(!is_past_reference("help me write code"));
    }
}
