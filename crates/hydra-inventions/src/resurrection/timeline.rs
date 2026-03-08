//! Timeline — branching and forking from checkpoints.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::checkpoint::CheckpointId;

pub type BranchId = String;

/// A timeline branch from a checkpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineBranch {
    pub id: BranchId,
    pub name: String,
    pub fork_point: CheckpointId,
    pub created_at: String,
    pub checkpoints: Vec<CheckpointId>,
    pub active: bool,
}

/// Timeline manager — supports branching and merging
pub struct Timeline {
    branches: parking_lot::RwLock<HashMap<BranchId, TimelineBranch>>,
    current_branch: parking_lot::RwLock<BranchId>,
}

impl Timeline {
    pub fn new() -> Self {
        let main_branch = TimelineBranch {
            id: "main".into(),
            name: "main".into(),
            fork_point: String::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
            checkpoints: Vec::new(),
            active: true,
        };

        let mut branches = HashMap::new();
        branches.insert("main".into(), main_branch);

        Self {
            branches: parking_lot::RwLock::new(branches),
            current_branch: parking_lot::RwLock::new("main".into()),
        }
    }

    /// Create a new branch from a checkpoint
    pub fn branch(&self, name: &str, fork_point: &str) -> BranchId {
        let id = format!("branch-{}", uuid::Uuid::new_v4());
        let branch = TimelineBranch {
            id: id.clone(),
            name: name.into(),
            fork_point: fork_point.into(),
            created_at: chrono::Utc::now().to_rfc3339(),
            checkpoints: Vec::new(),
            active: true,
        };

        self.branches.write().insert(id.clone(), branch);
        id
    }

    /// Switch to a branch
    pub fn switch(&self, branch_id: &str) -> bool {
        let branches = self.branches.read();
        if branches.contains_key(branch_id) {
            *self.current_branch.write() = branch_id.into();
            true
        } else {
            false
        }
    }

    /// Add a checkpoint to the current branch
    pub fn add_checkpoint(&self, checkpoint_id: &str) {
        let current = self.current_branch.read().clone();
        if let Some(branch) = self.branches.write().get_mut(&current) {
            branch.checkpoints.push(checkpoint_id.into());
        }
    }

    /// Get current branch
    pub fn current(&self) -> TimelineBranch {
        let current = self.current_branch.read().clone();
        self.branches.read().get(&current).cloned().unwrap()
    }

    /// List all branches
    pub fn list_branches(&self) -> Vec<TimelineBranch> {
        self.branches.read().values().cloned().collect()
    }

    /// Merge a branch into the current branch (append checkpoints)
    pub fn merge(&self, source_branch_id: &str) -> Result<usize, String> {
        let current_id = self.current_branch.read().clone();
        let mut branches = self.branches.write();

        let source_checkpoints = branches
            .get(source_branch_id)
            .ok_or_else(|| format!("branch not found: {}", source_branch_id))?
            .checkpoints
            .clone();

        let merged_count = source_checkpoints.len();

        let current = branches
            .get_mut(&current_id)
            .ok_or_else(|| "current branch not found".to_string())?;

        current.checkpoints.extend(source_checkpoints);

        // Deactivate merged branch
        if let Some(source) = branches.get_mut(source_branch_id) {
            source.active = false;
        }

        Ok(merged_count)
    }

    /// Number of branches
    pub fn branch_count(&self) -> usize {
        self.branches.read().len()
    }
}

impl Default for Timeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeline_branch() {
        let timeline = Timeline::new();
        assert_eq!(timeline.branch_count(), 1); // main

        let branch_id = timeline.branch("experiment", "cp-1");
        assert_eq!(timeline.branch_count(), 2);

        assert!(timeline.switch(&branch_id));
        assert_eq!(timeline.current().name, "experiment");
    }

    #[test]
    fn test_timeline_merge() {
        let timeline = Timeline::new();
        let branch_id = timeline.branch("feature", "cp-0");

        // Add checkpoints to feature branch
        timeline.switch(&branch_id);
        timeline.add_checkpoint("cp-1");
        timeline.add_checkpoint("cp-2");

        // Switch back to main and merge
        timeline.switch("main");
        let merged = timeline.merge(&branch_id).unwrap();
        assert_eq!(merged, 2);
        assert_eq!(timeline.current().checkpoints.len(), 2);
    }
}
