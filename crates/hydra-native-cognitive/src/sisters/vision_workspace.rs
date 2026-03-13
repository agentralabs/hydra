//! Vision sister core extras, workspace, and session tools.
//!
//! Observation log, similarity search, tracking, health, linking,
//! workspace management (create/add/list/query/compare/xref),
//! and session lifecycle (start/end/resume).

use super::connection::extract_text;
use super::cognitive::Sisters;
use hydra_native_state::utils::safe_truncate;

impl Sisters {
    // ── Core Extras (5 tools) ──

    /// Retrieve the observation log for a given context.
    pub async fn vision_observation_log(&self, context: &str) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("observation_log", serde_json::json!({
            "context": safe_truncate(context, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Find visually similar captures to a given capture ID.
    pub async fn vision_similar(&self, capture_id: &str) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_similar", serde_json::json!({
            "capture_id": safe_truncate(capture_id, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Track a URL for visual change detection.
    pub async fn vision_track(&self, url: &str) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_track", serde_json::json!({
            "url": safe_truncate(url, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Check vision sister health status.
    pub async fn vision_health(&self) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_health", serde_json::json!({})).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Link a capture to a target resource.
    pub async fn vision_link(&self, capture_id: &str, target: &str) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_link", serde_json::json!({
            "capture_id": safe_truncate(capture_id, 500),
            "target": safe_truncate(target, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ── Workspace Tools (6 tools) ──

    /// Create a new vision workspace.
    pub async fn vision_workspace_create(&self, name: &str) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_workspace_create", serde_json::json!({
            "name": safe_truncate(name, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Add a capture to a vision workspace.
    pub async fn vision_workspace_add(
        &self, workspace: &str, capture_id: &str,
    ) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_workspace_add", serde_json::json!({
            "workspace": safe_truncate(workspace, 500),
            "capture_id": safe_truncate(capture_id, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// List captures in a vision workspace.
    pub async fn vision_workspace_list(&self, workspace: &str) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_workspace_list", serde_json::json!({
            "workspace": safe_truncate(workspace, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Query a vision workspace with a search string.
    pub async fn vision_workspace_query(
        &self, workspace: &str, query: &str,
    ) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_workspace_query", serde_json::json!({
            "workspace": safe_truncate(workspace, 500),
            "query": safe_truncate(query, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Compare all captures in a vision workspace.
    pub async fn vision_workspace_compare(&self, workspace: &str) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_workspace_compare", serde_json::json!({
            "workspace": safe_truncate(workspace, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Cross-reference an element across a vision workspace.
    pub async fn vision_workspace_xref(
        &self, workspace: &str, element: &str,
    ) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_workspace_xref", serde_json::json!({
            "workspace": safe_truncate(workspace, 500),
            "element": safe_truncate(element, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ── Session Tools (3 tools) ──

    /// Start a vision observation session with context.
    pub async fn vision_session_start(&self, context: &str) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("session_start", serde_json::json!({
            "context": safe_truncate(context, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// End the current vision session (fire-and-forget).
    pub async fn vision_session_end(&self) {
        let Some(vision) = self.vision.as_ref() else { return };
        let _ = vision.call_tool("session_end", serde_json::json!({})).await;
    }

    /// Resume a previous vision session.
    pub async fn vision_session_resume(&self) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_session_resume", serde_json::json!({})).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ── Compare by ID ──

    /// Compare two captures by their numeric IDs.
    pub async fn vision_compare_ids(&self, id_a: i64, id_b: i64) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_compare", serde_json::json!({
            "id_a": id_a,
            "id_b": id_b,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_vision_workspace_compiles() {
        assert!(true);
    }
}
