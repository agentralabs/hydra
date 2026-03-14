//! Codebase Deep — CORE Codebase sister MCP tool wiring.
//!
//! Exposes symbol lookup, impact analysis, workspace management,
//! semantic search, grounding, architecture, patterns, citations,
//! hallucination checking, and truth tracking.

use super::connection::extract_text;
use super::cognitive::Sisters;
use hydra_native_state::utils::safe_truncate;

impl Sisters {
    // ═══════════════════════════════════════════════════════════════
    // Core Tools
    // ═══════════════════════════════════════════════════════════════

    /// Look up a symbol by name across the codebase graph.
    pub async fn codebase_symbol_lookup(&self, name: &str) -> Option<String> {
        let cb = self.codebase.as_ref()?;
        let r = cb.call_tool("symbol_lookup", serde_json::json!({
            "name": safe_truncate(name, 500),
        })).await.ok()?;
        let t = extract_text(&r);
        if t.is_empty() { None } else { Some(t) }
    }

    /// Analyze the impact of changing a code unit.
    pub async fn codebase_impact_analysis(&self, unit: &str) -> Option<String> {
        let cb = self.codebase.as_ref()?;
        let r = cb.call_tool("impact_analysis", serde_json::json!({
            "unit": safe_truncate(unit, 500),
        })).await.ok()?;
        let t = extract_text(&r);
        if t.is_empty() { None } else { Some(t) }
    }

    /// Get codebase graph statistics.
    pub async fn codebase_graph_stats(&self) -> Option<String> {
        let cb = self.codebase.as_ref()?;
        let r = cb.call_tool("graph_stats", serde_json::json!({})).await.ok()?;
        let t = extract_text(&r);
        if t.is_empty() { None } else { Some(t) }
    }

    /// List code units, optionally filtered by type.
    pub async fn codebase_list_units(&self, unit_type: Option<&str>) -> Option<String> {
        let cb = self.codebase.as_ref()?;
        let r = cb.call_tool("list_units", serde_json::json!({
            "unit_type": unit_type,
        })).await.ok()?;
        let t = extract_text(&r);
        if t.is_empty() { None } else { Some(t) }
    }

    /// Log an analysis event (fire-and-forget).
    pub async fn codebase_analysis_log(&self, intent: &str, context: &str) {
        let Some(cb) = self.codebase.as_ref() else { return };
        if let Err(e) = cb.call_tool("analysis_log", serde_json::json!({
            "intent": safe_truncate(intent, 500),
            "context": safe_truncate(context, 500),
        })).await {
            eprintln!("[hydra:codebase] analysis_log FAILED: {}", e);
        }
    }

    /// Start a codebase analysis session.
    pub async fn codebase_session_start(&self, project: &str) -> Option<String> {
        let cb = self.codebase.as_ref()?;
        let r = cb.call_tool("session_start", serde_json::json!({
            "project": safe_truncate(project, 500),
        })).await.ok()?;
        let t = extract_text(&r);
        if t.is_empty() { None } else { Some(t) }
    }

    /// End the current codebase session (fire-and-forget).
    pub async fn codebase_session_end(&self) {
        let Some(cb) = self.codebase.as_ref() else { return };
        if let Err(e) = cb.call_tool("session_end", serde_json::json!({})).await {
            eprintln!("[hydra:codebase] session_end FAILED: {}", e);
        }
    }

    /// Resume a previous codebase session.
    pub async fn codebase_session_resume(&self) -> Option<String> {
        let cb = self.codebase.as_ref()?;
        let r = cb.call_tool("codebase_session_resume", serde_json::json!({})).await.ok()?;
        let t = extract_text(&r);
        if t.is_empty() { None } else { Some(t) }
    }

    // ═══════════════════════════════════════════════════════════════
    // Workspace Tools
    // ═══════════════════════════════════════════════════════════════

    /// Create a new workspace.
    pub async fn codebase_workspace_create(&self, name: &str) -> Option<String> {
        let cb = self.codebase.as_ref()?;
        let r = cb.call_tool("workspace_create", serde_json::json!({
            "name": safe_truncate(name, 500),
        })).await.ok()?;
        let t = extract_text(&r);
        if t.is_empty() { None } else { Some(t) }
    }

    /// Add a path to a workspace.
    pub async fn codebase_workspace_add(&self, workspace: &str, path: &str) -> Option<String> {
        let cb = self.codebase.as_ref()?;
        let r = cb.call_tool("workspace_add", serde_json::json!({
            "workspace": safe_truncate(workspace, 500),
            "path": safe_truncate(path, 500),
        })).await.ok()?;
        let t = extract_text(&r);
        if t.is_empty() { None } else { Some(t) }
    }

