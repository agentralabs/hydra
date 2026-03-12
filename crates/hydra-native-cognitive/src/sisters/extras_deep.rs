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
    pub async fn cognition_model_update_session(
        &self,
        interaction_summary: &str,
        session_length: u32,
    ) {
        if let Some(cog) = &self.cognition {
            let _ = cog.call_tool("cognition_model_update", serde_json::json!({
                "context": "session_end",
                "observation": {
                    "summary": safe_truncate(interaction_summary, 300),
                    "session_length": session_length,
                }
            })).await;
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
        let result = cog.call_tool("cognition_detect_drift", serde_json::json!({
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
            let _ = evolve.call_tool("evolve_record_pattern", serde_json::json!({
                "pattern": pattern,
                "success": success,
            })).await;
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
            let _ = evolve.call_tool("evolve_collective_share", serde_json::json!({
                "patterns": patterns,
                "source": "cognitive_loop",
            })).await;
        }
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
