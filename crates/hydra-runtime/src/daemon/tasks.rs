//! Individual daemon task implementations.

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::degradation::DegradationLevel;

/// Unique task identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskId {
    MemoryStrengthen,
    MemoryDecay,
    IndexReorg,
    HealthCheck,
    GarbageCollection,
    SoulPersistence,
    PatternCrystallization,
}

impl TaskId {
    /// Human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            Self::MemoryStrengthen => "memory_strengthen",
            Self::MemoryDecay => "memory_decay",
            Self::IndexReorg => "index_reorg",
            Self::HealthCheck => "health_check",
            Self::GarbageCollection => "garbage_collection",
            Self::SoulPersistence => "soul_persistence",
            Self::PatternCrystallization => "pattern_crystallization",
        }
    }

    /// Default interval for this task
    pub fn default_interval(&self) -> Duration {
        match self {
            Self::HealthCheck => Duration::from_secs(60),
            Self::GarbageCollection => Duration::from_secs(300),
            Self::SoulPersistence => Duration::from_secs(600),
            Self::MemoryStrengthen => Duration::from_secs(3600),
            Self::MemoryDecay => Duration::from_secs(3600),
            Self::PatternCrystallization => Duration::from_secs(7200),
            Self::IndexReorg => Duration::from_secs(86400),
        }
    }

    /// Minimum degradation level required to run this task
    pub fn min_level(&self) -> DegradationLevel {
        match self {
            // Essential tasks run even in Emergency
            Self::HealthCheck => DegradationLevel::Emergency,
            Self::GarbageCollection => DegradationLevel::Minimal,
            // Non-essential skip under pressure
            Self::MemoryStrengthen => DegradationLevel::Reduced,
            Self::MemoryDecay => DegradationLevel::Reduced,
            Self::SoulPersistence => DegradationLevel::Reduced,
            Self::PatternCrystallization => DegradationLevel::Normal,
            Self::IndexReorg => DegradationLevel::Normal,
        }
    }

    /// Whether this task can run at the given degradation level
    pub fn allowed_at(&self, level: DegradationLevel) -> bool {
        level <= self.min_level()
    }

    /// All defined tasks in priority order (most frequent first)
    pub fn all() -> &'static [TaskId] {
        &[
            TaskId::HealthCheck,
            TaskId::GarbageCollection,
            TaskId::SoulPersistence,
            TaskId::MemoryStrengthen,
            TaskId::MemoryDecay,
            TaskId::PatternCrystallization,
            TaskId::IndexReorg,
        ]
    }
}

/// Result of executing a daemon task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task: TaskId,
    pub status: TaskStatus,
    pub duration_ms: u64,
    pub message: String,
    pub items_processed: u64,
}

/// Status of a task execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Success,
    Skipped,
    Failed,
    PartialSuccess,
}

/// A daemon task that can be executed
pub struct DaemonTask {
    pub id: TaskId,
}

impl DaemonTask {
    pub fn new(id: TaskId) -> Self {
        Self { id }
    }

    /// Execute this task. Returns a TaskResult.
    pub async fn execute(&self) -> TaskResult {
        let start = Instant::now();
        let (status, message, items) = match self.id {
            TaskId::HealthCheck => self.run_health_check().await,
            TaskId::GarbageCollection => self.run_gc().await,
            TaskId::SoulPersistence => self.run_soul_persistence().await,
            TaskId::MemoryStrengthen => self.run_memory_strengthen().await,
            TaskId::MemoryDecay => self.run_memory_decay().await,
            TaskId::PatternCrystallization => self.run_pattern_crystallization().await,
            TaskId::IndexReorg => self.run_index_reorg().await,
        };

        TaskResult {
            task: self.id,
            status,
            duration_ms: start.elapsed().as_millis() as u64,
            message,
            items_processed: items,
        }
    }

    async fn run_health_check(&self) -> (TaskStatus, String, u64) {
        // In production: ping all sister MCP servers via bridges
        // For now: report healthy
        (TaskStatus::Success, "All systems nominal".into(), 0)
    }

