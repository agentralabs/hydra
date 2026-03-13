//! Phase G Priority 3-4: Veritas + Aegis expanded integration.
//!
//! Veritas: claim extraction, causal reasoning, synthesis, question generation.
//! Aegis: code confidence, correction hints, rollback, streaming validation.

use super::connection::extract_text;
use super::cognitive::Sisters;
use hydra_native_state::utils::safe_truncate;

impl Sisters {
    // ── Veritas Extended Tools ──

    /// Extract claims from a response for verification.
    pub async fn veritas_extract_claims(&self, response: &str) -> Option<Vec<String>> {
        let veritas = self.veritas.as_ref()?;
        let result = veritas.call_tool("veritas_extract_claims", serde_json::json!({
            "text": safe_truncate(response, 500),
        })).await.ok()?;
        result.get("claims")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
    }

    /// Causal reasoning — validate "A causes B" reasoning chains.
    pub async fn veritas_reason_causally(&self, reasoning: &str) -> Option<String> {
        let veritas = self.veritas.as_ref()?;
        let result = veritas.call_tool("veritas_reason_causally", serde_json::json!({
            "reasoning": safe_truncate(reasoning, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Synthesize multiple information sources into coherent output.
    pub async fn veritas_synthesize(&self, sources: &[String]) -> Option<String> {
        let veritas = self.veritas.as_ref()?;
        let result = veritas.call_tool("veritas_synthesize", serde_json::json!({
            "sources": sources,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Generate clarifying questions when ambiguity detected.
    pub async fn veritas_generate_question(&self, context: &str) -> Option<String> {
        let veritas = self.veritas.as_ref()?;
        let result = veritas.call_tool("veritas_generate_question", serde_json::json!({
            "context": safe_truncate(context, 300),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Check uncertainty level for complex tasks.
    pub async fn veritas_check_uncertainty(&self, text: &str) -> Option<String> {
        let veritas = self.veritas.as_ref()?;
        let result = veritas.call_tool("veritas_check_uncertainty", serde_json::json!({
            "text": safe_truncate(text, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Compile structured intent from raw text.
    pub async fn veritas_compile_intent(&self, text: &str) -> Option<String> {
        let veritas = self.veritas.as_ref()?;
        let result = veritas.call_tool("veritas_compile_intent", serde_json::json!({
            "input": safe_truncate(text, 500),
        })).await.ok()?;
        let extracted = extract_text(&result);
        if extracted.is_empty() { None } else { Some(extracted) }
    }

    /// Detect ambiguities in intent text and list disambiguation options.
    pub async fn veritas_detect_ambiguity(&self, input: &str) -> Option<String> {
        let veritas = self.veritas.as_ref()?;
        let result = veritas.call_tool("veritas_detect_ambiguity", serde_json::json!({
            "input": safe_truncate(input, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Ground a claim against known facts and evidence.
    pub async fn veritas_ground(&self, claim: &str) -> Option<String> {
        let veritas = self.veritas.as_ref()?;
        let result = veritas.call_tool("veritas_verify_claim", serde_json::json!({
            "claim": safe_truncate(claim, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Gather evidence for a query using claim extraction and synthesis.
    pub async fn veritas_evidence(&self, query: &str) -> Option<String> {
        let veritas = self.veritas.as_ref()?;
        let result = veritas.call_tool("veritas_synthesize", serde_json::json!({
            "text": safe_truncate(query, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Suggest improvements or clarifications given a context.
    pub async fn veritas_suggest(&self, context: &str) -> Option<String> {
        let veritas = self.veritas.as_ref()?;
        let result = veritas.call_tool("veritas_generate_question", serde_json::json!({
            "input": safe_truncate(context, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ── Aegis Extended Tools ──

    /// Check input for safety before processing.
    pub async fn aegis_check_input(&self, input: &str) -> Option<String> {
        let aegis = self.aegis.as_ref()?;
        let result = aegis.call_tool("aegis_check_input", serde_json::json!({
            "input": safe_truncate(input, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Check output for safety before delivery.
    pub async fn aegis_check_output(&self, output: &str) -> Option<String> {
        let aegis = self.aegis.as_ref()?;
        let result = aegis.call_tool("aegis_check_output", serde_json::json!({
            "output": safe_truncate(output, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Score confidence in generated code.
    pub async fn aegis_confidence_score(&self, code: &str) -> Option<f64> {
        let aegis = self.aegis.as_ref()?;
        let result = aegis.call_tool("aegis_confidence_score", serde_json::json!({
            "code": safe_truncate(code, 1000),
        })).await.ok()?;
        result.get("confidence").and_then(|v| v.as_f64())
    }

    /// Get correction hints for problematic code.
    pub async fn aegis_correction_hint(&self, code: &str, error: &str) -> Option<String> {
        let aegis = self.aegis.as_ref()?;
        let result = aegis.call_tool("aegis_correction_hint", serde_json::json!({
            "code": safe_truncate(code, 500),
            "error": safe_truncate(error, 300),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Request rollback of a failed action.
    pub async fn aegis_rollback(&self, action_id: &str, reason: &str) -> Option<String> {
        let aegis = self.aegis.as_ref()?;
        let result = aegis.call_tool("aegis_rollback", serde_json::json!({
            "action_id": action_id,
            "reason": safe_truncate(reason, 200),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Validate code completeness.
    pub async fn aegis_validate_complete(&self, code: &str) -> Option<String> {
        let aegis = self.aegis.as_ref()?;
        let result = aegis.call_tool("aegis_validate_complete", serde_json::json!({
            "code": safe_truncate(code, 1000),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Scan code for security vulnerabilities.
    pub async fn aegis_scan_security(&self, code: &str) -> Option<String> {
        let aegis = self.aegis.as_ref()?;
        let result = aegis.call_tool("aegis_scan_security", serde_json::json!({
            "code": safe_truncate(code, 1000),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Validate streaming response chunks.
    pub async fn aegis_validate_streaming(&self, chunk: &str) -> Option<bool> {
        let aegis = self.aegis.as_ref()?;
        let result = aegis.call_tool("aegis_validate_streaming", serde_json::json!({
            "chunk": safe_truncate(chunk, 500),
        })).await.ok()?;
        result.get("safe").and_then(|v| v.as_bool())
    }

    /// Get Aegis session status.
    pub async fn aegis_session_status(&self) -> Option<String> {
        let aegis = self.aegis.as_ref()?;
        let result = aegis.call_tool("aegis_session_status", serde_json::json!({})).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // aegis_shadow_execute already defined in aegis_deep.rs
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_veritas_aegis_deep_compiles() {
        assert!(true);
    }
}
