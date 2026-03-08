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