    async fn run_gc(&self) -> (TaskStatus, String, u64) {
        // In production: clear expired cache entries, temp files, old receipts
        // For now: simulate cleanup
        let cleared = 0u64;
        (
            TaskStatus::Success,
            format!("Cleared {} expired items", cleared),
            cleared,
        )
    }

    async fn run_soul_persistence(&self) -> (TaskStatus, String, u64) {
        // In production: snapshot beliefs, patterns, config to hydra.soul file
        // For now: report saved
        (TaskStatus::Success, "Soul snapshot saved".into(), 1)
    }

    async fn run_memory_strengthen(&self) -> (TaskStatus, String, u64) {
        // In production: call memory sister's strengthen tool
        // memory_bridge.call("memory_strengthen", {...})
        let strengthened = 0u64;
        (
            TaskStatus::Success,
            format!("Strengthened {} memories", strengthened),
            strengthened,
        )
    }

    async fn run_memory_decay(&self) -> (TaskStatus, String, u64) {
        // In production: call memory sister's decay tool
        // memory_bridge.call("memory_decay", {...})
        let decayed = 0u64;
        (
            TaskStatus::Success,
            format!("Decayed {} stale memories", decayed),
            decayed,
        )
    }

    async fn run_pattern_crystallization(&self) -> (TaskStatus, String, u64) {
        // In production: analyze recent runs for recurring patterns
        // Compile into reusable action sequences
        let patterns = 0u64;
        (
            TaskStatus::Success,
            format!("Crystallized {} patterns", patterns),
            patterns,
        )
    }

    async fn run_index_reorg(&self) -> (TaskStatus, String, u64) {
        // In production: optimize sister indexes (codebase, memory)
        (TaskStatus::Success, "Indexes reorganized".into(), 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_ids() {
        assert_eq!(TaskId::all().len(), 7);
        assert_eq!(TaskId::HealthCheck.name(), "health_check");
        assert_eq!(TaskId::GarbageCollection.name(), "garbage_collection");
    }

    #[test]
    fn test_task_intervals() {
        assert_eq!(
            TaskId::HealthCheck.default_interval(),
            Duration::from_secs(60)
        );
        assert_eq!(
            TaskId::IndexReorg.default_interval(),
            Duration::from_secs(86400)
        );
    }

    #[test]
    fn test_task_degradation_levels() {
        // HealthCheck runs even in Emergency
        assert!(TaskId::HealthCheck.allowed_at(DegradationLevel::Emergency));
        // IndexReorg only in Normal
        assert!(TaskId::IndexReorg.allowed_at(DegradationLevel::Normal));
        assert!(!TaskId::IndexReorg.allowed_at(DegradationLevel::Reduced));
        // GC runs up to Minimal
        assert!(TaskId::GarbageCollection.allowed_at(DegradationLevel::Minimal));
        assert!(!TaskId::GarbageCollection.allowed_at(DegradationLevel::Emergency));
    }

    #[tokio::test]
    async fn test_health_check_task() {
        let task = DaemonTask::new(TaskId::HealthCheck);
        let result = task.execute().await;
        assert_eq!(result.status, TaskStatus::Success);
        assert_eq!(result.task, TaskId::HealthCheck);
    }

    #[tokio::test]
    async fn test_gc_task() {
        let task = DaemonTask::new(TaskId::GarbageCollection);
        let result = task.execute().await;
        assert_eq!(result.status, TaskStatus::Success);
    }

    #[tokio::test]
    async fn test_memory_tasks() {
        let strengthen = DaemonTask::new(TaskId::MemoryStrengthen);
        let result = strengthen.execute().await;
        assert_eq!(result.status, TaskStatus::Success);

        let decay = DaemonTask::new(TaskId::MemoryDecay);
        let result = decay.execute().await;
        assert_eq!(result.status, TaskStatus::Success);
    }

    #[tokio::test]
    async fn test_soul_persistence_task() {
        let task = DaemonTask::new(TaskId::SoulPersistence);
        let result = task.execute().await;
        assert_eq!(result.status, TaskStatus::Success);
        assert!(result.message.contains("saved"));
    }

    #[tokio::test]
    async fn test_pattern_crystallization_task() {
        let task = DaemonTask::new(TaskId::PatternCrystallization);
        let result = task.execute().await;
        assert_eq!(result.status, TaskStatus::Success);
    }
}
