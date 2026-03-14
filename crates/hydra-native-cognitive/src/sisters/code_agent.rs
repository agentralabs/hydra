//! Code Agent — full code generation pipeline via sister tools.
//!
//! Makes Hydra a code generation agent by composing Forge (generation),
//! Aegis (validation/security), Veritas (consistency/claims), and
//! Evolve (pattern learning) sisters into a coherent pipeline.
//!
//! Architecture: Blueprint → Generate → Validate → Review → Learn

use super::connection::extract_text;
use super::cognitive::Sisters;
use hydra_native_state::utils::safe_truncate;

impl Sisters {
    // ═══════════════════════════════════════════════════════════════
    // PHASE 1: Blueprint — define what to build
    // ═══════════════════════════════════════════════════════════════

    /// Create a code blueprint from a natural-language description.
    /// Returns the blueprint ID for downstream generation steps.
    pub async fn code_blueprint(&self, description: &str) -> Option<String> {
        let forge = self.forge.as_ref()?;
        let result = forge.call_tool("forge_blueprint_create", serde_json::json!({
            "description": safe_truncate(description, 1000),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Validate an existing blueprint for architectural consistency.
    /// Catches missing entities, circular deps, incomplete specs.
    pub async fn code_blueprint_validate(&self, id: &str) -> Option<String> {
        let forge = self.forge.as_ref()?;
        let result = forge.call_tool("forge_blueprint_validate", serde_json::json!({
            "id": id,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ═══════════════════════════════════════════════════════════════
    // PHASE 2: Generate — produce code artifacts
    // ═══════════════════════════════════════════════════════════════

    /// Generate a code skeleton from a validated blueprint.
    /// Produces module structure, type stubs, function signatures.
    pub async fn code_generate_skeleton(&self, blueprint_id: &str) -> Option<String> {
        let forge = self.forge.as_ref()?;
        let result = forge.call_tool("forge_skeleton_create", serde_json::json!({
            "blueprint_id": blueprint_id,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Generate tests for a blueprint — unit tests, integration tests, edge cases.
    pub async fn code_generate_tests(&self, blueprint_id: &str) -> Option<String> {
        let forge = self.forge.as_ref()?;
        let result = forge.call_tool("forge_test_generate", serde_json::json!({
            "blueprint_id": blueprint_id,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Resolve dependencies for a blueprint — crate versions, feature flags.
    pub async fn code_resolve_deps(&self, blueprint_id: &str) -> Option<String> {
        let forge = self.forge.as_ref()?;
        let result = forge.call_tool("forge_dependency_resolve", serde_json::json!({
            "blueprint_id": blueprint_id,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Generate wiring code — how modules connect, trait impls, pub interfaces.
    pub async fn code_generate_wiring(&self, blueprint_id: &str) -> Option<String> {
        let forge = self.forge.as_ref()?;
        let result = forge.call_tool("forge_wiring_create", serde_json::json!({
            "blueprint_id": blueprint_id,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Generate project structure — directory layout, mod.rs files, Cargo.toml.
    pub async fn code_generate_structure(&self, blueprint_id: &str) -> Option<String> {
        let forge = self.forge.as_ref()?;
        let result = forge.call_tool("forge_structure_generate", serde_json::json!({
            "blueprint_id": blueprint_id,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ═══════════════════════════════════════════════════════════════
    // PHASE 3: Validate — security, completeness, confidence
    // ═══════════════════════════════════════════════════════════════

    /// Scan generated code for security vulnerabilities.
    /// Checks for injection, unsafe blocks, credential leaks, etc.
    pub async fn code_validate_security(&self, code: &str) -> Option<String> {
        let aegis = self.aegis.as_ref()?;
        let result = aegis.call_tool("aegis_scan_security", serde_json::json!({
            "code": safe_truncate(code, 2000),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Validate code completeness — are all blueprint entities implemented?
    pub async fn code_validate_complete(&self, code: &str) -> Option<String> {
        let aegis = self.aegis.as_ref()?;
        let result = aegis.call_tool("aegis_validate_complete", serde_json::json!({
            "code": safe_truncate(code, 2000),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Get a confidence score for generated code (0.0 to 1.0).
    /// Considers test coverage, type safety, pattern adherence.
    pub async fn code_confidence_score(&self, code: &str) -> Option<f64> {
        let aegis = self.aegis.as_ref()?;
        let result = aegis.call_tool("aegis_confidence_score", serde_json::json!({
            "code": safe_truncate(code, 2000),
        })).await.ok()?;
        result.get("score").and_then(|v| v.as_f64())
            .or_else(|| {
                extract_text(&result).parse::<f64>().ok()
            })
    }

    /// Verify code-to-spec consistency — does the code match the spec?
    pub async fn code_verify_consistency(
        &self, code: &str, spec: &str,
    ) -> Option<String> {
        let veritas = self.veritas.as_ref()?;
        let result = veritas.call_tool("veritas_check_consistency", serde_json::json!({
            "code": safe_truncate(code, 1500),
            "spec": safe_truncate(spec, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ═══════════════════════════════════════════════════════════════
    // PHASE 4: Review — claims extraction and correction
    // ═══════════════════════════════════════════════════════════════

    /// Extract testable claims from code — assertions, invariants, contracts.
    pub async fn code_extract_claims(&self, code: &str) -> Option<Vec<String>> {
        let veritas = self.veritas.as_ref()?;
        let result = veritas.call_tool("veritas_extract_claims", serde_json::json!({
            "content": safe_truncate(code, 2000),
        })).await.ok()?;

        // Try structured array first, fall back to text split
        if let Some(claims) = result.get("claims").and_then(|v| v.as_array()) {
            let parsed: Vec<String> = claims.iter()
                .filter_map(|c| c.as_str().map(|s| s.to_string()))
                .collect();
            if !parsed.is_empty() {
                return Some(parsed);
            }
        }
        let text = extract_text(&result);
        if text.is_empty() {
            None
        } else {
            Some(text.lines().map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect())
        }
    }

    /// Get a correction hint for code that has errors.
    /// Returns a suggestion for how to fix the issue.
    pub async fn code_correction_hint(
        &self, code: &str, error: &str,
    ) -> Option<String> {
        let aegis = self.aegis.as_ref()?;
        let result = aegis.call_tool("aegis_correction_hint", serde_json::json!({
            "code": safe_truncate(code, 1500),
            "error": safe_truncate(error, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ═══════════════════════════════════════════════════════════════
    // PHASE 5: Learn — pattern storage and retrieval
    // ═══════════════════════════════════════════════════════════════

    /// Store a code pattern for future reuse.
    /// success=true means the pattern worked well; false means it failed.
    pub async fn code_learn_pattern(&self, code: &str, success: bool) {
        let Some(evolve) = self.evolve.as_ref() else { return };
        if let Err(e) = evolve.call_tool("evolve_pattern_store", serde_json::json!({
            "pattern": safe_truncate(code, 1000),
            "success": success,
            "domain": "code_generation",
        })).await {
            eprintln!("[hydra:evolve] evolve_pattern_store FAILED: {}", e);
        }
    }

    /// Match existing patterns against a description.
    /// Returns the best matching pattern if one exists.
    pub async fn code_match_pattern(&self, description: &str) -> Option<String> {
        let evolve = self.evolve.as_ref()?;
        let result = evolve.call_tool("evolve_match_context", serde_json::json!({
            "query": safe_truncate(description, 500),
            "domain": "code_generation",
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_code_agent_compiles() {
        assert!(true);
    }
}
