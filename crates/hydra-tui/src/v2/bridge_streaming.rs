//! Streaming bridge — manages the LLM streaming state machine.
//! Produces Actions for each streaming lifecycle event.

use crate::v2::action::{Action, StreamAction, StreamingAction};
use crate::stream_types::StreamItem;

/// Streaming state machine.
#[derive(Debug, Clone, PartialEq)]
pub enum StreamingState {
    Idle,
    Active {
        session_id: String,
        accumulated: String,
        started_at: std::time::Instant,
    },
}

impl Default for StreamingState {
    fn default() -> Self {
        Self::Idle
    }
}

impl StreamingState {
    /// Start a new streaming session.
    pub fn start(&mut self, session_id: String) {
        *self = Self::Active {
            session_id,
            accumulated: String::new(),
            started_at: std::time::Instant::now(),
        };
    }

    /// Append a chunk.
    pub fn append(&mut self, chunk: &str) {
        if let Self::Active { accumulated, .. } = self {
            accumulated.push_str(chunk);
        }
    }

    /// Finish streaming. Returns the full text and duration.
    pub fn finish(&mut self) -> Option<(String, u64)> {
        if let Self::Active { accumulated, started_at, .. } = self {
            let text = accumulated.clone();
            let duration_ms = started_at.elapsed().as_millis() as u64;
            *self = Self::Idle;
            Some((text, duration_ms))
        } else {
            None
        }
    }

    /// Cancel streaming.
    pub fn cancel(&mut self) {
        *self = Self::Idle;
    }

    pub fn is_active(&self) -> bool {
        matches!(self, Self::Active { .. })
    }

    pub fn accumulated_text(&self) -> &str {
        match self {
            Self::Active { accumulated, .. } => accumulated,
            Self::Idle => "",
        }
    }

    pub fn elapsed_ms(&self) -> u64 {
        match self {
            Self::Active { started_at, .. } => started_at.elapsed().as_millis() as u64,
            Self::Idle => 0,
        }
    }
}

/// Process a streaming action and produce stream mutations.
pub fn process_streaming_action(
    action: &StreamingAction,
    state: &mut StreamingState,
) -> Vec<Action> {
    let mut actions = Vec::new();

    match action {
        StreamingAction::Start { session_id } => {
            state.start(session_id.clone());
        }
        StreamingAction::Chunk(text) => {
            state.append(text);
            // Update the last AssistantText item in the stream
            actions.push(Action::Stream(StreamAction::PushItem(
                StreamItem::AssistantText {
                    id: uuid::Uuid::new_v4(),
                    text: state.accumulated_text().to_string(),
                    timestamp: chrono::Utc::now(),
                },
            )));
        }
        StreamingAction::Done { tokens: _, duration_ms } => {
            if let Some((_text, _dur)) = state.finish() {
                actions.push(Action::Stream(StreamAction::PushItem(
                    StreamItem::ThinkingPill {
                        duration_secs: *duration_ms as f64 / 1000.0,
                    },
                )));
            }
        }
        StreamingAction::Error(msg) => {
            state.cancel();
            actions.push(Action::Stream(StreamAction::PushItem(
                StreamItem::SystemNotification {
                    id: uuid::Uuid::new_v4(),
                    content: format!("Error: {msg}"),
                    timestamp: chrono::Utc::now(),
                },
            )));
        }
        StreamingAction::Interrupt => {
            state.cancel();
            actions.push(Action::Stream(StreamAction::PushItem(
                StreamItem::SystemNotification {
                    id: uuid::Uuid::new_v4(),
                    content: "Interrupted.".into(),
                    timestamp: chrono::Utc::now(),
                },
            )));
        }
    }

    actions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streaming_lifecycle() {
        let mut state = StreamingState::default();
        assert!(!state.is_active());

        state.start("test-session".into());
        assert!(state.is_active());

        state.append("Hello ");
        state.append("world");
        assert_eq!(state.accumulated_text(), "Hello world");

        let (text, _dur) = state.finish().unwrap();
        assert_eq!(text, "Hello world");
        assert!(!state.is_active());
    }

    #[test]
    fn cancel_resets() {
        let mut state = StreamingState::default();
        state.start("s".into());
        state.append("data");
        state.cancel();
        assert!(!state.is_active());
    }

    #[test]
    fn idle_accumulated_empty() {
        let state = StreamingState::default();
        assert_eq!(state.accumulated_text(), "");
    }
}
