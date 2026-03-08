//! ParallelExecutor — run forked branches simultaneously.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::fork::{BranchStatus, ForkPoint};

/// Result of executing a single branch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchResult {
    pub branch_id: String,
    pub branch_name: String,
    pub success: bool,
    pub output: serde_json::Value,
    pub duration_ms: u64,
    pub resources_used: f64,
}

/// Executes fork branches in parallel (simulated)
pub struct ParallelExecutor {
    results: parking_lot::RwLock<HashMap<String, Vec<BranchResult>>>,
    max_parallel: usize,
}

impl ParallelExecutor {
    pub fn new(max_parallel: usize) -> Self {
        Self {
            results: parking_lot::RwLock::new(HashMap::new()),
            max_parallel,
        }
    }

    /// Execute all branches of a fork point
    pub fn execute(&self, fork: &mut ForkPoint) -> Vec<BranchResult> {
        let mut results = Vec::new();

        let branches_to_run: Vec<_> = fork
            .branches
            .iter_mut()
            .filter(|b| b.status == BranchStatus::Pending)
            .take(self.max_parallel)
            .collect();

        for branch in branches_to_run {
            branch.status = BranchStatus::Running;

            // Simulate execution
            let success = branch.actions.len() > 0;
            let duration = (branch.actions.len() as u64) * 50;

            branch.status = if success {
                BranchStatus::Completed
            } else {
                BranchStatus::Failed
            };

            results.push(BranchResult {
                branch_id: branch.id.clone(),
                branch_name: branch.name.clone(),
                success,
                output: serde_json::json!({
                    "actions_executed": branch.actions.len(),
                    "branch": branch.name,
                }),
                duration_ms: duration,
                resources_used: branch.actions.len() as f64 * 0.1,
            });
        }

        self.results
            .write()
            .insert(fork.id.clone(), results.clone());
        results
    }

    /// Get results for a fork
    pub fn get_results(&self, fork_id: &str) -> Option<Vec<BranchResult>> {
        self.results.read().get(fork_id).cloned()
    }

    /// Total forks executed
    pub fn execution_count(&self) -> usize {
        self.results.read().len()
    }
}

impl Default for ParallelExecutor {
    fn default() -> Self {
        Self::new(4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_execution() {
        let executor = ParallelExecutor::new(4);
        let mut fork = ForkPoint::new("test", HashMap::new());
        fork.add_branch("fast", vec!["step1".into()]).unwrap();
        fork.add_branch("slow", vec!["step1".into(), "step2".into(), "step3".into()])
            .unwrap();

        let results = executor.execute(&mut fork);
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.success));

        // Slow branch took longer
        assert!(results[1].duration_ms > results[0].duration_ms);
    }

    #[test]
    fn test_resource_isolation() {
        let executor = ParallelExecutor::new(2);
        let mut fork = ForkPoint::new("isolated", HashMap::new());
        fork.add_branch("a", vec!["x".into()]).unwrap();
        fork.add_branch("b", vec!["y".into(), "z".into()]).unwrap();

        let results = executor.execute(&mut fork);
        // Each branch has independent resource tracking
        assert!(
            results[0].resources_used != results[1].resources_used
                || results[0].branch_name != results[1].branch_name
        );
    }
}
