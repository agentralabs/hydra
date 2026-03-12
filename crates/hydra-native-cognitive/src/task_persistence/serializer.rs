//! Serialize/deserialize task checkpoints to disk.
//!
//! Each checkpoint is a JSON file at `~/.hydra/checkpoints/{task_id}.json`.
//! Files are atomically written (write temp → rename) to survive crashes.

use std::path::PathBuf;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Type of long-running task being checkpointed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskType {
    ProjectExec,
    SelfImplement,
    AgentSwarm,
    RemoteExec,
    Custom(String),
}

impl TaskType {
    pub fn label(&self) -> &str {
        match self {
            Self::ProjectExec => "Project Execution",
            Self::SelfImplement => "Self-Implement",
            Self::AgentSwarm => "Agent Swarm",
            Self::RemoteExec => "Remote Execution",
            Self::Custom(s) => s.as_str(),
        }
    }
}

/// A checkpoint capturing the state of a long-running task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCheckpoint {
    pub task_id: String,
    pub task_type: TaskType,
    pub started_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub phase: String,
    pub progress: f32,
    pub state: serde_json::Value,
    pub completed_steps: Vec<String>,
    pub remaining_steps: Vec<String>,
}

impl TaskCheckpoint {
    /// Create a new checkpoint with initial state.
    pub fn new(task_id: &str, task_type: TaskType, all_steps: &[&str]) -> Self {
        Self {
            task_id: task_id.to_string(),
            task_type,
            started_at: Utc::now(),
            last_updated: Utc::now(),
            phase: all_steps.first().copied().unwrap_or("init").to_string(),
            progress: 0.0,
            state: serde_json::json!({}),
            completed_steps: Vec::new(),
            remaining_steps: all_steps.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Advance to the next phase after completing the current step.
    pub fn advance(&mut self, completed_step: &str, new_phase: &str, state: serde_json::Value) {
        self.completed_steps.push(completed_step.to_string());
        self.remaining_steps.retain(|s| s != completed_step);
        self.phase = new_phase.to_string();
        self.state = state;
        self.last_updated = Utc::now();
        let total = self.completed_steps.len() + self.remaining_steps.len();
        if total > 0 {
            self.progress = self.completed_steps.len() as f32 / total as f32;
        }
    }

    /// Mark this task as fully complete.
    pub fn mark_complete(&mut self) {
        self.progress = 1.0;
        self.phase = "complete".to_string();
        self.remaining_steps.clear();
        self.last_updated = Utc::now();
    }

    /// Check if this checkpoint is stale (older than given days).
    pub fn is_stale(&self, max_age_days: i64) -> bool {
        let age = Utc::now().signed_duration_since(self.last_updated);
        age.num_days() > max_age_days
    }
}

/// Persists task checkpoints to disk.
pub struct TaskPersister {
    checkpoint_dir: PathBuf,
}

impl TaskPersister {
    pub fn new() -> Self {
        Self {
            checkpoint_dir: super::checkpoint_dir(),
        }
    }

    /// Create with a custom directory (for tests).
    pub fn with_dir(dir: PathBuf) -> Self {
        Self { checkpoint_dir: dir }
    }

    /// Ensure the checkpoint directory exists.
    fn ensure_dir(&self) -> Result<(), String> {
        std::fs::create_dir_all(&self.checkpoint_dir)
            .map_err(|e| format!("Failed to create checkpoint dir: {}", e))
    }

    fn checkpoint_path(&self, task_id: &str) -> PathBuf {
        // Sanitize task_id for filesystem safety
        let safe_id: String = task_id
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect();
        self.checkpoint_dir.join(format!("{}.json", safe_id))
    }

    /// Save a checkpoint to disk (atomic write via temp file + rename).
    pub fn save(&self, checkpoint: &TaskCheckpoint) -> Result<(), String> {
        self.ensure_dir()?;
        let path = self.checkpoint_path(&checkpoint.task_id);
        let json = serde_json::to_string_pretty(checkpoint)
            .map_err(|e| format!("Serialize error: {}", e))?;

        // Atomic: write to .tmp, then rename
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, &json)
            .map_err(|e| format!("Write error: {}", e))?;
        std::fs::rename(&tmp, &path)
            .map_err(|e| format!("Rename error: {}", e))?;
        Ok(())
    }

    /// Load a specific checkpoint by task ID.
    pub fn load(&self, task_id: &str) -> Result<TaskCheckpoint, String> {
        let path = self.checkpoint_path(task_id);
        let json = std::fs::read_to_string(&path)
            .map_err(|e| format!("Read error: {}", e))?;
        serde_json::from_str(&json)
            .map_err(|e| format!("Deserialize error: {}", e))
    }

    /// List all incomplete checkpoints (progress < 1.0).
    pub fn list_incomplete(&self) -> Result<Vec<TaskCheckpoint>, String> {
        self.ensure_dir()?;
        let mut results = Vec::new();
        let entries = std::fs::read_dir(&self.checkpoint_dir)
            .map_err(|e| format!("Read dir error: {}", e))?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json") {
                if let Ok(json) = std::fs::read_to_string(&path) {
                    if let Ok(cp) = serde_json::from_str::<TaskCheckpoint>(&json) {
                        if cp.progress < 1.0 {
                            results.push(cp);
                        }
                    }
                }
            }
        }
        // Sort by last_updated descending (most recent first)
        results.sort_by(|a, b| b.last_updated.cmp(&a.last_updated));
        Ok(results)
    }

    /// Mark a task as complete (removes the checkpoint file).
    pub fn complete(&self, task_id: &str) -> Result<(), String> {
        let path = self.checkpoint_path(task_id);
        if path.exists() {
            std::fs::remove_file(&path)
                .map_err(|e| format!("Remove error: {}", e))?;
        }
        Ok(())
    }

    /// Clean up checkpoints older than `max_age_days`. Returns count removed.
    pub fn cleanup_old(&self, max_age_days: i64) -> Result<usize, String> {
        self.ensure_dir()?;
        let mut removed = 0;
        let entries = std::fs::read_dir(&self.checkpoint_dir)
            .map_err(|e| format!("Read dir error: {}", e))?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json") {
                if let Ok(json) = std::fs::read_to_string(&path) {
                    if let Ok(cp) = serde_json::from_str::<TaskCheckpoint>(&json) {
                        if cp.is_stale(max_age_days) {
                            let _ = std::fs::remove_file(&path);
                            removed += 1;
                        }
                    }
                }
            }
        }
        Ok(removed)
    }
}

