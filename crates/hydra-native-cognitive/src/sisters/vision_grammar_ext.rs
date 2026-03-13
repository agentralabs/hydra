//! Vision sister grammar extras — status, update, and pin tools.
//!
//! Grammar learn and get are in browser_agent.rs. This module covers
//! the remaining grammar management tools: status, update, and pin.

use super::connection::extract_text;
use super::cognitive::Sisters;
use hydra_native_state::utils::safe_truncate;

impl Sisters {
    // ── Grammar Management (3 tools) ──

    /// Get the current grammar learning status across all domains.
    pub async fn vision_grammar_status(&self) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_grammar_status", serde_json::json!({})).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Update grammar rules for a specific domain.
    pub async fn vision_grammar_update(
        &self, domain: &str, grammar: &str,
    ) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_grammar_update", serde_json::json!({
            "domain": safe_truncate(domain, 500),
            "grammar": safe_truncate(grammar, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Pin grammar for a domain so it persists across sessions.
    pub async fn vision_grammar_pin(&self, domain: &str) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_grammar_pin", serde_json::json!({
            "domain": safe_truncate(domain, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_vision_grammar_ext_compiles() {
        assert!(true);
    }
}
