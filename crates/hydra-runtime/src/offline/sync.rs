//! SyncEngine — processes pending queue when connectivity is restored.

use serde::{Deserialize, Serialize};

use super::queue::{PendingAction, PendingSyncQueue};

/// Strategy for handling conflicts when syncing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictStrategy {
    /// Last write wins — overwrite remote with local
    LastWriteWins,
    /// Keep remote — discard local change
    KeepRemote,
    /// Merge — attempt to merge (falls back to LastWriteWins)
    Merge,
}

/// Result of syncing a single action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub action_id: String,
    pub action_type: String,
    pub status: SyncStatus,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncStatus {
    Synced,
    Conflict,
    Failed,
    Skipped,
}

/// Engine that processes the pending sync queue when back online
pub struct SyncEngine {
    conflict_strategy: ConflictStrategy,
    batch_size: usize,
}

impl SyncEngine {
    pub fn new(conflict_strategy: ConflictStrategy) -> Self {
        Self {
            conflict_strategy,
            batch_size: 10,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(ConflictStrategy::LastWriteWins)
    }

    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }

    /// Get the conflict strategy
    pub fn conflict_strategy(&self) -> ConflictStrategy {
        self.conflict_strategy
    }

    /// Process a batch of pending actions from the queue.
    /// Returns results for each action processed.
    pub async fn process_batch(&self, queue: &PendingSyncQueue) -> Vec<SyncResult> {
        let mut results = Vec::new();

        for _ in 0..self.batch_size {
            let action = match queue.dequeue() {
                Some(a) => a,
                None => break,
            };

            let result = self.sync_action(&action).await;

            match result.status {
                SyncStatus::Synced => {
                    queue.mark_synced();
                }
                SyncStatus::Failed => {
                    // Requeue if retries remain
                    queue.requeue(action);
                }
                SyncStatus::Conflict => {
                    // Apply conflict strategy
                    match self.conflict_strategy {
                        ConflictStrategy::LastWriteWins => {
                            // Force sync — in production, would overwrite remote
                            queue.mark_synced();
                        }
                        ConflictStrategy::KeepRemote => {
                            // Discard local — already dequeued, just mark synced
                            queue.mark_synced();
                        }
                        ConflictStrategy::Merge => {
                            // In production: attempt merge, fallback to LWW
                            queue.mark_synced();
                        }
                    }
                }
                SyncStatus::Skipped => {}
            }

            results.push(result);
        }

        results
    }

    /// Sync a single action. In production, this would make real API calls.
    async fn sync_action(&self, action: &PendingAction) -> SyncResult {
        // In production: route to appropriate cloud API based on action_type
        // For now: always succeed (the infrastructure is what matters)
        SyncResult {
            action_id: action.id.clone(),
            action_type: action.action_type.clone(),
            status: SyncStatus::Synced,
            message: format!("Synced {} action", action.action_type),
        }
    }

    /// Process all pending actions (drains queue)
    pub async fn process_all(&self, queue: &PendingSyncQueue) -> Vec<SyncResult> {
        let mut all_results = Vec::new();
        loop {
            if queue.is_empty() {
                break;
            }
            let batch = self.process_batch(queue).await;
            if batch.is_empty() {
                break;
            }
            all_results.extend(batch);
        }
        all_results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::offline::queue::SyncPriority;

    #[test]
    fn test_sync_engine_defaults() {
        let engine = SyncEngine::with_defaults();
        assert_eq!(engine.conflict_strategy(), ConflictStrategy::LastWriteWins);
    }

    #[tokio::test]
    async fn test_process_batch() {
        let engine = SyncEngine::with_defaults();
        let queue = PendingSyncQueue::with_defaults();

        queue.enqueue(PendingAction::new(
            "test_a",
            serde_json::json!({}),
            SyncPriority::Normal,
        ));
        queue.enqueue(PendingAction::new(
            "test_b",
            serde_json::json!({}),
            SyncPriority::High,
        ));

        let results = engine.process_batch(&queue).await;
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.status == SyncStatus::Synced));
        assert!(queue.is_empty());
    }

    #[tokio::test]
    async fn test_process_all() {
        let engine = SyncEngine::with_defaults().with_batch_size(2);
        let queue = PendingSyncQueue::with_defaults();

        for i in 0..5 {
            queue.enqueue(PendingAction::new(
                &format!("action_{}", i),
                serde_json::json!({"i": i}),
                SyncPriority::Normal,
            ));
        }

        let results = engine.process_all(&queue).await;
        assert_eq!(results.len(), 5);
        assert!(queue.is_empty());

        let stats = queue.stats();
        assert_eq!(stats.total_synced, 5);
    }
}