impl Default for TaskPersister {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_persister() -> (TaskPersister, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let p = TaskPersister::with_dir(dir.path().to_path_buf());
        (p, dir)
    }

    #[test]
    fn test_save_and_load_checkpoint() {
        let (p, _dir) = temp_persister();
        let cp = TaskCheckpoint::new("test-1", TaskType::ProjectExec, &["clone", "test"]);
        p.save(&cp).unwrap();
        let loaded = p.load("test-1").unwrap();
        assert_eq!(loaded.task_id, "test-1");
        assert_eq!(loaded.task_type, TaskType::ProjectExec);
        assert_eq!(loaded.remaining_steps, vec!["clone", "test"]);
    }

    #[test]
    fn test_list_incomplete() {
        let (p, _dir) = temp_persister();
        let cp1 = TaskCheckpoint::new("a", TaskType::ProjectExec, &["clone"]);
        let mut cp2 = TaskCheckpoint::new("b", TaskType::SelfImplement, &["spec"]);
        cp2.mark_complete();
        p.save(&cp1).unwrap();
        p.save(&cp2).unwrap();
        let incomplete = p.list_incomplete().unwrap();
        assert_eq!(incomplete.len(), 1);
        assert_eq!(incomplete[0].task_id, "a");
    }

    #[test]
    fn test_complete_removes_checkpoint() {
        let (p, _dir) = temp_persister();
        let cp = TaskCheckpoint::new("rm-me", TaskType::ProjectExec, &["clone"]);
        p.save(&cp).unwrap();
        assert!(p.load("rm-me").is_ok());
        p.complete("rm-me").unwrap();
        assert!(p.load("rm-me").is_err());
    }

    #[test]
    fn test_cleanup_old() {
        let (p, _dir) = temp_persister();
        let mut cp = TaskCheckpoint::new("old", TaskType::ProjectExec, &["clone"]);
        // Backdate by 10 days
        cp.last_updated = Utc::now() - chrono::Duration::days(10);
        p.save(&cp).unwrap();
        let removed = p.cleanup_old(7).unwrap();
        assert_eq!(removed, 1);
        assert!(p.load("old").is_err());
    }

    #[test]
    fn test_checkpoint_advance() {
        let mut cp = TaskCheckpoint::new("adv", TaskType::ProjectExec, &["clone", "test", "report"]);
        assert_eq!(cp.progress, 0.0);
        cp.advance("clone", "testing", serde_json::json!({"dir": "/tmp"}));
        assert_eq!(cp.completed_steps, vec!["clone"]);
        assert_eq!(cp.remaining_steps, vec!["test", "report"]);
        assert!((cp.progress - 1.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn test_checkpoint_survives_restart() {
        let (p, dir) = temp_persister();
        let cp = TaskCheckpoint::new("persist", TaskType::ProjectExec, &["clone"]);
        p.save(&cp).unwrap();
        // Create new persister pointing to same dir (simulates restart)
        let p2 = TaskPersister::with_dir(dir.path().to_path_buf());
        let loaded = p2.load("persist").unwrap();
        assert_eq!(loaded.task_id, "persist");
    }

    #[test]
    fn test_multiple_tasks() {
        let (p, _dir) = temp_persister();
        for i in 0..5 {
            let cp = TaskCheckpoint::new(&format!("task-{}", i), TaskType::ProjectExec, &["a"]);
            p.save(&cp).unwrap();
        }
        let incomplete = p.list_incomplete().unwrap();
        assert_eq!(incomplete.len(), 5);
    }

    #[test]
    fn test_task_type_label() {
        assert_eq!(TaskType::ProjectExec.label(), "Project Execution");
        assert_eq!(TaskType::SelfImplement.label(), "Self-Implement");
        assert_eq!(TaskType::Custom("Deploy".into()).label(), "Deploy");
    }

    #[test]
    fn test_mark_complete() {
        let mut cp = TaskCheckpoint::new("done", TaskType::ProjectExec, &["a", "b"]);
        cp.mark_complete();
        assert_eq!(cp.progress, 1.0);
        assert_eq!(cp.phase, "complete");
        assert!(cp.remaining_steps.is_empty());
    }

    #[test]
    fn test_is_stale() {
        let mut cp = TaskCheckpoint::new("s", TaskType::ProjectExec, &["a"]);
        assert!(!cp.is_stale(7));
        cp.last_updated = Utc::now() - chrono::Duration::days(8);
        assert!(cp.is_stale(7));
    }

    #[test]
    fn test_sanitize_task_id() {
        let (p, _dir) = temp_persister();
        let cp = TaskCheckpoint::new("https://github.com/user/repo", TaskType::ProjectExec, &["a"]);
        p.save(&cp).unwrap();
        // Should be loadable with same ID
        let loaded = p.load("https://github.com/user/repo").unwrap();
        assert_eq!(loaded.task_id, "https://github.com/user/repo");
    }
}
