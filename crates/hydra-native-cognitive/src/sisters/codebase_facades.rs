//! Codebase Facades — compact facade routers and standalone analysis tools.

use super::connection::extract_text;
use super::cognitive::Sisters;
use hydra_native_state::utils::safe_truncate;

/// Helper: call a facade tool with operation + params on codebase sister.
async fn cb_facade(
    sisters: &Sisters,
    tool: &str,
    operation: &str,
    params: &str,
) -> Option<String> {
    let s = sisters.codebase.as_ref()?;
    let pj: serde_json::Value =
        serde_json::from_str(params).unwrap_or(serde_json::json!({}));
    let result = s.call_tool(tool, serde_json::json!({
        "operation": operation, "params": pj,
    })).await.ok()?;
    let text = extract_text(&result);
    if text.is_empty() { None } else { Some(text) }
}

impl Sisters {
    // ═══════════════════════════════════════════════════════════════
    // FACADE ROUTERS — generic operation + params dispatch
    // ═══════════════════════════════════════════════════════════════

    /// Route an operation to the `codebase_core` facade tool.
    pub async fn codebase_core_facade(&self, operation: &str, params: &str) -> Option<String> {
        cb_facade(self, "codebase_core", operation, params).await
    }

    /// Route an operation to the `codebase_grounding` facade tool.
    pub async fn codebase_grounding_facade(
        &self, operation: &str, params: &str,
    ) -> Option<String> {
        cb_facade(self, "codebase_grounding", operation, params).await
    }

    /// Route an operation to the `codebase_patterns` facade tool.
    pub async fn codebase_patterns_facade(&self, operation: &str, params: &str) -> Option<String> {
        cb_facade(self, "codebase_patterns", operation, params).await
    }

    /// Route an operation to the `codebase_conceptual` facade tool.
    pub async fn codebase_conceptual_facade(
        &self, operation: &str, params: &str,
    ) -> Option<String> {
        cb_facade(self, "codebase_conceptual", operation, params).await
    }

    /// Route an operation to the `codebase_workspace` facade tool.
    pub async fn codebase_workspace_facade(
        &self, operation: &str, params: &str,
    ) -> Option<String> {
        cb_facade(self, "codebase_workspace", operation, params).await
    }

    /// Route an operation to the `codebase_session` facade tool.
    pub async fn codebase_session_facade(&self, operation: &str, params: &str) -> Option<String> {
        cb_facade(self, "codebase_session", operation, params).await
    }

    /// Route an operation to the `codebase_intelligence` facade tool.
    pub async fn codebase_intelligence_facade(
        &self, operation: &str, params: &str,
    ) -> Option<String> {
        cb_facade(self, "codebase_intelligence", operation, params).await
    }

    /// Route an operation to the `codebase_translation` facade tool.
    pub async fn codebase_translation_facade(
        &self, operation: &str, params: &str,
    ) -> Option<String> {
        cb_facade(self, "codebase_translation", operation, params).await
    }

    /// Route an operation to the `codebase_archaeology` facade tool.
    pub async fn codebase_archaeology_facade(
        &self, operation: &str, params: &str,
    ) -> Option<String> {
        cb_facade(self, "codebase_archaeology", operation, params).await
    }

    /// Route an operation to the `codebase_collective` facade tool.
    pub async fn codebase_collective_facade(
        &self, operation: &str, params: &str,
    ) -> Option<String> {
        cb_facade(self, "codebase_collective", operation, params).await
    }

    // ═══════════════════════════════════════════════════════════════
    // STANDALONE ANALYSIS TOOLS
    // ═══════════════════════════════════════════════════════════════

    /// Analyse a specific code unit by ID.
    pub async fn codebase_analyse_unit(&self, unit_id: i64) -> Option<String> {
        let s = self.codebase.as_ref()?;
        let r = s.call_tool("analyse_unit", serde_json::json!({
            "unit_id": unit_id,
        })).await.ok()?;
        let text = extract_text(&r);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Explain coupling between two code units.
    pub async fn codebase_explain_coupling(&self, unit_a: i64, unit_b: i64) -> Option<String> {
        let s = self.codebase.as_ref()?;
        let r = s.call_tool("explain_coupling", serde_json::json!({
            "unit_a": unit_a, "unit_b": unit_b,
        })).await.ok()?;
        let text = extract_text(&r);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Analyse the impact of a change on a code unit.
    pub async fn codebase_impact_analyze(
        &self, unit_id: i64, change_type: &str,
    ) -> Option<String> {
        let s = self.codebase.as_ref()?;
        let r = s.call_tool("impact_analyze", serde_json::json!({
            "unit_id": unit_id,
            "change_type": safe_truncate(change_type, 200),
        })).await.ok()?;
        let text = extract_text(&r);
        if text.is_empty() { None } else { Some(text) }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_codebase_facades_compiles() {
        assert!(true);
    }
}
