//! Subscription registry for signal topics.
//!
//! Modules subscribe to signal topics (e.g. "signal.fleet") and receive
//! signals routed to those topics during dispatch.

use crate::constants::{MAX_SUBSCRIBERS_PER_TOPIC, MAX_TOPICS};
use crate::errors::SignalError;
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for a subscriber.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubscriberId(String);

impl SubscriberId {
    /// Generate a new unique subscriber ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Create a subscriber ID from a known string value.
    pub fn from_value(s: &str) -> Self {
        Self(s.to_string())
    }

    /// Returns the string representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for SubscriberId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SubscriberId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A single subscription binding a subscriber to a topic.
#[derive(Debug, Clone)]
pub struct Subscription {
    /// The subscriber's unique ID.
    pub subscriber_id: SubscriberId,
    /// The topic being subscribed to (e.g. "signal.fleet").
    pub topic: String,
    /// Human-readable label for the subscriber.
    pub label: String,
}

impl Subscription {
    /// Create a new subscription.
    pub fn new(subscriber_id: SubscriberId, topic: &str, label: &str) -> Self {
        Self {
            subscriber_id,
            topic: topic.to_string(),
            label: label.to_string(),
        }
    }
}

/// Registry of all active subscriptions, keyed by topic.
pub struct SubscriptionRegistry {
    /// Map from topic name to list of subscriptions.
    topics: HashMap<String, Vec<Subscription>>,
}

impl SubscriptionRegistry {
    /// Create a new empty subscription registry.
    pub fn new() -> Self {
        Self {
            topics: HashMap::new(),
        }
    }

    /// Subscribe to a topic. Returns the subscription or an error.
    pub fn subscribe(
        &mut self,
        topic: &str,
        subscriber_id: SubscriberId,
        label: &str,
    ) -> Result<Subscription, SignalError> {
        if self.topics.len() >= MAX_TOPICS && !self.topics.contains_key(topic) {
            return Err(SignalError::SubscriptionFailed {
                topic: topic.to_string(),
                reason: format!("maximum topics ({}) reached", MAX_TOPICS),
            });
        }

        let subs = self.topics.entry(topic.to_string()).or_default();

        if subs.len() >= MAX_SUBSCRIBERS_PER_TOPIC {
            return Err(SignalError::SubscriptionFailed {
                topic: topic.to_string(),
                reason: format!(
                    "maximum subscribers ({}) reached for topic",
                    MAX_SUBSCRIBERS_PER_TOPIC
                ),
            });
        }

        let subscription = Subscription::new(subscriber_id, topic, label);
        subs.push(subscription.clone());
        Ok(subscription)
    }

    /// Unsubscribe a subscriber from a topic.
    pub fn unsubscribe(&mut self, topic: &str, subscriber_id: &SubscriberId) {
        if let Some(subs) = self.topics.get_mut(topic) {
            subs.retain(|s| s.subscriber_id != *subscriber_id);
            if subs.is_empty() {
                self.topics.remove(topic);
            }
        }
    }

    /// Get all subscribers for a given topic.
    pub fn subscribers_for(&self, topic: &str) -> &[Subscription] {
        self.topics.get(topic).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Returns the total number of subscriptions across all topics.
    pub fn total_subscriptions(&self) -> usize {
        self.topics.values().map(|v| v.len()).sum()
    }

    /// Returns the number of topics with at least one subscriber.
    pub fn topic_count(&self) -> usize {
        self.topics.len()
    }
}

impl Default for SubscriptionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscribe_and_lookup() {
        let mut registry = SubscriptionRegistry::new();
        let id = SubscriberId::from_value("sub-1");
        registry
            .subscribe("signal.fleet", id.clone(), "fleet-handler")
            .unwrap();

        let subs = registry.subscribers_for("signal.fleet");
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].subscriber_id, id);
    }

    #[test]
    fn unsubscribe_removes_entry() {
        let mut registry = SubscriptionRegistry::new();
        let id = SubscriberId::from_value("sub-1");
        registry
            .subscribe("signal.fleet", id.clone(), "fleet-handler")
            .unwrap();
        registry.unsubscribe("signal.fleet", &id);

        assert_eq!(registry.subscribers_for("signal.fleet").len(), 0);
        assert_eq!(registry.topic_count(), 0);
    }

    #[test]
    fn empty_topic_returns_empty_slice() {
        let registry = SubscriptionRegistry::new();
        assert!(registry.subscribers_for("nonexistent").is_empty());
    }
}
