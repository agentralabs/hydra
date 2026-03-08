use std::convert::Infallible;
use std::sync::Arc;

use axum::response::sse::Event;
use futures::stream::{self, Stream, StreamExt};
use hydra_runtime::sse::SseEvent;

use crate::state::AppState;

/// Create an SSE stream from the event bus
pub fn sse_stream(state: Arc<AppState>) -> impl Stream<Item = Result<Event, Infallible>> {
    let rx = state.event_bus.subscribe();

    // Initial connected event
    let initial = stream::once(async {
        Ok(Event::default()
            .event("system_ready")
            .data(serde_json::json!({"version": "0.1.0"}).to_string()))
    });

    let events = stream::unfold(rx, |mut rx| async move {
        match rx.recv().await {
            Ok(sse_event) => {
                let event = format_sse_event(&sse_event);
                Some((Ok(event), rx))
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!("SSE client lagged, skipped {n} events");
                let event = Event::default()
                    .event("system_ready")
                    .data(serde_json::json!({"warning": "reconnected after lag"}).to_string());
                Some((Ok(event), rx))
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => None,
        }
    });

    initial.chain(events)
}

fn format_sse_event(event: &SseEvent) -> Event {
    let event_name = serde_json::to_string(&event.event_type)
        .unwrap_or_else(|_| "\"unknown\"".into())
        .trim_matches('"')
        .to_string();

    let data = serde_json::to_string(&event.data).unwrap_or_default();

    Event::default().event(event_name).data(data)
}
