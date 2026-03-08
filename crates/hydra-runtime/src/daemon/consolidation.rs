//! Consolidation daemon — runs periodic maintenance tasks in the background.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Notify;
use tokio::task::JoinHandle;

use super::opportunistic::{OpportunisticRunner, OpportunisticTask};
use super::scheduler::TaskScheduler;
use super::tasks::{DaemonTask, TaskResult, TaskStatus};
use crate::degradation::DegradationLevel;

/// Configuration for the consolidation daemon
#[derive(Debug, Clone)]
pub struct DaemonConfig {
    /// How often to check for due tasks (tick interval)
    pub tick_interval: Duration,
    /// Whether opportunistic tasks are enabled
    pub opportunistic_enabled: bool,
    /// CPU threshold for opportunistic tasks (percentage)
    pub cpu_threshold: f64,
    /// Minimum idle duration before opportunistic tasks run
    pub min_idle_duration: Duration,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            tick_interval: Duration::from_secs(10),
            opportunistic_enabled: true,
            cpu_threshold: 10.0,
            min_idle_duration: Duration::from_secs(30),
        }
    }
}

/// Background daemon that runs periodic maintenance tasks.
///
/// The daemon:
/// - Checks the scheduler each tick for due tasks
/// - Filters tasks by current degradation level
/// - Executes tasks and records results
/// - Runs opportunistic tasks when the system is idle
pub struct ConsolidationDaemon {
    scheduler: Arc<TaskScheduler>,
    opportunistic: Arc<OpportunisticRunner>,
    config: DaemonConfig,
    degradation_level: Arc<parking_lot::Mutex<DegradationLevel>>,
    shutdown: Arc<Notify>,
    handle: parking_lot::Mutex<Option<JoinHandle<()>>>,
    results: Arc<parking_lot::Mutex<Vec<TaskResult>>>,
}

impl ConsolidationDaemon {
    pub fn new(config: DaemonConfig) -> Self {
        let opportunistic = Arc::new(OpportunisticRunner::new(
            config.cpu_threshold,
            config.min_idle_duration,
        ));

        Self {
            scheduler: Arc::new(TaskScheduler::new()),
            opportunistic,
            config,
            degradation_level: Arc::new(parking_lot::Mutex::new(DegradationLevel::Normal)),
            shutdown: Arc::new(Notify::new()),
            handle: parking_lot::Mutex::new(None),
            results: Arc::new(parking_lot::Mutex::new(Vec::new())),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(DaemonConfig::default())
    }

    /// Start the daemon background loop
    pub fn start(&self) {
        let scheduler = self.scheduler.clone();
        let opportunistic = self.opportunistic.clone();
        let degradation_level = self.degradation_level.clone();
        let shutdown = self.shutdown.clone();
        let tick_interval = self.config.tick_interval;
        let opportunistic_enabled = self.config.opportunistic_enabled;
        let results = self.results.clone();

        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown.notified() => {
                        break;
                    }
                    _ = tokio::time::sleep(tick_interval) => {
                        let level = *degradation_level.lock();

                        // Run due scheduled tasks
                        let due = scheduler.due_tasks(level);
                        for task_id in due {
                            let task = DaemonTask::new(task_id);
                            let result = task.execute().await;
                            let status = result.status;
                            results.lock().push(result);
                            scheduler.record_result(task_id, status);
                        }

                        // Run opportunistic tasks if idle
                        if opportunistic_enabled && opportunistic.should_run() {
                            for opp_task in OpportunisticTask::all() {
                                let opp_result = opportunistic.execute(*opp_task).await;
                                results.lock().push(TaskResult {
                                    task: super::tasks::TaskId::GarbageCollection, // categorize under GC
                                    status: TaskStatus::Success,
                                    duration_ms: opp_result.duration_ms,
                                    message: opp_result.message,
                                    items_processed: 0,
                                });
                            }
                        }
                    }
                }
            }
        });

        *self.handle.lock() = Some(handle);
    }

    /// Stop the daemon gracefully
    pub fn stop(&self) {
        self.shutdown.notify_one();
    }

    /// Update the current degradation level
    pub fn set_degradation_level(&self, level: DegradationLevel) {
        *self.degradation_level.lock() = level;
    }

    /// Get the current degradation level
    pub fn degradation_level(&self) -> DegradationLevel {
        *self.degradation_level.lock()
    }

    /// Get a reference to the scheduler
    pub fn scheduler(&self) -> &TaskScheduler {
        &self.scheduler
    }

    /// Get a reference to the opportunistic runner
    pub fn opportunistic(&self) -> &OpportunisticRunner {
        &self.opportunistic
    }

    /// Get all recorded task results (drains the buffer)
    pub fn drain_results(&self) -> Vec<TaskResult> {
        std::mem::take(&mut *self.results.lock())
    }

    /// Get count of recorded results without draining
    pub fn result_count(&self) -> usize {
        self.results.lock().len()
    }

    /// Whether the daemon is running
    pub fn is_running(&self) -> bool {
        self.handle
            .lock()
            .as_ref()
            .map(|h| !h.is_finished())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daemon_config_defaults() {
        let config = DaemonConfig::default();
        assert_eq!(config.tick_interval, Duration::from_secs(10));
        assert!(config.opportunistic_enabled);
        assert_eq!(config.cpu_threshold, 10.0);
    }

    #[tokio::test]
    async fn test_daemon_starts_and_stops() {
        let daemon = ConsolidationDaemon::with_defaults();
        daemon.start();
        assert!(daemon.is_running());
        daemon.stop();
        tokio::time::sleep(Duration::from_millis(50)).await;
        // After stop notification, the loop should exit
    }

    #[tokio::test]
    async fn test_daemon_degradation_level() {
        let daemon = ConsolidationDaemon::with_defaults();
        assert_eq!(daemon.degradation_level(), DegradationLevel::Normal);
        daemon.set_degradation_level(DegradationLevel::Emergency);
        assert_eq!(daemon.degradation_level(), DegradationLevel::Emergency);
    }

    #[tokio::test]
    async fn test_daemon_runs_tasks() {
        let config = DaemonConfig {
            tick_interval: Duration::from_millis(50),
            opportunistic_enabled: false,
            ..Default::default()
        };
        let daemon = ConsolidationDaemon::new(config);

        // All tasks are initially due
        daemon.start();
        tokio::time::sleep(Duration::from_millis(200)).await;
        daemon.stop();
        tokio::time::sleep(Duration::from_millis(50)).await;

        let results = daemon.drain_results();
        // Should have executed at least 7 tasks (all initially due)
        assert!(
            results.len() >= 7,
            "Expected >=7 results, got {}",
            results.len()
        );
    }

    #[tokio::test]
    async fn test_daemon_respects_degradation() {
        let config = DaemonConfig {
            tick_interval: Duration::from_millis(50),
            opportunistic_enabled: false,
            ..Default::default()
        };
        let daemon = ConsolidationDaemon::new(config);

        // Set Emergency: only HealthCheck should run
        daemon.set_degradation_level(DegradationLevel::Emergency);
        daemon.start();
        tokio::time::sleep(Duration::from_millis(200)).await;
        daemon.stop();
        tokio::time::sleep(Duration::from_millis(50)).await;

        let results = daemon.drain_results();
        // In emergency, only HealthCheck runs
        assert!(!results.is_empty());
        // Should be fewer than 7 (not all tasks allowed)
        assert!(
            results.len() < 7,
            "Expected <7 results in Emergency, got {}",
            results.len()
        );
    }
}
