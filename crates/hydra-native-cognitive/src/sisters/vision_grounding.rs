//! Vision sister Grounding V2-V3 tools — claim grounding, truth tracking,
//! hallucination detection, and visual comparison pipelines.

use super::connection::extract_text;
use super::cognitive::Sisters;
use hydra_native_state::utils::safe_truncate;

impl Sisters {
    // ── Grounding V2 ──
    // NOTE: vision_ground() already defined in extras_deep.rs

    /// Query visual evidence for a given search query.
    pub async fn vision_evidence_query(&self, query: &str) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_evidence", serde_json::json!({
            "query": safe_truncate(query, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Suggest corrections based on visual evidence.
    pub async fn vision_suggest_correction(&self, query: &str) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_suggest", serde_json::json!({
            "query": safe_truncate(query, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Ground a specific claim with full evidence chain.
    pub async fn vision_ground_claim(&self, claim: &str) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_ground_claim", serde_json::json!({
            "claim": safe_truncate(claim, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Verify a claim against captured visual state.
    pub async fn vision_verify_claim(&self, claim: &str) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_verify_claim", serde_json::json!({
            "claim": safe_truncate(claim, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Cite a specific visual capture as evidence.
    pub async fn vision_cite(&self, capture_id: &str) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_cite", serde_json::json!({
            "capture_id": safe_truncate(capture_id, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Find visual evidence that contradicts a claim.
    pub async fn vision_contradict(&self, claim: &str) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_contradict", serde_json::json!({
            "claim": safe_truncate(claim, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ── Hallucination Detection ──

    /// Check output for hallucinations against visual evidence.
    pub async fn vision_hallucination_check(&self, output: &str) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_hallucination_check", serde_json::json!({
            "output": safe_truncate(output, 1000),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Fix hallucinations in output using visual evidence.
    pub async fn vision_hallucination_fix(&self, output: &str) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_hallucination_fix", serde_json::json!({
            "output": safe_truncate(output, 1000),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ── Truth Tracking V3 ──

    /// Check if a claim is still true based on latest visual evidence.
    pub async fn vision_truth_check(&self, claim: &str) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_truth_check", serde_json::json!({
            "claim": safe_truncate(claim, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Refresh truth status of a claim with a new visual capture.
    pub async fn vision_truth_refresh(&self, claim: &str) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_truth_refresh", serde_json::json!({
            "claim": safe_truncate(claim, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Get truth history for a claim over time.
    pub async fn vision_truth_history(&self, claim: &str) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_truth_history", serde_json::json!({
            "claim": safe_truncate(claim, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ── Visual Comparison ──

    /// Compare two visual contexts side by side.
    pub async fn vision_compare_contexts(
        &self, ctx_a: &str, ctx_b: &str,
    ) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_compare_contexts", serde_json::json!({
            "context_a": safe_truncate(ctx_a, 500),
            "context_b": safe_truncate(ctx_b, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Compare visual state of two different sites.
    pub async fn vision_compare_sites(
        &self, url_a: &str, url_b: &str,
    ) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_compare_sites", serde_json::json!({
            "url_a": safe_truncate(url_a, 500),
            "url_b": safe_truncate(url_b, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Compare visual state across different versions of a URL.
    pub async fn vision_compare_versions(&self, url: &str) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_compare_versions", serde_json::json!({
            "url": safe_truncate(url, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Compare visual state across different devices for a URL.
    pub async fn vision_compare_devices(&self, url: &str) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_compare_devices", serde_json::json!({
            "url": safe_truncate(url, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_vision_grounding_module_loads() {
        assert!(true);
    }
}
