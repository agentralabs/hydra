//! Time Exploration — temporal replay, timeline forking, temporal cloning, echo detection.

use super::connection::extract_text;
use super::cognitive::Sisters;
use hydra_native_state::utils::safe_truncate;

impl Sisters {
    // ── Timeline Forking (4 tools) ──

    /// Fork the current timeline for speculative exploration.
    pub async fn time_timeline_fork(&self, reason: &str) -> Option<String> {
        let time = self.time.as_ref()?;
        let result = time.call_tool("time_timeline_fork", serde_json::json!({
            "reason": safe_truncate(reason, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Merge a forked timeline back into the main timeline.
    pub async fn time_timeline_merge(&self, fork_id: &str) -> Option<String> {
        let time = self.time.as_ref()?;
        let result = time.call_tool("time_timeline_merge", serde_json::json!({
            "fork_id": safe_truncate(fork_id, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Compare two timeline forks to see divergences.
    pub async fn time_timeline_compare(
        &self,
        fork_a: &str,
        fork_b: &str,
    ) -> Option<String> {
        let time = self.time.as_ref()?;
        let result = time.call_tool("time_timeline_compare", serde_json::json!({
            "fork_a": safe_truncate(fork_a, 500),
            "fork_b": safe_truncate(fork_b, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Visualize the current timeline (optionally within a range).
    pub async fn time_timeline_visualize(&self, range: &str) -> Option<String> {
        let time = self.time.as_ref()?;
        let result = time.call_tool("time_timeline_visualize", serde_json::json!({
            "range": safe_truncate(range, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ── Temporal Cloning (4 tools) ──

    /// Create a temporal clone for parallel task execution.
    pub async fn time_clone_create(&self, task: &str) -> Option<String> {
        let time = self.time.as_ref()?;
        let result = time.call_tool("time_clone_create", serde_json::json!({
            "task": safe_truncate(task, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Race multiple clones and pick the winner.
    pub async fn time_clone_race(&self, clone_ids: &str) -> Option<String> {
        let time = self.time.as_ref()?;
        let result = time.call_tool("time_clone_race", serde_json::json!({
            "clone_ids": safe_truncate(clone_ids, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Merge a temporal clone's results back.
    pub async fn time_clone_merge(&self, clone_id: &str) -> Option<String> {
        let time = self.time.as_ref()?;
        let result = time.call_tool("time_clone_merge", serde_json::json!({
            "clone_id": safe_truncate(clone_id, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Check status of a temporal clone.
    pub async fn time_clone_status(&self, clone_id: &str) -> Option<String> {
        let time = self.time.as_ref()?;
        let result = time.call_tool("time_clone_status", serde_json::json!({
            "clone_id": safe_truncate(clone_id, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ── Echo Detection (3 tools) ──

    /// Detect temporal echoes matching a pattern.
    pub async fn time_echo_detect(&self, pattern: &str) -> Option<String> {
        let time = self.time.as_ref()?;
        let result = time.call_tool("time_echo_detect", serde_json::json!({
            "pattern": safe_truncate(pattern, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Amplify a detected temporal echo for stronger recall.
    pub async fn time_echo_amplify(&self, echo_id: &str) -> Option<String> {
        let time = self.time.as_ref()?;
        let result = time.call_tool("time_echo_amplify", serde_json::json!({
            "echo_id": safe_truncate(echo_id, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Suppress a temporal echo to reduce noise.
    pub async fn time_echo_suppress(&self, echo_id: &str) -> Option<String> {
        let time = self.time.as_ref()?;
        let result = time.call_tool("time_echo_suppress", serde_json::json!({
            "echo_id": safe_truncate(echo_id, 500),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_time_exploration_compiles() {
        assert!(true);
    }
}
