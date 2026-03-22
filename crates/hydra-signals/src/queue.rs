//! Tier-based signal queues with priority-ordered dispatch.
//!
//! Constitutional signals CANNOT be queued — they must be handled immediately.
//! All other tiers have bounded queues with backpressure detection.

use crate::constants::{
    BACKPRESSURE_THRESHOLD, QUEUE_CAPACITY_ADVERSARIAL, QUEUE_CAPACITY_BELIEF_REVISION,
    QUEUE_CAPACITY_COMPANION, QUEUE_CAPACITY_FLEET, QUEUE_CAPACITY_PREDICTION,
};
use crate::errors::SignalError;
use hydra_animus::{Signal, SignalTier};
use std::collections::VecDeque;

/// A bounded queue for a single signal tier.
pub struct TierQueue {
    /// The tier this queue serves.
    tier: SignalTier,
    /// The bounded buffer.
    buffer: VecDeque<Signal>,
    /// Maximum capacity.
    capacity: usize,
}

impl TierQueue {
    /// Create a new tier queue with the given capacity.
    pub fn new(tier: SignalTier, capacity: usize) -> Self {
        Self {
            tier,
            buffer: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// Push a signal into the queue. Returns error if full.
    pub fn push(&mut self, signal: Signal) -> Result<(), SignalError> {
        if self.buffer.len() >= self.capacity {
            return Err(SignalError::QueueFull {
                tier: format!("{:?}", self.tier),
                capacity: self.capacity,
            });
        }
        self.buffer.push_back(signal);
        Ok(())
    }

    /// Pop the next signal from the front of the queue.
    pub fn pop(&mut self) -> Option<Signal> {
        self.buffer.pop_front()
    }

    /// Returns the number of signals currently in the queue.
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Returns true if the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Returns the queue capacity.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the fill fraction (0.0 = empty, 1.0 = full).
    pub fn fill_fraction(&self) -> f64 {
        if self.capacity == 0 {
            return 1.0;
        }
        self.len() as f64 / self.capacity as f64
    }

    /// Returns true if backpressure should be applied.
    pub fn is_backpressured(&self) -> bool {
        self.fill_fraction() >= BACKPRESSURE_THRESHOLD
    }
}

/// All signal queues, one per non-constitutional tier.
pub struct SignalQueues {
    /// Adversarial tier queue (highest priority among queued).
    pub adversarial: TierQueue,
    /// Belief revision tier queue.
    pub belief_revision: TierQueue,
    /// Fleet tier queue.
    pub fleet: TierQueue,
    /// Companion tier queue.
    pub companion: TierQueue,
    /// Prediction tier queue (lowest priority).
    pub prediction: TierQueue,
}

impl SignalQueues {
    /// Create all queues with default capacities.
    pub fn new() -> Self {
        Self {
            adversarial: TierQueue::new(SignalTier::Adversarial, QUEUE_CAPACITY_ADVERSARIAL),
            belief_revision: TierQueue::new(
                SignalTier::BeliefRevision,
                QUEUE_CAPACITY_BELIEF_REVISION,
            ),
            fleet: TierQueue::new(SignalTier::Fleet, QUEUE_CAPACITY_FLEET),
            companion: TierQueue::new(SignalTier::Companion, QUEUE_CAPACITY_COMPANION),
            prediction: TierQueue::new(SignalTier::Prediction, QUEUE_CAPACITY_PREDICTION),
        }
    }

    /// Enqueue a signal into the appropriate tier queue.
    /// Constitutional signals CANNOT be queued — returns an error.
    pub fn enqueue(&mut self, signal: Signal) -> Result<(), SignalError> {
        match signal.tier {
            SignalTier::Constitution => Err(SignalError::NoHandlerForTier {
                tier: "Constitution (cannot be queued)".to_string(),
            }),
            SignalTier::Adversarial => self.adversarial.push(signal),
            SignalTier::BeliefRevision => self.belief_revision.push(signal),
            SignalTier::Fleet => self.fleet.push(signal),
            SignalTier::Companion => self.companion.push(signal),
            SignalTier::Prediction => self.prediction.push(signal),
        }
    }

    /// Pop the highest-priority signal across all queues.
    /// Priority order: adversarial > belief_revision > fleet > companion > prediction.
    pub fn pop_highest_priority(&mut self) -> Option<Signal> {
        if let Some(s) = self.adversarial.pop() {
            return Some(s);
        }
        if let Some(s) = self.belief_revision.pop() {
            return Some(s);
        }
        if let Some(s) = self.fleet.pop() {
            return Some(s);
        }
        if let Some(s) = self.companion.pop() {
            return Some(s);
        }
        self.prediction.pop()
    }

    /// Returns the total number of signals across all queues.
    pub fn total_count(&self) -> usize {
        self.adversarial.len()
            + self.belief_revision.len()
            + self.fleet.len()
            + self.companion.len()
            + self.prediction.len()
    }

    /// Returns true if any queue is experiencing backpressure.
    pub fn any_backpressure(&self) -> bool {
        self.adversarial.is_backpressured()
            || self.belief_revision.is_backpressured()
            || self.fleet.is_backpressured()
            || self.companion.is_backpressured()
            || self.prediction.is_backpressured()
    }
}

impl Default for SignalQueues {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydra_animus::{
        graph::PrimeGraph,
        semiring::signal::{SignalId, SignalWeight},
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
    fn constitutional_cannot_be_queued() {
        let mut queues = SignalQueues::new();
        let result = queues.enqueue(make_signal(SignalTier::Constitution));
        assert!(result.is_err());
    }

    #[test]
    fn priority_order_respected() {
        let mut queues = SignalQueues::new();
        queues.enqueue(make_signal(SignalTier::Prediction)).unwrap();
        queues.enqueue(make_signal(SignalTier::Fleet)).unwrap();
        queues
            .enqueue(make_signal(SignalTier::Adversarial))
            .unwrap();

        let first = queues.pop_highest_priority().unwrap();
        assert_eq!(first.tier, SignalTier::Adversarial);
        let second = queues.pop_highest_priority().unwrap();
        assert_eq!(second.tier, SignalTier::Fleet);
        let third = queues.pop_highest_priority().unwrap();
        assert_eq!(third.tier, SignalTier::Prediction);
    }

    #[test]
    fn queue_full_returns_error() {
        let mut queue = TierQueue::new(SignalTier::Fleet, 2);
        assert!(queue.push(make_signal(SignalTier::Fleet)).is_ok());
        assert!(queue.push(make_signal(SignalTier::Fleet)).is_ok());
        assert!(matches!(
            queue.push(make_signal(SignalTier::Fleet)),
            Err(SignalError::QueueFull { .. })
        ));
    }

    #[test]
    fn backpressure_detected() {
        let mut queue = TierQueue::new(SignalTier::Fleet, 10);
        for _ in 0..9 {
            queue.push(make_signal(SignalTier::Fleet)).unwrap();
        }
        assert!(queue.is_backpressured());
    }
}
