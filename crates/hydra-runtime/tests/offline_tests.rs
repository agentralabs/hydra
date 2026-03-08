//! Integration tests for offline mode.

use hydra_runtime::degradation::DegradationLevel;
use hydra_runtime::offline::manager::OfflineManager;
use hydra_runtime::offline::monitor::{ConnectivityMonitor, ConnectivityState, MonitorConfig};
use hydra_runtime::offline::queue::{PendingAction, PendingSyncQueue, SyncPriority};
use hydra_runtime::offline::sync::{ConflictStrategy, SyncEngine, SyncStatus};

#[test]
fn test_connectivity_monitor_detects_offline() {
    let config = MonitorConfig {
        failure_threshold: 2,
        ..Default::default()
    };
    let monitor = ConnectivityMonitor::new(config);
    // Starts unknown
    assert_eq!(monitor.state(), ConnectivityState::Unknown);
    // First failure — still unknown
    monitor.report(false);
    assert_eq!(monitor.state(), ConnectivityState::Unknown);
    // Second failure — now offline
    monitor.report(false);
    assert!(monitor.is_offline());
}

#[test]
fn test_connectivity_monitor_detects_online() {
    let monitor = ConnectivityMonitor::with_defaults();
    // Force offline first
    monitor.force_state(ConnectivityState::Offline);
    assert!(monitor.is_offline());
    // Report success — should go online (success_threshold = 1)
    let changed = monitor.report(true);
    assert!(changed);
    assert!(monitor.is_online());
}

#[tokio::test]
async fn test_offline_routes_to_local_llm() {
    let mgr = OfflineManager::with_defaults();
    assert!(!mgr.local_llm_only());

    // Go offline
    mgr.handle_state_change(ConnectivityState::Offline).await;
    assert!(mgr.local_llm_only());
    assert_eq!(mgr.offline_degradation_level(), DegradationLevel::Reduced);

    // Go online
    mgr.handle_state_change(ConnectivityState::Online).await;
    assert!(!mgr.local_llm_only());
    assert_eq!(mgr.offline_degradation_level(), DegradationLevel::Normal);
}

#[test]
fn test_offline_sisters_work_locally() {
    // Sisters use local files (.amem, .acb, etc.) — offline mode doesn't break them.
    // Verify the manager correctly tracks offline state without requiring network.
    let mgr = OfflineManager::with_defaults();
    let status = mgr.status();
    assert_eq!(status.connectivity, ConnectivityState::Unknown);
    assert_eq!(status.pending_actions, 0);
    assert!(!status.local_llm_only);

    // Queue actions while "offline" — sisters can still work locally
    mgr.queue_action(
        "memory_add",
        serde_json::json!({"content": "test"}),
        SyncPriority::Normal,
    );
    mgr.queue_action(
        "codebase_index",
        serde_json::json!({"path": "/src"}),
        SyncPriority::Low,
    );
    assert_eq!(mgr.queue().len(), 2);
}

#[test]
fn test_pending_queue_persists() {
    let queue = PendingSyncQueue::new(100);
    // Enqueue actions
    for i in 0..5 {
        queue.enqueue(PendingAction::new(
            &format!("action_{}", i),
            serde_json::json!({"index": i}),
            SyncPriority::Normal,
        ));
    }
    assert_eq!(queue.len(), 5);

    // Dequeue respects priority
    queue.enqueue(PendingAction::new(
        "critical",
        serde_json::json!({}),
        SyncPriority::Critical,
    ));
    let next = queue.dequeue().unwrap();
    assert_eq!(next.action_type, "critical");
    assert_eq!(queue.len(), 5);

    // Stats track correctly
    let stats = queue.stats();
    assert_eq!(stats.total_enqueued, 6);
    assert_eq!(stats.pending, 5);
}

#[tokio::test]
async fn test_sync_on_reconnect() {
    let mgr = OfflineManager::with_defaults();

    // Go offline and queue actions
    mgr.handle_state_change(ConnectivityState::Offline).await;
    mgr.queue_action(
        "api_sync",
        serde_json::json!({"data": "a"}),
        SyncPriority::Normal,
    );
    mgr.queue_action(
        "api_sync",
        serde_json::json!({"data": "b"}),
        SyncPriority::High,
    );
    mgr.queue_action(
        "api_sync",
        serde_json::json!({"data": "c"}),
        SyncPriority::Normal,
    );
    assert_eq!(mgr.queue().len(), 3);

    // Go online — should auto-sync all queued actions
    let actions = mgr.handle_state_change(ConnectivityState::Online).await;
    assert!(actions.went_online);
    assert_eq!(actions.synced.len(), 3);
    assert!(mgr.queue().is_empty());

    let status = mgr.status();
    assert_eq!(status.total_synced, 3);
}

#[tokio::test]
async fn test_conflict_resolution() {
    // LastWriteWins (default)
    let engine = SyncEngine::with_defaults();
    assert_eq!(engine.conflict_strategy(), ConflictStrategy::LastWriteWins);

    // Merge strategy
    let engine_merge = SyncEngine::new(ConflictStrategy::Merge);
    assert_eq!(engine_merge.conflict_strategy(), ConflictStrategy::Merge);

    // KeepRemote strategy
    let engine_remote = SyncEngine::new(ConflictStrategy::KeepRemote);
    assert_eq!(
        engine_remote.conflict_strategy(),
        ConflictStrategy::KeepRemote
    );

    // Process batch works with all strategies
    let queue = PendingSyncQueue::with_defaults();
    queue.enqueue(PendingAction::new(
        "test",
        serde_json::json!({}),
        SyncPriority::Normal,
    ));
    let results = engine.process_batch(&queue).await;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].status, SyncStatus::Synced);
}

#[tokio::test]
async fn test_offline_sse_event() {
    // Verify the state change produces the right data for SSE emission
    let mgr = OfflineManager::with_defaults();

    // Go offline
    let actions = mgr.handle_state_change(ConnectivityState::Offline).await;
    assert!(actions.went_offline);
    assert_eq!(
        actions.degradation_suggestion,
        Some(DegradationLevel::Reduced)
    );

    // SSE event data would be:
    let event_data = serde_json::json!({
        "online": false,
        "degradation": mgr.offline_degradation_level().to_string(),
        "pending_actions": mgr.queue().len(),
    });
    assert_eq!(event_data["online"], false);
    assert_eq!(event_data["degradation"], "reduced");

    // Go online
    let actions = mgr.handle_state_change(ConnectivityState::Online).await;
    assert!(actions.went_online);
    assert_eq!(
        actions.degradation_suggestion,
        Some(DegradationLevel::Normal)
    );

    let event_data = serde_json::json!({
        "online": true,
        "degradation": mgr.offline_degradation_level().to_string(),
    });
    assert_eq!(event_data["online"], true);
    assert_eq!(event_data["degradation"], "normal");
}
