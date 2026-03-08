//! ForkPoint — create branches at decision points.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A point where execution forks into branches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkPoint {
    pub id: String,
    pub description: String,
    pub state: HashMap<String, serde_json::Value>,
    pub branches: Vec<ForkBranch>,
    pub created_at: String,
    pub max_branches: usize,
}

/// A branch from a fork point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkBranch {
    pub id: String,
    pub fork_id: String,
    pub name: String,
    pub actions: Vec<String>,
    pub status: BranchStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BranchStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl ForkPoint {
    pub fn new(description: &str, state: HashMap<String, serde_json::Value>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            description: description.into(),
            state,
            branches: Vec::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
            max_branches: 5,
        }
    }

    pub fn with_max_branches(mut self, max: usize) -> Self {
        self.max_branches = max;
        self
    }

    /// Add a branch to this fork point
    pub fn add_branch(&mut self, name: &str, actions: Vec<String>) -> Result<String, String> {
        if self.branches.len() >= self.max_branches {
            return Err(format!("max branches ({}) reached", self.max_branches));
        }

        let branch = ForkBranch {
            id: uuid::Uuid::new_v4().to_string(),
            fork_id: self.id.clone(),
            name: name.into(),
            actions,
            status: BranchStatus::Pending,
        };
        let id = branch.id.clone();
        self.branches.push(branch);
        Ok(id)
    }

    /// Get active branch count
    pub fn active_branches(&self) -> usize {
        self.branches
            .iter()
            .filter(|b| matches!(b.status, BranchStatus::Pending | BranchStatus::Running))
            .count()
    }

    /// Cancel all pending branches
    pub fn cancel_pending(&mut self) -> usize {
        let mut cancelled = 0;
        for branch in &mut self.branches {
            if branch.status == BranchStatus::Pending {
                branch.status = BranchStatus::Cancelled;
                cancelled += 1;
            }
        }
        cancelled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fork_creation() {
        let mut fork = ForkPoint::new("decision point", HashMap::new());
        let id1 = fork
            .add_branch("approach_a", vec!["read".into(), "process".into()])
            .unwrap();
        let id2 = fork
            .add_branch("approach_b", vec!["fetch".into(), "transform".into()])
            .unwrap();

        assert_eq!(fork.branches.len(), 2);
        assert_ne!(id1, id2);
        assert_eq!(fork.active_branches(), 2);
    }

    #[test]
    fn test_fork_limits() {
        let mut fork = ForkPoint::new("limited", HashMap::new()).with_max_branches(2);
        fork.add_branch("a", vec![]).unwrap();
        fork.add_branch("b", vec![]).unwrap();
        assert!(fork.add_branch("c", vec![]).is_err());
    }

    #[test]
    fn test_fork_cleanup() {
        let mut fork = ForkPoint::new("cleanup", HashMap::new());
        fork.add_branch("a", vec![]).unwrap();
        fork.add_branch("b", vec![]).unwrap();
        fork.branches[0].status = BranchStatus::Running;

        let cancelled = fork.cancel_pending();
        assert_eq!(cancelled, 1);
        assert_eq!(fork.active_branches(), 1); // only the running one
    }
}
