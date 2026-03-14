//! Priority 7: Time + Vision + Cognition + Veritas + Evolve deep integrations.
//!
//! Smaller but important sister integrations that replace local implementations.

use super::connection::extract_text;
use super::cognitive::Sisters;
use hydra_native_state::utils::safe_truncate;

impl Sisters {
    // ── TIME SISTER ──

    /// Get current temporal awareness from Time sister.
    /// Replaces conversation_engine's local time context.
    pub async fn time_query_now(&self) -> Option<String> {
        let time = self.time.as_ref()?;
        let result = time.call_tool("time_query_now", serde_json::json!({})).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Check deadlines for active tasks.
    pub async fn time_check_deadline(&self, task: &str) -> Option<String> {
        let time = self.time.as_ref()?;
        let result = time.call_tool("time_check_deadline", serde_json::json!({
            "task": task,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Query temporal patterns — "when do I usually do X".
    pub async fn time_temporal_patterns(&self, query: &str) -> Option<String> {
        let time = self.time.as_ref()?;
        let result = time.call_tool("time_temporal_patterns", serde_json::json!({
            "query": query,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ── VISION SISTER ──

    /// Capture a screenshot for visual context.
    pub async fn vision_capture_screen(&self) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_capture", serde_json::json!({
            "source": "screen",
            "format": "description",
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Compare before/after visual states.
    pub async fn vision_diff(&self, before: &str, after: &str) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_diff", serde_json::json!({
            "before": before,
            "after": after,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Ground an image in context — extract meaning from visual input.
    pub async fn vision_ground(&self, image_context: &str) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_ground", serde_json::json!({
            "context": image_context,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ── COGNITION SISTER ──

    /// Update user model after a session interaction.
    pub async fn cognition_model_update_session(&self, interaction_summary: &str, session_length: u32) {
        if let Some(cog) = &self.cognition {
            if let Err(e) = cog.call_tool("cognition_model_heartbeat", serde_json::json!({
                "context": "session_end",
                "observation": { "summary": safe_truncate(interaction_summary, 300), "session_length": session_length }
            })).await { eprintln!("[hydra:extras] cognition_model_heartbeat FAILED: {}", e); }
        }
    }

    /// Predict what the user might need next.
    pub async fn cognition_predict_intent(&self, context: &str) -> Option<String> {
        let cog = self.cognition.as_ref()?;
        let result = cog.call_tool("cognition_predict", serde_json::json!({
            "context": context,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Detect behavioral drift in user patterns.
    pub async fn cognition_detect_drift(&self) -> Option<String> {
        let cog = self.cognition.as_ref()?;
        let result = cog.call_tool("cognition_drift_track", serde_json::json!({
            "window": "last_10_sessions",
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ── VERITAS SISTER ──

    /// Verify a factual claim made by the LLM.
    pub async fn veritas_verify_claim(&self, claim: &str) -> Option<String> {
        let veritas = self.veritas.as_ref()?;
        let result = veritas.call_tool("veritas_verify_claim", serde_json::json!({
            "claim": claim,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Get uncertainty score for a response.
    pub async fn veritas_uncertainty_score(&self, response: &str) -> Option<f64> {
        let veritas = self.veritas.as_ref()?;
        let result = veritas.call_tool("veritas_uncertainty_score", serde_json::json!({
            "response": safe_truncate(response, 500),
        })).await.ok()?;
        result.get("score").and_then(|v| v.as_f64())
    }

    /// Check causal validity — "does A really cause B?"
    pub async fn veritas_causal_check(&self, cause: &str, effect: &str) -> Option<String> {
        let veritas = self.veritas.as_ref()?;
        let result = veritas.call_tool("veritas_causal_check", serde_json::json!({
            "cause": cause,
            "effect": effect,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ── EVOLVE SISTER ──

    /// Record a successful action pattern for skill evolution.
    pub async fn evolve_record_pattern(&self, pattern: &str, success: bool) {
        if let Some(evolve) = &self.evolve {
            if let Err(e) = evolve.call_tool("evolve_record_pattern", serde_json::json!({
                "pattern": pattern,
                "success": success,
            })).await {
                eprintln!("[hydra:evolve] evolve_record_pattern FAILED: {}", e);
            }
        }
    }

    /// Get improvement suggestions from Evolve sister.
    pub async fn evolve_suggest_improvement(&self) -> Option<String> {
        let evolve = self.evolve.as_ref()?;
        let result = evolve.call_tool("evolve_suggest_improvement", serde_json::json!({
            "scope": "recent_sessions",
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Share compiled patterns with multi-agent collective.
    pub async fn evolve_collective_share(&self, patterns: &[String]) {
        if let Some(evolve) = &self.evolve {
            if let Err(e) = evolve.call_tool("evolve_collective_share", serde_json::json!({
                "patterns": patterns,
                "source": "cognitive_loop",
            })).await {
                eprintln!("[hydra:evolve] evolve_collective_share FAILED: {}", e);
            }
        }
    }

    // ── TIME SISTER: Session & Deadline Intelligence ──

    /// Start a time-tracked session for this conversation.
    pub async fn time_session_start(&self, user_name: &str) -> Option<String> {
        let time = self.time.as_ref()?;
        let result = time.call_tool("time_session_start", serde_json::json!({
            "session_type": "conversation",
            "agent_id": user_name,
        })).await.ok()?;
        result.get("session_id").and_then(|v| v.as_str()).map(|s| s.to_string())
    }

    /// End the current time-tracked session.
    pub async fn time_session_end(&self, summary: &str) {
        if let Some(time) = &self.time {
            if let Err(e) = time.call_tool("time_session_end", serde_json::json!({
                "summary": safe_truncate(summary, 300),
            })).await {
                eprintln!("[hydra:time] time_session_end FAILED: {}", e);
            }
        }
    }

    /// List upcoming deadlines — surface time pressure in PERCEIVE.
    pub async fn time_deadline_list(&self) -> Option<String> {
        let time = self.time.as_ref()?;
        let result = time.call_tool("time_deadline_list", serde_json::json!({
            "status": "active",
            "sort": "soonest",
            "limit": 5,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() || text.contains("No deadlines") { None } else { Some(text) }
    }

    /// Search past sessions by time reference ("yesterday", "last week").
    pub async fn time_search_sessions(&self, query: &str) -> Option<String> {
        let time = self.time.as_ref()?;
        let result = time.call_tool("time_search_sessions", serde_json::json!({
            "query": query,
            "limit": 3,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() || text.contains("No sessions") { None } else { Some(text) }
    }

    /// Analyze time usage patterns — called at session end.
    pub async fn time_analysis_patterns(&self) -> Option<String> {
        let time = self.time.as_ref()?;
        let result = time.call_tool("time_analysis_patterns", serde_json::json!({
            "window": "last_7_days",
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ── COGNITION SISTER: Deep Intelligence ──

    /// Track belief drift over time — called at session end.
    pub async fn cognition_drift_track(&self) -> Option<String> {
        let cog = self.cognition.as_ref()?;
        let result = cog.call_tool("cognition_drift_track", serde_json::json!({
            "window": "current_session",
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Simulate how the user would decide — for proactive suggestions.
    pub async fn cognition_simulate_decision(&self, scenario: &str) -> Option<String> {
        let cog = self.cognition.as_ref()?;
        let result = cog.call_tool("cognition_simulate", serde_json::json!({
            "scenario": scenario,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ── VERITAS SISTER: Response Verification ──

    /// Score confidence in a response before delivery.
    pub async fn veritas_score_confidence(&self, response: &str) -> Option<f64> {
        let veritas = self.veritas.as_ref()?;
        let result = veritas.call_tool("veritas_score_confidence", serde_json::json!({
            "claim": safe_truncate(response, 500),
        })).await.ok()?;
        result.get("confidence").and_then(|v| v.as_f64())
    }

    /// Check consistency of response with past statements.
    pub async fn veritas_check_consistency(&self, response: &str, context: &str) -> Option<String> {
        let veritas = self.veritas.as_ref()?;
        let result = veritas.call_tool("veritas_check_consistency", serde_json::json!({
            "text": safe_truncate(response, 500),
            "context": safe_truncate(context, 300),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ── PLANNING SISTER: Deeper Integration ──

    /// List commitments approaching deadline — surface in PERCEIVE.
    pub async fn planning_commitments_due_soon(&self) -> Option<String> {
        let planning = self.planning.as_ref()?;
        let result = planning.call_tool("planning_commitment", serde_json::json!({
            "operation": "due_soon",
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() || text.contains("No commitments") { None } else { Some(text) }
    }

    /// Decompose a complex task into subtasks.
    pub async fn planning_decompose_goal(&self, goal: &str) -> Option<Vec<String>> {
        let planning = self.planning.as_ref()?;
        let result = planning.call_tool("planning_goal", serde_json::json!({
            "operation": "decompose",
            "params": { "description": goal }
        })).await.ok()?;
        result.get("subtasks")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
    }

    /// Identify recurring themes across sessions.
    pub async fn planning_identify_themes(&self) -> Option<String> {
        let planning = self.planning.as_ref()?;
        let result = planning.call_tool("planning_singularity", serde_json::json!({
            "operation": "themes",
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ── MEMORY SISTER: Periodic Maintenance ──

    /// Process memory metabolism — consolidation/decay for health.
    pub async fn memory_metabolism_process(&self) {
        if let Some(mem) = &self.memory {
            if let Err(e) = mem.call_tool("memory_metabolism_process", serde_json::json!({
                "mode": "auto",
            })).await {
                eprintln!("[hydra:memory] memory_metabolism_process FAILED: {}", e);
            }
        }
    }

    /// Identify knowledge gaps for proactive learning.
    pub async fn memory_meta_gaps(&self, context: &str) -> Option<String> {
        let mem = self.memory.as_ref()?;
        let result = mem.call_tool("memory_meta_gaps", serde_json::json!({
            "context": context,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() || text.contains("No gaps") { None } else { Some(text) }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_module_exists() {
        // Structural test — ensures this module compiles
        assert!(true);
    }
}
