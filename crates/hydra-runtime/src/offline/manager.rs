//! OfflineManager — coordinates offline mode, routing, and sync.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::Notify;

use super::monitor::{ConnectivityMonitor, ConnectivityState, MonitorConfig};
use super::queue::{PendingAction, PendingSyncQueue, SyncPriority};
use super::sync::{ConflictStrategy, SyncEngine, SyncResult};
use crate::degradation::DegradationLevel;

/// Configuration for offline mode
#[derive(Debug, Clone)]
pub struct OfflineConfig {
    /// Monitor configuration
    pub monitor: MonitorConfig,
    /// Conflict resolution strategy
    pub conflict_strategy: ConflictStrategy,
    /// Maximum queue size for pending actions
    pub max_queue_size: usize,
    /// Minimum degradation level when offline
    pub offline_degradation: DegradationLevel,
}

impl Default for OfflineConfig {
    fn default() -> Self {
        Self {
            monitor: MonitorConfig::default(),
            conflict_strategy: ConflictStrategy::LastWriteWins,
            max_queue_size: 1000,
            offline_degradation: DegradationLevel::Reduced,
        }
    }
}

/// Snapshot of offline manager state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfflineStatus {
    pub connectivity: ConnectivityState,
    pub pending_actions: usize,
    pub total_synced: u64,
    pub total_dropped: u64,
    pub local_llm_only: bool,
}

/// Manages offline mode: detects connectivity, routes to local LLM, queues cloud actions
pub struct OfflineManager {
    monitor: Arc<ConnectivityMonitor>,
    queue: Arc<PendingSyncQueue>,
    sync_engine: SyncEngine,
    config: OfflineConfig,
    /// Whether we're currently in offline mode (may differ from raw connectivity)
    offline_mode: parking_lot::Mutex<bool>,
    /// Shutdown signal
    shutdown: Arc<Notify>,
}