    /// List contents of a workspace.
    pub async fn codebase_workspace_list(&self, workspace: &str) -> Option<String> {
        let cb = self.codebase.as_ref()?;
        let r = cb.call_tool("workspace_list", serde_json::json!({
            "workspace": safe_truncate(workspace, 500),
        })).await.ok()?;
        let t = extract_text(&r);
        if t.is_empty() { None } else { Some(t) }
    }

    /// Query a workspace with a natural-language question.
    pub async fn codebase_workspace_query(&self, workspace: &str, query: &str) -> Option<String> {
        let cb = self.codebase.as_ref()?;
        let r = cb.call_tool("workspace_query", serde_json::json!({
            "workspace": safe_truncate(workspace, 500),
            "query": safe_truncate(query, 500),
        })).await.ok()?;
        let t = extract_text(&r);
        if t.is_empty() { None } else { Some(t) }
    }

    /// Compare a symbol across workspace repos.
    pub async fn codebase_workspace_compare(&self, workspace: &str, symbol: &str) -> Option<String> {
        let cb = self.codebase.as_ref()?;
        let r = cb.call_tool("workspace_compare", serde_json::json!({
            "workspace": safe_truncate(workspace, 500),
            "symbol": safe_truncate(symbol, 500),
        })).await.ok()?;
        let t = extract_text(&r);
        if t.is_empty() { None } else { Some(t) }
    }

    /// Cross-reference a symbol across workspace repos.
    pub async fn codebase_workspace_xref(&self, workspace: &str, symbol: &str) -> Option<String> {
        let cb = self.codebase.as_ref()?;
        let r = cb.call_tool("workspace_xref", serde_json::json!({
            "workspace": safe_truncate(workspace, 500),
            "symbol": safe_truncate(symbol, 500),
        })).await.ok()?;
        let t = extract_text(&r);
        if t.is_empty() { None } else { Some(t) }
    }

    // ═══════════════════════════════════════════════════════════════
    // Search Tools
    // ═══════════════════════════════════════════════════════════════

    /// Semantic search across the codebase.
    pub async fn codebase_search_semantic(&self, query: &str) -> Option<String> {
        let cb = self.codebase.as_ref()?;
        let r = cb.call_tool("search_semantic", serde_json::json!({
            "query": safe_truncate(query, 500),
        })).await.ok()?;
        let t = extract_text(&r);
        if t.is_empty() { None } else { Some(t) }
    }

    /// Find code units similar to the given unit.
    pub async fn codebase_search_similar(&self, unit: &str) -> Option<String> {
        let cb = self.codebase.as_ref()?;
        let r = cb.call_tool("search_similar", serde_json::json!({
            "unit": safe_truncate(unit, 500),
        })).await.ok()?;
        let t = extract_text(&r);
        if t.is_empty() { None } else { Some(t) }
    }

    /// Explain a code unit in the context of a query.
    pub async fn codebase_search_explain(&self, unit: &str, query: &str) -> Option<String> {
        let cb = self.codebase.as_ref()?;
        let r = cb.call_tool("search_explain", serde_json::json!({
            "unit": safe_truncate(unit, 500),
            "query": safe_truncate(query, 500),
        })).await.ok()?;
        let t = extract_text(&r);
        if t.is_empty() { None } else { Some(t) }
    }

    // ═══════════════════════════════════════════════════════════════
    // Grounding Tools
    // ═══════════════════════════════════════════════════════════════

    /// Ground a claim against actual codebase evidence.
    pub async fn codebase_ground(&self, claim: &str) -> Option<String> {
        let cb = self.codebase.as_ref()?;
        let r = cb.call_tool("codebase_ground", serde_json::json!({
            "claim": safe_truncate(claim, 1000),
        })).await.ok()?;
        let t = extract_text(&r);
        if t.is_empty() { None } else { Some(t) }
    }

    /// Retrieve evidence for a symbol.
    pub async fn codebase_evidence(&self, symbol: &str) -> Option<String> {
        let cb = self.codebase.as_ref()?;
        let r = cb.call_tool("codebase_evidence", serde_json::json!({
            "symbol": safe_truncate(symbol, 500),
        })).await.ok()?;
        let t = extract_text(&r);
        if t.is_empty() { None } else { Some(t) }
    }

    /// Suggest corrections for a symbol name.
    pub async fn codebase_suggest(&self, name: &str) -> Option<String> {
        let cb = self.codebase.as_ref()?;
        let r = cb.call_tool("codebase_suggest", serde_json::json!({
            "name": safe_truncate(name, 500),
        })).await.ok()?;
        let t = extract_text(&r);
        if t.is_empty() { None } else { Some(t) }
    }

