//! Task scheduler — tracks intervals, next-run times, and backoff.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use super::tasks::{TaskId, TaskStatus};
use crate::degradation::DegradationLevel;

/// A scheduled task entry
#[derive(Debug, Clone)]
pub struct ScheduledTask {
    pub id: TaskId,
    pub interval: Duration,
    pub last_run: Option<Instant>,
    pub next_run: Instant,
    pub consecutive_failures: u32,
    pub max_backoff: Duration,
    pub enabled: bool,
}

impl ScheduledTask {
    pub fn new(id: TaskId) -> Self {
        Self {
            id,
            interval: id.default_interval(),
            last_run: None,
            next_run: Instant::now(),
            consecutive_failures: 0,
            max_backoff: Duration::from_secs(3600), // 1 hour max backoff
            enabled: true,
        }
    }

    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// Whether this task is due to run
    pub fn is_due(&self) -> bool {
        self.enabled && Instant::now() >= self.next_run
    }

    /// Record a successful run
    pub fn record_success(&mut self) {
        self.last_run = Some(Instant::now());
        self.consecutive_failures = 0;
        self.next_run = Instant::now() + self.interval;
    }

    /// Record a failed run with exponential backoff
    pub fn record_failure(&mut self) {
        self.last_run = Some(Instant::now());
        self.consecutive_failures += 1;
        let backoff = self.interval * 2u32.pow(self.consecutive_failures.min(6));
        self.next_run = Instant::now() + backoff.min(self.max_backoff);
    }

    /// Record a skipped run (degradation)
    pub fn record_skipped(&mut self) {
        // Don't count as failure, just reschedule
        self.next_run = Instant::now() + self.interval;
    }

    /// Time until next run
    pub fn time_until_next(&self) -> Duration {
        self.next_run.saturating_duration_since(Instant::now())
    }
}

/// Manages all scheduled tasks
pub struct TaskScheduler {
    tasks: parking_lot::Mutex<HashMap<TaskId, ScheduledTask>>,
}

impl TaskScheduler {
    pub fn new() -> Self {
        let mut tasks = HashMap::new();
        for &id in TaskId::all() {
            tasks.insert(id, ScheduledTask::new(id));
        }
        Self {
            tasks: parking_lot::Mutex::new(tasks),
        }
    }

    /// Get all tasks that are due to run, filtered by degradation level
    pub fn due_tasks(&self, degradation_level: DegradationLevel) -> Vec<TaskId> {
        self.tasks
            .lock()
            .values()
            .filter(|t| t.is_due() && t.id.allowed_at(degradation_level))
            .map(|t| t.id)
            .collect()
    }

    /// Record the result of a task execution
    pub fn record_result(&self, id: TaskId, status: TaskStatus) {
        if let Some(task) = self.tasks.lock().get_mut(&id) {
            match status {
                TaskStatus::Success | TaskStatus::PartialSuccess => task.record_success(),
                TaskStatus::Failed => task.record_failure(),
                TaskStatus::Skipped => task.record_skipped(),
            }
        }
    }

    /// Get a snapshot of all task states (for status reporting)
    pub fn snapshot(&self) -> Vec<TaskSnapshot> {
        self.tasks
            .lock()
            .values()
            .map(|t| TaskSnapshot {
                id: t.id,
                enabled: t.enabled,
                interval_secs: t.interval.as_secs(),
                last_run_ago_secs: t.last_run.map(|lr| lr.elapsed().as_secs()),
                next_run_in_secs: t.time_until_next().as_secs(),
                consecutive_failures: t.consecutive_failures,
            })
            .collect()
    }

    /// Override the interval for a specific task
    pub fn set_interval(&self, id: TaskId, interval: Duration) {
        if let Some(task) = self.tasks.lock().get_mut(&id) {
            task.interval = interval;
        }
    }

    /// Enable or disable a task
    pub fn set_enabled(&self, id: TaskId, enabled: bool) {
        if let Some(task) = self.tasks.lock().get_mut(&id) {
            task.enabled = enabled;
        }
    }

