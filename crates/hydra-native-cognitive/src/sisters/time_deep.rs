//! Phase G Priority 6: Time Full Temporal — scheduling, sequences, decay, workspace.
//!
//! Extends extras_deep.rs time tools with scheduling, sequence management,
//! decay policies, and duration estimation.

use super::connection::extract_text;
use super::cognitive::Sisters;
use hydra_native_state::utils::safe_truncate;

impl Sisters {
    // ── Scheduling (5 tools) ──

    /// Create a scheduled event.
    pub async fn time_schedule_create(&self, title: &str, when: &str) -> Option<String> {
        let time = self.time.as_ref()?;
        let result = time.call_tool("time_schedule_create", serde_json::json!({
            "title": title, "when": when,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Check schedule availability for a time range.
    pub async fn time_schedule_available(&self, start: &str, end: &str) -> Option<String> {
        let time = self.time.as_ref()?;
        let result = time.call_tool("time_schedule_available", serde_json::json!({
            "start": start, "end": end,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Find schedule conflicts for a proposed event.
    pub async fn time_schedule_conflicts(&self, when: &str) -> Option<String> {
        let time = self.time.as_ref()?;
        let result = time.call_tool("time_schedule_conflicts", serde_json::json!({
            "when": when,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Query a schedule range.
    pub async fn time_schedule_range(&self, start: &str, end: &str) -> Option<String> {
        let time = self.time.as_ref()?;
        let result = time.call_tool("time_schedule_range", serde_json::json!({
            "start": start, "end": end,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Reschedule an existing event.
    pub async fn time_schedule_reschedule(&self, event_id: &str, new_when: &str) -> Option<String> {
        let time = self.time.as_ref()?;
        let result = time.call_tool("time_schedule_reschedule", serde_json::json!({
            "event_id": event_id, "new_when": new_when,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ── Sequences (3 tools) ──

    /// Create a task sequence (ordered steps).
    pub async fn time_sequence_create(&self, name: &str, steps: &[String]) -> Option<String> {
        let time = self.time.as_ref()?;
        let result = time.call_tool("time_sequence_create", serde_json::json!({
            "name": name, "steps": steps,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Advance a sequence to the next step.
    pub async fn time_sequence_advance(&self, sequence_id: &str) -> Option<String> {
        let time = self.time.as_ref()?;
        let result = time.call_tool("time_sequence_advance", serde_json::json!({
            "sequence_id": sequence_id,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Get sequence status — current step, progress.
    pub async fn time_sequence_status(&self, sequence_id: &str) -> Option<String> {
        let time = self.time.as_ref()?;
        let result = time.call_tool("time_sequence_status", serde_json::json!({
            "sequence_id": sequence_id,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ── Decay Policies (3 tools) ──

    /// Create a decay policy for knowledge freshness.
    pub async fn time_decay_create(&self, topic: &str, half_life_days: u32) {
        if let Some(time) = &self.time {
            let _ = time.call_tool("time_decay_create", serde_json::json!({
                "topic": topic, "half_life_days": half_life_days,
            })).await;
        }
    }

    /// Check decay value for a topic — how fresh is it?
    pub async fn time_decay_value(&self, topic: &str) -> Option<f64> {
        let time = self.time.as_ref()?;
        let result = time.call_tool("time_decay_value", serde_json::json!({
            "topic": topic,
        })).await.ok()?;
        result.get("freshness").and_then(|v| v.as_f64())
    }

    /// Get decay alerts — topics that need refreshing.
    pub async fn time_decay_alert(&self) -> Option<String> {
        let time = self.time.as_ref()?;
        let result = time.call_tool("time_decay_alert", serde_json::json!({})).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ── Duration & Evidence ──

    /// Estimate how long a task will take.
    pub async fn time_duration_estimate(&self, task: &str) -> Option<String> {
        let time = self.time.as_ref()?;
        let result = time.call_tool("time_duration_estimate", serde_json::json!({
            "task": safe_truncate(task, 200),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Aggregate duration across tasks.
    pub async fn time_duration_aggregate(&self) -> Option<String> {
        let time = self.time.as_ref()?;
        let result = time.call_tool("time_duration_aggregate", serde_json::json!({
            "window": "current_session",
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Add deadline for a task.
    pub async fn time_deadline_add(&self, task: &str, deadline: &str) -> Option<String> {
        let time = self.time.as_ref()?;
        let result = time.call_tool("time_deadline_add", serde_json::json!({
            "task": task, "deadline": deadline,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Mark a deadline as completed.
    pub async fn time_deadline_complete(&self, task: &str) {
        if let Some(time) = &self.time {
            let _ = time.call_tool("time_deadline_complete", serde_json::json!({
                "task": task,
            })).await;
        }
    }

    /// Check overdue deadlines.
    pub async fn time_deadline_overdue(&self) -> Option<String> {
        let time = self.time.as_ref()?;
        let result = time.call_tool("time_deadline_overdue", serde_json::json!({})).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Get temporal evidence for claims.
    pub async fn time_evidence(&self, claim: &str) -> Option<String> {
        let time = self.time.as_ref()?;
        let result = time.call_tool("time_evidence", serde_json::json!({
            "claim": safe_truncate(claim, 200),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Ground time references ("yesterday", "last week").
    pub async fn time_ground(&self, reference: &str) -> Option<String> {
        let time = self.time.as_ref()?;
        let result = time.call_tool("time_ground", serde_json::json!({
            "reference": reference,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Refresh time context.
    pub async fn time_refresh(&self) -> Option<String> {
        let time = self.time.as_ref()?;
        let result = time.call_tool("time_refresh", serde_json::json!({})).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Get time suggestions.
    pub async fn time_suggest(&self, context: &str) -> Option<String> {
        let time = self.time.as_ref()?;
        let result = time.call_tool("time_suggest", serde_json::json!({
            "context": safe_truncate(context, 200),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Resume time session context.
    pub async fn time_session_resume(&self) -> Option<String> {
        let time = self.time.as_ref()?;
        let result = time.call_tool("time_session_resume", serde_json::json!({})).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_time_deep_compiles() {
        assert!(true);
    }
}
