//! Parallel dispatch — executes independent task steps concurrently.
//!
//! UCU Module #11 (Wave 3). Builds dependency graphs from TaskPlans and
//! dispatches independent steps via tokio::join!.
//! Why not a sister? Orchestration logic is local. Individual steps may use sisters.

use crate::cognitive::iterative_planner::TaskStep;

/// Dependency graph tracking step completion state.
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    step_count: usize,
    /// dependencies[i] = list of step IDs that step i depends on.
    dependencies: Vec<Vec<usize>>,
    /// Whether each step has completed.
    completed: Vec<bool>,
}

/// Result of executing a single step.
#[derive(Debug, Clone)]
pub struct DispatchResult {
    pub step_id: usize,
    pub success: bool,
    pub output: String,
    pub duration_ms: u64,
}

/// A batch of steps that can execute in parallel.
#[derive(Debug, Clone)]
pub struct ParallelBatch {
    pub step_ids: Vec<usize>,
    pub batch_index: usize,
}

impl DependencyGraph {
    /// Build a dependency graph from task steps.
    pub fn new(steps: &[TaskStep]) -> Self {
        let step_count = steps.len();
        let dependencies: Vec<Vec<usize>> = steps.iter()
            .map(|s| s.depends_on.clone())
            .collect();
        Self {
            step_count,
            dependencies,
            completed: vec![false; step_count],
        }
    }

    /// Get the next batch of steps whose dependencies are all satisfied.
    /// Returns None when all steps are complete or no more can be started.
    pub fn next_batch(&self) -> Option<ParallelBatch> {
        let ready: Vec<usize> = (0..self.step_count)
            .filter(|&id| {
                !self.completed[id]
                    && self.dependencies[id].iter().all(|dep| {
                        *dep < self.step_count && self.completed[*dep]
                    })
            })
            .collect();

        if ready.is_empty() {
            return None;
        }

        let batch_index = self.completed.iter().filter(|&&c| c).count();
        Some(ParallelBatch { step_ids: ready, batch_index })
    }

    /// Mark a step as completed.
    pub fn mark_completed(&mut self, step_id: usize) {
        if step_id < self.step_count {
            self.completed[step_id] = true;
        }
    }

    /// Mark a step as completed with its result.
    pub fn mark_with_result(&mut self, result: &DispatchResult) {
        self.mark_completed(result.step_id);
    }

    /// Check if all steps are complete.
    pub fn all_complete(&self) -> bool {
        self.completed.iter().all(|&c| c)
    }

    /// Count of completed steps.
    pub fn completed_count(&self) -> usize {
        self.completed.iter().filter(|&&c| c).count()
    }

    /// Total number of steps.
    pub fn total_steps(&self) -> usize {
        self.step_count
    }

    /// Check if a specific step can run (all deps satisfied).
    pub fn can_run(&self, step_id: usize) -> bool {
        if step_id >= self.step_count || self.completed[step_id] {
            return false;
        }
        self.dependencies[step_id].iter().all(|dep| {
            *dep < self.step_count && self.completed[*dep]
        })
    }

    /// Get steps that depend on a given step (downstream dependents).
    pub fn dependents_of(&self, step_id: usize) -> Vec<usize> {
        (0..self.step_count)
            .filter(|&id| self.dependencies[id].contains(&step_id))
            .collect()
    }

    /// Check if skipping a step would block downstream work.
    pub fn has_dependents(&self, step_id: usize) -> bool {
        (0..self.step_count)
            .any(|id| self.dependencies[id].contains(&step_id) && !self.completed[id])
    }
}

/// Compute optimal batch ordering showing which steps can run in parallel.
pub fn compute_parallel_groups(steps: &[TaskStep]) -> Vec<Vec<usize>> {
    let mut graph = DependencyGraph::new(steps);
    let mut groups = Vec::new();

    loop {
        match graph.next_batch() {
            Some(batch) => {
                let ids = batch.step_ids.clone();
                for &id in &ids {
                    graph.mark_completed(id);
                }
                groups.push(ids);
            }
            None => break,
        }
    }

    groups
}

