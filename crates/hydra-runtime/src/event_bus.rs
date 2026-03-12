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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sse::SseEventType;

    #[test]
    fn test_new_event_bus() {
        let bus = EventBus::new(16);
        assert_eq!(bus.total_published(), 0);
    }

    #[test]
    fn test_default_event_bus() {
        let bus = EventBus::default();
        assert_eq!(bus.total_published(), 0);
    }

    #[test]
    fn test_publish_increments_counter() {
        let bus = EventBus::new(16);
        bus.publish(SseEvent::heartbeat());
        assert_eq!(bus.total_published(), 1);
        bus.publish(SseEvent::heartbeat());
        bus.publish(SseEvent::heartbeat());
        assert_eq!(bus.total_published(), 3);
    }

    #[test]
    fn test_subscribe_receives_events() {
        let bus = EventBus::new(16);
        let mut rx = bus.subscribe();
        bus.publish(SseEvent::heartbeat());
        let event = rx.try_recv().unwrap();
        assert_eq!(event.event_type, SseEventType::Heartbeat);
    }

    #[test]
    fn test_multiple_subscribers() {
        let bus = EventBus::new(16);
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();
        bus.publish(SseEvent::system_ready("1.0"));
        assert!(rx1.try_recv().is_ok());
        assert!(rx2.try_recv().is_ok());
    }

    #[test]
    fn test_publish_system_events() {
        let bus = EventBus::new(16);
        let mut rx = bus.subscribe();
        bus.publish(SseEvent::system_shutdown("test"));
        let event = rx.try_recv().unwrap();
        assert_eq!(event.event_type, SseEventType::SystemShutdown);
    }
}