impl OfflineManager {
    pub fn new(config: OfflineConfig) -> Self {
        let monitor = Arc::new(ConnectivityMonitor::new(config.monitor.clone()));
        let queue = Arc::new(PendingSyncQueue::new(config.max_queue_size));
        let sync_engine = SyncEngine::new(config.conflict_strategy);

        Self {
            monitor,
            queue,
            sync_engine,
            config,
            offline_mode: parking_lot::Mutex::new(false),
            shutdown: Arc::new(Notify::new()),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(OfflineConfig::default())
    }

    /// Get a reference to the connectivity monitor
    pub fn monitor(&self) -> &ConnectivityMonitor {
        &self.monitor
    }

    /// Get a reference to the pending sync queue
    pub fn queue(&self) -> &PendingSyncQueue {
        &self.queue
    }

    /// Whether the system is in offline mode
    pub fn is_offline(&self) -> bool {
        *self.offline_mode.lock()
    }

    /// Whether routing should go to local LLM only
    pub fn local_llm_only(&self) -> bool {
        *self.offline_mode.lock()
    }

    /// The minimum degradation level to apply when offline
    pub fn offline_degradation_level(&self) -> DegradationLevel {
        if *self.offline_mode.lock() {
            self.config.offline_degradation
        } else {
            DegradationLevel::Normal
        }
    }

    /// Queue an action for later sync (when we're offline)
    pub fn queue_action(
        &self,
        action_type: &str,
        payload: serde_json::Value,
        priority: SyncPriority,
    ) -> bool {
        let action = PendingAction::new(action_type, payload, priority);
        self.queue.enqueue(action)
    }

    /// Handle a connectivity state change. Returns actions to take.
    pub async fn handle_state_change(&self, new_state: ConnectivityState) -> StateChangeActions {
        match new_state {
            ConnectivityState::Online => {
                let was_offline = *self.offline_mode.lock();
                *self.offline_mode.lock() = false;

                if was_offline && !self.queue.is_empty() {
                    // Sync pending actions
                    let results = self.sync_engine.process_all(&self.queue).await;
                    StateChangeActions {
                        went_online: true,
                        went_offline: false,
                        synced: results,
                        degradation_suggestion: Some(DegradationLevel::Normal),
                    }
                } else {
                    StateChangeActions {
                        went_online: was_offline,
                        went_offline: false,
                        synced: vec![],
                        degradation_suggestion: Some(DegradationLevel::Normal),
                    }
                }
            }
            ConnectivityState::Offline => {
                let was_online = !*self.offline_mode.lock();
                *self.offline_mode.lock() = true;

                StateChangeActions {
                    went_online: false,
                    went_offline: was_online,
                    synced: vec![],
                    degradation_suggestion: Some(self.config.offline_degradation),
                }
            }
            ConnectivityState::Unknown => StateChangeActions {
                went_online: false,
                went_offline: false,
                synced: vec![],
                degradation_suggestion: None,
            },
        }
    }

    /// Run a connectivity check and handle any state change.
    /// Returns true if state changed.
    pub async fn check_and_handle(&self) -> Option<StateChangeActions> {
        let changed = self.monitor.check().await;
        if changed {
            Some(self.handle_state_change(self.monitor.state()).await)
        } else {
            None
        }
    }

    /// Get current status
    pub fn status(&self) -> OfflineStatus {
        let stats = self.queue.stats();
        OfflineStatus {
            connectivity: self.monitor.state(),
            pending_actions: stats.pending,
            total_synced: stats.total_synced,
            total_dropped: stats.total_dropped,
            local_llm_only: self.local_llm_only(),
        }
    }

    /// Signal shutdown
    pub fn shutdown(&self) {
        self.shutdown.notify_one();
    }
}

/// Actions resulting from a connectivity state change
#[derive(Debug)]
pub struct StateChangeActions {
    pub went_online: bool,
    pub went_offline: bool,
    pub synced: Vec<SyncResult>,
    pub degradation_suggestion: Option<DegradationLevel>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_starts_online_mode() {
        let mgr = OfflineManager::with_defaults();
        assert!(!mgr.is_offline());
        assert!(!mgr.local_llm_only());
    }

    #[tokio::test]
    async fn test_go_offline() {
        let mgr = OfflineManager::with_defaults();
        let actions = mgr.handle_state_change(ConnectivityState::Offline).await;
        assert!(actions.went_offline);
        assert!(mgr.is_offline());
        assert!(mgr.local_llm_only());
        assert_eq!(mgr.offline_degradation_level(), DegradationLevel::Reduced);
    }

    #[tokio::test]
    async fn test_go_online_syncs_queue() {
        let mgr = OfflineManager::with_defaults();
        // Go offline first
        mgr.handle_state_change(ConnectivityState::Offline).await;
        // Queue some actions
        mgr.queue_action("test", serde_json::json!({}), SyncPriority::Normal);
        mgr.queue_action("test2", serde_json::json!({}), SyncPriority::High);
        assert_eq!(mgr.queue().len(), 2);

        // Go online — should sync
        let actions = mgr.handle_state_change(ConnectivityState::Online).await;
        assert!(actions.went_online);
        assert_eq!(actions.synced.len(), 2);
        assert!(mgr.queue().is_empty());
        assert!(!mgr.is_offline());
    }

    #[test]
    fn test_status() {
        let mgr = OfflineManager::with_defaults();
        let status = mgr.status();
        assert_eq!(status.connectivity, ConnectivityState::Unknown);
        assert_eq!(status.pending_actions, 0);
        assert!(!status.local_llm_only);
    }

    #[test]
    fn test_queue_action() {
        let mgr = OfflineManager::with_defaults();
        assert!(mgr.queue_action(
            "api_call",
            serde_json::json!({"url": "/test"}),
            SyncPriority::Normal
        ));
        assert_eq!(mgr.queue().len(), 1);
    }
}