    // ═══════════════════════════════════════════════════════════════
    // Architecture Tools
    // ═══════════════════════════════════════════════════════════════

    /// Infer codebase architecture from the code graph.
    pub async fn codebase_architecture_infer(&self) -> Option<String> {
        let cb = self.codebase.as_ref()?;
        let r = cb.call_tool("architecture_infer", serde_json::json!({})).await.ok()?;
        let t = extract_text(&r);
        if t.is_empty() { None } else { Some(t) }
    }

    /// Validate codebase architecture against inferred rules.
    pub async fn codebase_architecture_validate(&self) -> Option<String> {
        let cb = self.codebase.as_ref()?;
        let r = cb.call_tool("architecture_validate", serde_json::json!({})).await.ok()?;
        let t = extract_text(&r);
        if t.is_empty() { None } else { Some(t) }
    }

    // ═══════════════════════════════════════════════════════════════
    // Pattern Tools
    // ═══════════════════════════════════════════════════════════════

    /// Extract patterns from the codebase.
    pub async fn codebase_pattern_extract(&self) -> Option<String> {
        let cb = self.codebase.as_ref()?;
        let r = cb.call_tool("pattern_extract", serde_json::json!({})).await.ok()?;
        let t = extract_text(&r);
        if t.is_empty() { None } else { Some(t) }
    }

    /// Check a code unit against known patterns.
    pub async fn codebase_pattern_check(&self, unit: &str) -> Option<String> {
        let cb = self.codebase.as_ref()?;
        let r = cb.call_tool("pattern_check", serde_json::json!({
            "unit": safe_truncate(unit, 500),
        })).await.ok()?;
        let t = extract_text(&r);
        if t.is_empty() { None } else { Some(t) }
    }

    /// Suggest patterns for a file.
    pub async fn codebase_pattern_suggest(&self, file: &str) -> Option<String> {
        let cb = self.codebase.as_ref()?;
        let r = cb.call_tool("pattern_suggest", serde_json::json!({
            "file": safe_truncate(file, 500),
        })).await.ok()?;
        let t = extract_text(&r);
        if t.is_empty() { None } else { Some(t) }
    }

    // ═══════════════════════════════════════════════════════════════
    // Citation Tools
    // ═══════════════════════════════════════════════════════════════

    /// Ground a claim with codebase citations.
    pub async fn codebase_ground_claim(&self, claim: &str) -> Option<String> {
        let cb = self.codebase.as_ref()?;
        let r = cb.call_tool("codebase_ground_claim", serde_json::json!({
            "claim": safe_truncate(claim, 1000),
        })).await.ok()?;
        let t = extract_text(&r);
        if t.is_empty() { None } else { Some(t) }
    }

    /// Cite a code unit with source references.
    pub async fn codebase_cite(&self, unit: &str) -> Option<String> {
        let cb = self.codebase.as_ref()?;
        let r = cb.call_tool("codebase_cite", serde_json::json!({
            "unit": safe_truncate(unit, 500),
        })).await.ok()?;
        let t = extract_text(&r);
        if t.is_empty() { None } else { Some(t) }
    }

    // ═══════════════════════════════════════════════════════════════
    // Hallucination + Truth Tools
    // ═══════════════════════════════════════════════════════════════

    /// Check LLM output for hallucinated code references.
    pub async fn codebase_hallucination_check(&self, output: &str) -> Option<String> {
        let cb = self.codebase.as_ref()?;
        let r = cb.call_tool("hallucination_check", serde_json::json!({
            "output": safe_truncate(output, 1000),
        })).await.ok()?;
        let t = extract_text(&r);
        if t.is_empty() { None } else { Some(t) }
    }

    /// Register a verified claim as ground truth.
    pub async fn codebase_truth_register(&self, claim: &str) -> Option<String> {
        let cb = self.codebase.as_ref()?;
        let r = cb.call_tool("truth_register", serde_json::json!({
            "claim": safe_truncate(claim, 1000),
        })).await.ok()?;
        let t = extract_text(&r);
        if t.is_empty() { None } else { Some(t) }
    }

    /// Check a claim against registered ground truths.
    pub async fn codebase_truth_check(&self, claim: &str) -> Option<String> {
        let cb = self.codebase.as_ref()?;
        let r = cb.call_tool("truth_check", serde_json::json!({
            "claim": safe_truncate(claim, 1000),
        })).await.ok()?;
        let t = extract_text(&r);
        if t.is_empty() { None } else { Some(t) }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn codebase_deep_compiles() {
        // Compile-time verification that this module is valid.
        assert!(true);
    }
}
