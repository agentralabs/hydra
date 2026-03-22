//! Signal dispatch — drains queues and delivers signals to subscribers.
//!
//! The dispatch cycle pops signals from queues in priority order and
//! creates delivery receipts for each one.

use crate::queue::SignalQueues;
use crate::receipt::{DeliveryOutcome, DeliveryReceipt, DeliveryReceiptLog};
use crate::router::signal_topic;
use crate::subscription::SubscriptionRegistry;

/// Result of a single dispatch cycle.
#[derive(Debug)]
pub struct DispatchResult {
    /// Number of signals successfully dispatched.
    pub dispatched: usize,
    /// Number of signals with no matching subscribers (unrouted).
    pub unrouted: usize,
}

/// Run one dispatch cycle: drain all queues in priority order,
/// deliver to subscribers, and record receipts.
///
/// Returns the count of dispatched and unrouted signals.
pub fn dispatch_cycle(
    queues: &mut SignalQueues,
    registry: &SubscriptionRegistry,
    receipt_log: &mut DeliveryReceiptLog,
) -> DispatchResult {
    let mut dispatched = 0;
    let mut unrouted = 0;

    while let Some(signal) = queues.pop_highest_priority() {
        let topic = signal_topic(&signal);
        let subscribers = registry.subscribers_for(topic);

        if subscribers.is_empty() {
            let receipt = DeliveryReceipt::new(
                signal.id.as_str(),
                DeliveryOutcome::Dropped {
                    reason: format!("no subscribers for topic '{}'", topic),
                },
                "fabric",
            );
            receipt_log.record(receipt);
            unrouted += 1;
        } else {
            for sub in subscribers {
                let receipt = DeliveryReceipt::new(
                    signal.id.as_str(),
                    DeliveryOutcome::Delivered,
                    &sub.label,
                );
                receipt_log.record(receipt);
            }
            dispatched += 1;
        }
    }

    DispatchResult {
        dispatched,
        unrouted,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subscription::SubscriberId;
    use hydra_animus::{
        graph::PrimeGraph,
        semiring::signal::{Signal, SignalId, SignalTier, SignalWeight},
    };

    fn make_signal(tier: SignalTier) -> Signal {
        Signal::new(
            PrimeGraph::new(),
            SignalId::identity(),
            SignalWeight::max(),
            tier,
            3,
        )
    }

    #[test]
    fn dispatch_with_no_subscribers_is_unrouted() {
        let mut queues = SignalQueues::new();
        let registry = SubscriptionRegistry::new();
        let mut log = DeliveryReceiptLog::new();

        queues.enqueue(make_signal(SignalTier::Fleet)).unwrap();
        let result = dispatch_cycle(&mut queues, &registry, &mut log);

        assert_eq!(result.dispatched, 0);
        assert_eq!(result.unrouted, 1);
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn dispatch_with_subscriber_delivers() {
        let mut queues = SignalQueues::new();
        let mut registry = SubscriptionRegistry::new();
        let mut log = DeliveryReceiptLog::new();

        registry
            .subscribe(
                "signal.fleet",
                SubscriberId::from_value("sub-1"),
                "fleet-handler",
            )
            .unwrap();

        queues.enqueue(make_signal(SignalTier::Fleet)).unwrap();
        let result = dispatch_cycle(&mut queues, &registry, &mut log);

        assert_eq!(result.dispatched, 1);
        assert_eq!(result.unrouted, 0);
        assert_eq!(log.success_count(), 1);
    }
}
