//! Integration tests for the consolidation daemon.

use std::time::Duration;

use hydra_runtime::daemon::consolidation::{ConsolidationDaemon, DaemonConfig};
use hydra_runtime::daemon::tasks::{TaskId, TaskStatus};
use hydra_runtime::degradation::DegradationLevel;

#[test]
fn test_daemon_creates_with_defaults() {
    let daemon = ConsolidationDaemon::with_defaults();
    assert_eq!(daemon.degradation_level(), DegradationLevel::Normal);
    assert!(!daemon.is_running());
}

#[tokio::test]
async fn test_daemon_lifecycle() {
    let daemon = ConsolidationDaemon::with_defaults();
    assert!(!daemon.is_running());
    daemon.start();
    assert!(daemon.is_running());
    daemon.stop();
    tokio::time::sleep(Duration::from_millis(50)).await;
}

#[tokio::test]
async fn test_scheduler_due_tasks_normal() {
    let daemon = ConsolidationDaemon::with_defaults();
    let due = daemon.scheduler().due_tasks(DegradationLevel::Normal);
    // All 7 tasks should be due initially
    assert_eq!(due.len(), 7);
}

#[tokio::test]
async fn test_scheduler_due_tasks_emergency() {
    let daemon = ConsolidationDaemon::with_defaults();
    let due = daemon.scheduler().due_tasks(DegradationLevel::Emergency);
    // Only HealthCheck runs in Emergency
    assert_eq!(due.len(), 1);
    assert!(due.contains(&TaskId::HealthCheck));
}

#[tokio::test]
async fn test_scheduler_records_success() {
    let daemon = ConsolidationDaemon::with_defaults();
    daemon
        .scheduler()
        .record_result(TaskId::HealthCheck, TaskStatus::Success);
    let due = daemon.scheduler().due_tasks(DegradationLevel::Normal);
    assert!(!due.contains(&TaskId::HealthCheck));
}

#[tokio::test]
async fn test_scheduler_backoff_on_failure() {
    let daemon = ConsolidationDaemon::with_defaults();
    daemon
        .scheduler()
        .record_result(TaskId::HealthCheck, TaskStatus::Failed);
    assert_eq!(daemon.scheduler().failure_count(TaskId::HealthCheck), 1);
    daemon
        .scheduler()
        .record_result(TaskId::HealthCheck, TaskStatus::Failed);
    assert_eq!(daemon.scheduler().failure_count(TaskId::HealthCheck), 2);
    // Success resets
    daemon
        .scheduler()
        .record_result(TaskId::HealthCheck, TaskStatus::Success);
    assert_eq!(daemon.scheduler().failure_count(TaskId::HealthCheck), 0);
}

#[tokio::test]
async fn test_daemon_executes_tasks_on_tick() {
    let config = DaemonConfig {
        tick_interval: Duration::from_millis(50),
        opportunistic_enabled: false,
        ..Default::default()
    };
    let daemon = ConsolidationDaemon::new(config);
    daemon.start();
    tokio::time::sleep(Duration::from_millis(200)).await;
    daemon.stop();
    tokio::time::sleep(Duration::from_millis(50)).await;

    let results = daemon.drain_results();
    assert!(
        results.len() >= 7,
        "Should execute all 7 due tasks, got {}",
        results.len()
    );
}

#[tokio::test]
async fn test_daemon_degradation_filters_tasks() {
    let config = DaemonConfig {
        tick_interval: Duration::from_millis(50),
        opportunistic_enabled: false,
        ..Default::default()
    };
    let daemon = ConsolidationDaemon::new(config);
    daemon.set_degradation_level(DegradationLevel::Emergency);
    daemon.start();
    tokio::time::sleep(Duration::from_millis(200)).await;
    daemon.stop();
    tokio::time::sleep(Duration::from_millis(50)).await;

    let results = daemon.drain_results();
    // Emergency only allows HealthCheck
    assert!(!results.is_empty());
    assert!(
        results.len() < 7,
        "Emergency should filter tasks, got {}",
        results.len()
    );
}

#[tokio::test]
async fn test_opportunistic_idle_detection() {
    let daemon = ConsolidationDaemon::with_defaults();
    assert!(!daemon.opportunistic().should_run());
    // Simulate low CPU
    daemon.opportunistic().update_cpu(5.0);
    // Won't run yet due to min_idle_duration (30s default)
    // But with 0ms idle duration:
    let config = DaemonConfig {
        min_idle_duration: Duration::from_millis(0),
        ..Default::default()
    };
    let daemon2 = ConsolidationDaemon::new(config);
    daemon2.opportunistic().update_cpu(5.0);
    assert!(daemon2.opportunistic().should_run());
}

#[tokio::test]
async fn test_scheduler_trigger_forces_due() {
    let daemon = ConsolidationDaemon::with_defaults();
    daemon
        .scheduler()
        .record_result(TaskId::HealthCheck, TaskStatus::Success);
    assert!(!daemon
        .scheduler()
        .due_tasks(DegradationLevel::Normal)
        .contains(&TaskId::HealthCheck));
    daemon.scheduler().trigger(TaskId::HealthCheck);
    assert!(daemon
        .scheduler()
        .due_tasks(DegradationLevel::Normal)
        .contains(&TaskId::HealthCheck));
}

#[tokio::test]
async fn test_scheduler_disable_task() {
    let daemon = ConsolidationDaemon::with_defaults();
    daemon.scheduler().set_enabled(TaskId::IndexReorg, false);
    let due = daemon.scheduler().due_tasks(DegradationLevel::Normal);
    assert!(!due.contains(&TaskId::IndexReorg));
}

#[tokio::test]
async fn test_degradation_level_change_at_runtime() {
    let config = DaemonConfig {
        tick_interval: Duration::from_millis(50),
        opportunistic_enabled: false,
        ..Default::default()
    };
    let daemon = ConsolidationDaemon::new(config);
    daemon.start();
    // Start at Normal, let first tick run all 7
    tokio::time::sleep(Duration::from_millis(150)).await;
    let count_normal = daemon.result_count();
    assert!(count_normal >= 7);

    // Switch to Emergency
    daemon.set_degradation_level(DegradationLevel::Emergency);
    assert_eq!(daemon.degradation_level(), DegradationLevel::Emergency);

    daemon.stop();
    tokio::time::sleep(Duration::from_millis(50)).await;
}
