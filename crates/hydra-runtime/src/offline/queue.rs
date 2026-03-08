//! PendingSyncQueue — queues actions that need cloud access for later sync.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

/// Priority of a pending action
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// An action that was deferred while offline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingAction {
    pub id: String,
    pub action_type: String,
    pub payload: serde_json::Value,
    pub priority: SyncPriority,
    pub created_at: String,
    pub retry_count: u32,
    pub max_retries: u32,
}

impl PendingAction {
    pub fn new(action_type: &str, payload: serde_json::Value, priority: SyncPriority) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            action_type: action_type.to_string(),
            payload,
            priority,
            created_at: chrono::Utc::now().to_rfc3339(),
            retry_count: 0,
            max_retries: 3,
        }
    }

    /// Whether this action can be retried
    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    /// Increment retry count
    pub fn record_retry(&mut self) {
        self.retry_count += 1;
    }
}

/// Queue for actions that need to sync when connectivity is restored
pub struct PendingSyncQueue {
    queue: parking_lot::Mutex<VecDeque<PendingAction>>,
    max_size: usize,
    total_enqueued: parking_lot::Mutex<u64>,
    total_synced: parking_lot::Mutex<u64>,
    total_dropped: parking_lot::Mutex<u64>,
}

impl PendingSyncQueue {
    pub fn new(max_size: usize) -> Self {
        Self {
            queue: parking_lot::Mutex::new(VecDeque::new()),
            max_size,
            total_enqueued: parking_lot::Mutex::new(0),
            total_synced: parking_lot::Mutex::new(0),
            total_dropped: parking_lot::Mutex::new(0),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(1000)
    }

    /// Add an action to the queue. Returns false if queue is full and action was dropped.
    pub fn enqueue(&self, action: PendingAction) -> bool {
        let mut queue = self.queue.lock();
        if queue.len() >= self.max_size {
            // Drop lowest priority from front if new action is higher priority
            if let Some(front) = queue.front() {
                if action.priority > front.priority {
                    queue.pop_front();
                    *self.total_dropped.lock() += 1;
                } else {
                    *self.total_dropped.lock() += 1;
                    return false;
                }
            }
        }
        queue.push_back(action);
        *self.total_enqueued.lock() += 1;
        true
    }

    /// Take the next action to sync (highest priority first)
    pub fn dequeue(&self) -> Option<PendingAction> {
        let mut queue = self.queue.lock();
        if queue.is_empty() {
            return None;
        }

        // Find highest priority item
        let mut best_idx = 0;
        let mut best_priority = queue[0].priority;
        for (i, action) in queue.iter().enumerate() {
            if action.priority > best_priority {
                best_priority = action.priority;
                best_idx = i;
            }
        }

        queue.remove(best_idx)
    }

    /// Peek at the next action without removing it
    pub fn peek(&self) -> Option<PendingAction> {
        let queue = self.queue.lock();
        if queue.is_empty() {
            return None;
        }
        let mut best_idx = 0;
        let mut best_priority = queue[0].priority;
        for (i, action) in queue.iter().enumerate() {
            if action.priority > best_priority {
                best_priority = action.priority;
                best_idx = i;
            }
        }
        queue.get(best_idx).cloned()
    }

    /// Mark an action as successfully synced
    pub fn mark_synced(&self) {
        *self.total_synced.lock() += 1;
    }

    /// Re-enqueue a failed action (if retries remain)
    pub fn requeue(&self, mut action: PendingAction) -> bool {
        action.record_retry();
        if action.can_retry() {
            self.queue.lock().push_back(action);
            true
        } else {
            *self.total_dropped.lock() += 1;
            false
        }
    }

    /// Number of pending actions
    pub fn len(&self) -> usize {
        self.queue.lock().len()
    }

    /// Whether the queue is empty
    pub fn is_empty(&self) -> bool {
        self.queue.lock().is_empty()
    }

    /// Clear all pending actions
    pub fn clear(&self) -> usize {
        let mut queue = self.queue.lock();
        let count = queue.len();
        queue.clear();
        count
    }

    /// Get queue statistics
    pub fn stats(&self) -> QueueStats {
        QueueStats {
            pending: self.queue.lock().len(),
            total_enqueued: *self.total_enqueued.lock(),
            total_synced: *self.total_synced.lock(),
            total_dropped: *self.total_dropped.lock(),
            max_size: self.max_size,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStats {
    pub pending: usize,
    pub total_enqueued: u64,
    pub total_synced: u64,
    pub total_dropped: u64,
    pub max_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enqueue_dequeue() {
        let queue = PendingSyncQueue::with_defaults();
        let action = PendingAction::new("test", serde_json::json!({}), SyncPriority::Normal);
        assert!(queue.enqueue(action));
        assert_eq!(queue.len(), 1);
        let out = queue.dequeue().unwrap();
        assert_eq!(out.action_type, "test");
        assert!(queue.is_empty());
    }

    #[test]
    fn test_priority_ordering() {
        let queue = PendingSyncQueue::with_defaults();
        queue.enqueue(PendingAction::new(
            "low",
            serde_json::json!({}),
            SyncPriority::Low,
        ));
        queue.enqueue(PendingAction::new(
            "high",
            serde_json::json!({}),
            SyncPriority::High,
        ));
        queue.enqueue(PendingAction::new(
            "normal",
            serde_json::json!({}),
            SyncPriority::Normal,
        ));

        // Should dequeue highest priority first
        assert_eq!(queue.dequeue().unwrap().action_type, "high");
        assert_eq!(queue.dequeue().unwrap().action_type, "normal");
        assert_eq!(queue.dequeue().unwrap().action_type, "low");
    }

    #[test]
    fn test_max_size_eviction() {
        let queue = PendingSyncQueue::new(2);
        queue.enqueue(PendingAction::new(
            "a",
            serde_json::json!({}),
            SyncPriority::Low,
        ));
        queue.enqueue(PendingAction::new(
            "b",
            serde_json::json!({}),
            SyncPriority::Low,
        ));
        // Queue full — higher priority should evict
        let ok = queue.enqueue(PendingAction::new(
            "c",
            serde_json::json!({}),
            SyncPriority::High,
        ));
        assert!(ok);
        assert_eq!(queue.len(), 2);

        let stats = queue.stats();
        assert_eq!(stats.total_dropped, 1);
    }

    #[test]
    fn test_retry_limit() {
        let mut action = PendingAction::new("test", serde_json::json!({}), SyncPriority::Normal);
        assert!(action.can_retry());
        action.record_retry();
        action.record_retry();
        action.record_retry();
        assert!(!action.can_retry());
    }

    #[test]
    fn test_requeue() {
        let queue = PendingSyncQueue::with_defaults();
        let action = PendingAction::new("test", serde_json::json!({}), SyncPriority::Normal);
        assert!(queue.requeue(action));
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn test_stats() {
        let queue = PendingSyncQueue::with_defaults();
        queue.enqueue(PendingAction::new(
            "a",
            serde_json::json!({}),
            SyncPriority::Normal,
        ));
        queue.enqueue(PendingAction::new(
            "b",
            serde_json::json!({}),
            SyncPriority::Normal,
        ));
        queue.dequeue();
        queue.mark_synced();

        let stats = queue.stats();
        assert_eq!(stats.total_enqueued, 2);
        assert_eq!(stats.total_synced, 1);
        assert_eq!(stats.pending, 1);
    }
}
