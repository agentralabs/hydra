//! ShadowExecutor — parallel execution for validation.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A shadow execution run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowRun {
    pub id: String,
    pub description: String,
    pub input: serde_json::Value,
    pub status: ShadowStatus,
    pub created_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShadowStatus {
    Pending,
    Running,
    Completed,
    Aborted,
    Failed,
}

/// Result of a shadow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowResult {
    pub run_id: String,
    pub success: bool,
    pub outputs: HashMap<String, serde_json::Value>,
    pub duration_ms: u64,
    pub tokens_used: u64,
    pub safe: bool,
}

/// Executes actions in a shadow timeline for validation
pub struct ShadowExecutor {
    runs: parking_lot::RwLock<Vec<(ShadowRun, Option<ShadowResult>)>>,
    _max_parallel: usize,
}

impl ShadowExecutor {
    pub fn new(max_parallel: usize) -> Self {
        Self {
            runs: parking_lot::RwLock::new(Vec::new()),
            _max_parallel: max_parallel,
        }
    }

    /// Start a shadow execution
    pub fn execute(&self, description: &str, input: serde_json::Value) -> ShadowRun {
        let run = ShadowRun {
            id: uuid::Uuid::new_v4().to_string(),
            description: description.into(),
            input: input.clone(),
            status: ShadowStatus::Running,
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        // Simulate execution (real impl would run in parallel)
        let result = ShadowResult {
            run_id: run.id.clone(),
            success: true,
            outputs: HashMap::from([("shadow_output".into(), input)]),
            duration_ms: 50,
            tokens_used: 0,
            safe: true,
        };

        let mut completed_run = run.clone();
        completed_run.status = ShadowStatus::Completed;

        self.runs.write().push((completed_run, Some(result)));
        run
    }

    /// Abort a shadow run
    pub fn abort(&self, run_id: &str) -> bool {
        let mut runs = self.runs.write();
        if let Some((run, _)) = runs.iter_mut().find(|(r, _)| r.id == run_id) {
            run.status = ShadowStatus::Aborted;
            true
        } else {
            false
        }
    }

    /// Get result of a shadow run
    pub fn result(&self, run_id: &str) -> Option<ShadowResult> {
        self.runs
            .read()
            .iter()
            .find(|(r, _)| r.id == run_id)
            .and_then(|(_, result)| result.clone())
    }

    /// Get metrics
    pub fn metrics(&self) -> ShadowMetrics {
        let runs = self.runs.read();
        let total = runs.len();
        let completed = runs
            .iter()
            .filter(|(r, _)| r.status == ShadowStatus::Completed)
            .count();
        let safe = runs
            .iter()
            .filter(|(_, r)| r.as_ref().map(|r| r.safe).unwrap_or(false))
            .count();

        ShadowMetrics {
            total,
            completed,
            safe,
        }
    }

    /// Number of active runs
    pub fn active_count(&self) -> usize {
        self.runs
            .read()
            .iter()
            .filter(|(r, _)| r.status == ShadowStatus::Running)
            .count()
    }
}

#[derive(Debug, Clone)]
pub struct ShadowMetrics {
    pub total: usize,
    pub completed: usize,
    pub safe: usize,
}

impl Default for ShadowExecutor {
    fn default() -> Self {
        Self::new(4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shadow_execution() {
        let executor = ShadowExecutor::default();
        let run = executor.execute("test action", serde_json::json!({"input": "test"}));

        let result = executor.result(&run.id).unwrap();
        assert!(result.success);
        assert!(result.safe);
    }

    #[test]
    fn test_shadow_abort() {
        let executor = ShadowExecutor::default();
        let run = executor.execute("test", serde_json::json!(null));
        assert!(executor.abort(&run.id));
    }

    #[test]
    fn test_shadow_metrics() {
        let executor = ShadowExecutor::default();
        executor.execute("a", serde_json::json!(1));
        executor.execute("b", serde_json::json!(2));

        let metrics = executor.metrics();
        assert_eq!(metrics.total, 2);
        assert_eq!(metrics.completed, 2);
        assert_eq!(metrics.safe, 2);
    }
}