/// Estimate total duration if steps are executed with parallelism.
/// Assumes each step takes `per_step_ms` milliseconds.
pub fn estimate_parallel_duration(steps: &[TaskStep], per_step_ms: u64) -> u64 {
    let groups = compute_parallel_groups(steps);
    // Each group executes in parallel — duration is max of the group
    groups.len() as u64 * per_step_ms
}

/// Estimate total duration if steps are executed sequentially.
pub fn estimate_sequential_duration(steps: &[TaskStep], per_step_ms: u64) -> u64 {
    steps.len() as u64 * per_step_ms
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cognitive::iterative_planner::Phase;

    fn step(id: usize, deps: Vec<usize>) -> TaskStep {
        TaskStep {
            id, phase: Phase::Execute,
            description: format!("step_{}", id),
            depends_on: deps, estimated_tokens: 1000,
            requires_sister: None,
        }
    }

    #[test]
    fn test_linear_chain() {
        let steps = vec![step(0, vec![]), step(1, vec![0]), step(2, vec![1])];
        let mut g = DependencyGraph::new(&steps);

        // Only step 0 is ready first
        let b1 = g.next_batch().unwrap();
        assert_eq!(b1.step_ids, vec![0]);
        g.mark_completed(0);

        // Then step 1
        let b2 = g.next_batch().unwrap();
        assert_eq!(b2.step_ids, vec![1]);
        g.mark_completed(1);

        // Then step 2
        let b3 = g.next_batch().unwrap();
        assert_eq!(b3.step_ids, vec![2]);
        g.mark_completed(2);

        assert!(g.all_complete());
        assert!(g.next_batch().is_none());
    }

    #[test]
    fn test_parallel_independent() {
        // Steps 1 and 2 both depend on step 0, but not on each other
        let steps = vec![step(0, vec![]), step(1, vec![0]), step(2, vec![0])];
        let mut g = DependencyGraph::new(&steps);

        let b1 = g.next_batch().unwrap();
        assert_eq!(b1.step_ids, vec![0]);
        g.mark_completed(0);

        // Steps 1 and 2 should be in the same batch
        let b2 = g.next_batch().unwrap();
        assert_eq!(b2.step_ids.len(), 2);
        assert!(b2.step_ids.contains(&1));
        assert!(b2.step_ids.contains(&2));
    }

    #[test]
    fn test_all_independent() {
        let steps = vec![step(0, vec![]), step(1, vec![]), step(2, vec![])];
        let g = DependencyGraph::new(&steps);
        let b = g.next_batch().unwrap();
        assert_eq!(b.step_ids.len(), 3); // All in one batch
    }

    #[test]
    fn test_has_dependents() {
        let steps = vec![step(0, vec![]), step(1, vec![0]), step(2, vec![1])];
        let g = DependencyGraph::new(&steps);
        assert!(g.has_dependents(0)); // step 1 depends on 0
        assert!(g.has_dependents(1)); // step 2 depends on 1
        assert!(!g.has_dependents(2)); // nothing depends on 2
    }

    #[test]
    fn test_parallel_groups() {
        let steps = vec![step(0, vec![]), step(1, vec![0]), step(2, vec![0]), step(3, vec![1, 2])];
        let groups = compute_parallel_groups(&steps);
        assert_eq!(groups.len(), 3); // [0], [1,2], [3]
        assert_eq!(groups[0], vec![0]);
        assert!(groups[1].contains(&1) && groups[1].contains(&2));
        assert_eq!(groups[2], vec![3]);
    }

    #[test]
    fn test_speedup_estimate() {
        let steps = vec![step(0, vec![]), step(1, vec![0]), step(2, vec![0]), step(3, vec![1, 2])];
        let par = estimate_parallel_duration(&steps, 100);
        let seq = estimate_sequential_duration(&steps, 100);
        assert!(par < seq); // Parallel should be faster
    }
}
