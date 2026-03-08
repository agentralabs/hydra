use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::broadcast;

use crate::sse::SseEvent;

/// In-process event bus for pub/sub
pub struct EventBus {
    tx: broadcast::Sender<SseEvent>,
    events_published: AtomicU64,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self {
            tx,
            events_published: AtomicU64::new(0),
        }
    }

    /// Publish an event to all subscribers
    pub fn publish(&self, event: SseEvent) {
        self.events_published.fetch_add(1, Ordering::Relaxed);
        let _ = self.tx.send(event);
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<SseEvent> {
        self.tx.subscribe()
    }

    /// Total events published
    pub fn total_published(&self) -> u64 {
        self.events_published.load(Ordering::Relaxed)
    }

    /// Spawn a heartbeat task that publishes heartbeat events every `interval`.
    /// Returns a JoinHandle that can be used to cancel the heartbeat.
    pub fn spawn_heartbeat(self: &Arc<Self>, interval: Duration) -> tokio::task::JoinHandle<()> {
        let bus = Arc::clone(self);
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            ticker.tick().await; // First tick is immediate, skip it
            loop {
                ticker.tick().await;
                bus.publish(SseEvent::heartbeat());
            }
        })
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(1024)
    }
}
