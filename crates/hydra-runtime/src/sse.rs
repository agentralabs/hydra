use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Server-Sent Event for Hydra
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseEvent {
    pub event_type: SseEventType,
    pub data: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SseEventType {
    RunStarted,
    StepStarted,
    StepProgress,
    StepCompleted,
    ApprovalRequired,
    RunCompleted,
    RunError,
    /// Streamed text chunk from the LLM (for real-time display)
    StreamChunk,
    /// A cognitive phase has started (perceive, think, decide, act, learn)
    PhaseStarted,
    Heartbeat,
    SystemReady,
    SystemShutdown,
}

impl SseEvent {
    pub fn new(event_type: SseEventType, data: serde_json::Value) -> Self {
        Self {
            event_type,
            data,
            timestamp: Utc::now(),
        }
    }

    pub fn heartbeat() -> Self {
        Self::new(
            SseEventType::Heartbeat,
            serde_json::json!({"status": "alive"}),
        )
    }

    pub fn system_ready(version: &str) -> Self {
        Self::new(
            SseEventType::SystemReady,
            serde_json::json!({"version": version}),
        )
    }

    pub fn system_shutdown(reason: &str) -> Self {
        Self::new(
            SseEventType::SystemShutdown,
            serde_json::json!({"reason": reason}),
        )
    }

    /// Format as SSE wire format
    pub fn to_sse_string(&self) -> String {
        let event_name = serde_json::to_string(&self.event_type)
            .unwrap_or_else(|_| "\"unknown\"".into())
            .trim_matches('"')
            .to_string();
        let data = serde_json::to_string(&self.data).unwrap_or_default();
        format!("event: {event_name}\ndata: {data}\n\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heartbeat_event() {
        let event = SseEvent::heartbeat();
        assert_eq!(event.event_type, SseEventType::Heartbeat);
        assert_eq!(event.data["status"], "alive");
    }

    #[test]
    fn test_system_ready_event() {
        let event = SseEvent::system_ready("0.2.0");
        assert_eq!(event.event_type, SseEventType::SystemReady);
        assert_eq!(event.data["version"], "0.2.0");
    }

    #[test]
    fn test_system_shutdown_event() {
        let event = SseEvent::system_shutdown("user request");
        assert_eq!(event.event_type, SseEventType::SystemShutdown);
        assert_eq!(event.data["reason"], "user request");
    }

    #[test]
    fn test_to_sse_string_format() {
        let event = SseEvent::heartbeat();
        let sse = event.to_sse_string();
        assert!(sse.starts_with("event: "));
        assert!(sse.contains("data: "));
        assert!(sse.ends_with("\n\n"));
    }

    #[test]
    fn test_sse_event_serde() {
        let event = SseEvent::system_ready("1.0");
        let json = serde_json::to_string(&event).unwrap();
        let restored: SseEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.event_type, SseEventType::SystemReady);
    }

    #[test]
    fn test_event_type_serde() {
        for et in [
            SseEventType::RunStarted,
            SseEventType::StepStarted,
            SseEventType::StepProgress,
            SseEventType::StepCompleted,
            SseEventType::ApprovalRequired,
            SseEventType::RunCompleted,
            SseEventType::RunError,
            SseEventType::Heartbeat,
            SseEventType::SystemReady,
            SseEventType::SystemShutdown,
        ] {
            let json = serde_json::to_string(&et).unwrap();
            let restored: SseEventType = serde_json::from_str(&json).unwrap();
            assert_eq!(restored, et);
        }
    }

    #[test]
    fn test_new_custom_event() {
        let event = SseEvent::new(SseEventType::RunStarted, serde_json::json!({"run_id": "abc"}));
        assert_eq!(event.event_type, SseEventType::RunStarted);
        assert_eq!(event.data["run_id"], "abc");
    }

    #[test]
    fn test_timestamp_populated() {
        let event = SseEvent::heartbeat();
        // Timestamp should be recent (within last second)
        let now = chrono::Utc::now();
        let diff = now - event.timestamp;
        assert!(diff.num_seconds() < 2);
    }
}