    /// Force a task to be due immediately
    pub fn trigger(&self, id: TaskId) {
        if let Some(task) = self.tasks.lock().get_mut(&id) {
            task.next_run = Instant::now();
        }
    }

    /// Get consecutive failure count for a task
    pub fn failure_count(&self, id: TaskId) -> u32 {
        self.tasks
            .lock()
            .get(&id)
            .map(|t| t.consecutive_failures)
            .unwrap_or(0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSnapshot {
    pub id: TaskId,
    pub enabled: bool,
    pub interval_secs: u64,
    pub last_run_ago_secs: Option<u64>,
    pub next_run_in_secs: u64,
    pub consecutive_failures: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_creates_all_tasks() {
        let scheduler = TaskScheduler::new();
        let snapshot = scheduler.snapshot();
        assert_eq!(snapshot.len(), 7);
    }

    #[test]
    fn test_scheduler_intervals() {
        let scheduler = TaskScheduler::new();
        let snapshot = scheduler.snapshot();
        let health = snapshot
            .iter()
            .find(|t| t.id == TaskId::HealthCheck)
            .unwrap();
        assert_eq!(health.interval_secs, 60);
        let reorg = snapshot
            .iter()
            .find(|t| t.id == TaskId::IndexReorg)
            .unwrap();
        assert_eq!(reorg.interval_secs, 86400);
    }

    #[test]
    fn test_tasks_initially_due() {
        let scheduler = TaskScheduler::new();
        let due = scheduler.due_tasks(DegradationLevel::Normal);
        // All tasks should be immediately due (next_run = now)
        assert_eq!(due.len(), 7);
    }

    #[test]
    fn test_success_reschedules() {
        let scheduler = TaskScheduler::new();
        scheduler.record_result(TaskId::HealthCheck, TaskStatus::Success);
        let due = scheduler.due_tasks(DegradationLevel::Normal);
        assert!(
            !due.contains(&TaskId::HealthCheck),
            "Should not be due right after success"
        );
    }

    #[test]
    fn test_failure_backoff() {
        let scheduler = TaskScheduler::new();
        scheduler.record_result(TaskId::HealthCheck, TaskStatus::Failed);
        assert_eq!(scheduler.failure_count(TaskId::HealthCheck), 1);
        scheduler.record_result(TaskId::HealthCheck, TaskStatus::Failed);
        assert_eq!(scheduler.failure_count(TaskId::HealthCheck), 2);
        // Success resets
        scheduler.record_result(TaskId::HealthCheck, TaskStatus::Success);
        assert_eq!(scheduler.failure_count(TaskId::HealthCheck), 0);
    }

    #[test]
    fn test_degradation_filters() {
        let scheduler = TaskScheduler::new();
        // Emergency: only HealthCheck
        let due = scheduler.due_tasks(DegradationLevel::Emergency);
        assert!(due.contains(&TaskId::HealthCheck));
        assert!(!due.contains(&TaskId::IndexReorg));
        assert!(!due.contains(&TaskId::PatternCrystallization));
    }

    #[test]
    fn test_trigger_forces_due() {
        let scheduler = TaskScheduler::new();
        scheduler.record_result(TaskId::HealthCheck, TaskStatus::Success);
        assert!(!scheduler
            .due_tasks(DegradationLevel::Normal)
            .contains(&TaskId::HealthCheck));
        scheduler.trigger(TaskId::HealthCheck);
        assert!(scheduler
            .due_tasks(DegradationLevel::Normal)
            .contains(&TaskId::HealthCheck));
    }

    #[test]
    fn test_disable_task() {
        let scheduler = TaskScheduler::new();
        scheduler.set_enabled(TaskId::IndexReorg, false);
        let due = scheduler.due_tasks(DegradationLevel::Normal);
        assert!(!due.contains(&TaskId::IndexReorg));
    }
}
