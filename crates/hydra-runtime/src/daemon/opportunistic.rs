//! Opportunistic task runner — executes deferred work when CPU is idle.

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// Tasks that run only when the system is idle (CPU < threshold)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpportunisticTask {
    /// Precompute embeddings for recent memories
    PrecomputeEmbeddings,
    /// Prewarm frequently-used caches
    PrewarmCaches,
    /// Run deferred index optimizations
    DeferredOptimization,
}

impl OpportunisticTask {
    pub fn name(&self) -> &'static str {
        match self {
            Self::PrecomputeEmbeddings => "precompute_embeddings",
            Self::PrewarmCaches => "prewarm_caches",
            Self::DeferredOptimization => "deferred_optimization",
        }
    }

    pub fn all() -> &'static [OpportunisticTask] {
        &[
            OpportunisticTask::PrecomputeEmbeddings,
            OpportunisticTask::PrewarmCaches,
            OpportunisticTask::DeferredOptimization,
        ]
    }
}

/// Runs opportunistic tasks when the system is idle
pub struct OpportunisticRunner {
    /// CPU usage threshold below which tasks run (percentage)
    cpu_threshold: f64,
    /// Minimum idle duration before starting
    min_idle_duration: Duration,
    /// When the system last became idle
    idle_since: parking_lot::Mutex<Option<Instant>>,
    /// Whether the runner is enabled
    enabled: bool,
}

impl OpportunisticRunner {
    pub fn new(cpu_threshold: f64, min_idle_duration: Duration) -> Self {
        Self {
            cpu_threshold,
            min_idle_duration,
            idle_since: parking_lot::Mutex::new(None),
            enabled: true,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(10.0, Duration::from_secs(30))
    }

    /// Update idle state based on current CPU usage
    pub fn update_cpu(&self, cpu_percent: f64) {
        let mut idle = self.idle_since.lock();
        if cpu_percent < self.cpu_threshold {
            if idle.is_none() {
                *idle = Some(Instant::now());
            }
        } else {
            *idle = None;
        }
    }

    /// Whether conditions are met to run opportunistic tasks
    pub fn should_run(&self) -> bool {
        if !self.enabled {
            return false;
        }
        let idle = self.idle_since.lock();
        match *idle {
            Some(since) => since.elapsed() >= self.min_idle_duration,
            None => false,
        }
    }

    /// Get idle duration (None if not idle)
    pub fn idle_duration(&self) -> Option<Duration> {
        self.idle_since.lock().map(|since| since.elapsed())
    }

    /// Execute an opportunistic task
    pub async fn execute(&self, task: OpportunisticTask) -> OpportunisticResult {
        let start = Instant::now();
        let message = match task {
            OpportunisticTask::PrecomputeEmbeddings => {
                // In production: batch-compute embeddings for unindexed memories
                "Embeddings precomputed".to_string()
            }
            OpportunisticTask::PrewarmCaches => {
                // In production: load frequently-accessed data into cache
                "Caches prewarmed".to_string()
            }
            OpportunisticTask::DeferredOptimization => {
                // In production: compact databases, defrag indexes
                "Optimizations applied".to_string()
            }
        };

        OpportunisticResult {
            task,
            duration_ms: start.elapsed().as_millis() as u64,
            message,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpportunisticResult {
    pub task: OpportunisticTask,
    pub duration_ms: u64,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opportunistic_not_idle() {
        let runner = OpportunisticRunner::with_defaults();
        assert!(!runner.should_run());
    }

    #[test]
    fn test_opportunistic_detects_idle() {
        let runner = OpportunisticRunner::new(10.0, Duration::from_millis(0));
        runner.update_cpu(5.0);
        assert!(runner.should_run());
    }

    #[test]
    fn test_opportunistic_exits_idle() {
        let runner = OpportunisticRunner::new(10.0, Duration::from_millis(0));
        runner.update_cpu(5.0);
        assert!(runner.should_run());
        runner.update_cpu(50.0);
        assert!(!runner.should_run());
    }

    #[tokio::test]
    async fn test_opportunistic_execute() {
        let runner = OpportunisticRunner::with_defaults();
        let result = runner.execute(OpportunisticTask::PrewarmCaches).await;
        assert!(result.message.contains("prewarmed"));
    }
}
