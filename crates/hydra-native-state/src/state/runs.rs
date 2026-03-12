//! Active runs tracking — monitors in-progress and completed runs.

use serde::{Deserialize, Serialize};

/// Status of a tracked run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Running,
    Completed,
    Failed,
}

/// A single step within a run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunStep {
    pub label: String,
    pub emoji: String,
    pub completed: bool,
}

/// A run being tracked by the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedRun {
    pub id: String,
    pub intent: String,
    pub status: RunStatus,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub steps: Vec<RunStep>,
}

/// Tracks active, completed, and historical runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunTracker {
    pub active_runs: Vec<TrackedRun>,
    pub completed_today: Vec<TrackedRun>,
    pub history: Vec<TrackedRun>,
}

impl RunTracker {
    pub fn new() -> Self {
        Self {
            active_runs: Vec::new(),
            completed_today: Vec::new(),
            history: Vec::new(),
        }
    }

    /// Start tracking a new run. Returns a reference to the tracked run.
    pub fn start_run(&mut self, id: &str, intent: &str) -> &TrackedRun {
        let run = TrackedRun {
            id: id.to_string(),
            intent: intent.to_string(),
            status: RunStatus::Running,
            started_at: chrono::Utc::now().to_rfc3339(),
            completed_at: None,
            steps: Vec::new(),
        };
        self.active_runs.push(run);
        self.active_runs.last().unwrap()
    }

    /// Mark a run as completed and move it to completed_today.
    pub fn complete_run(&mut self, id: &str) {
        if let Some(pos) = self.active_runs.iter().position(|r| r.id == id) {
            let mut run = self.active_runs.remove(pos);
            run.status = RunStatus::Completed;
            run.completed_at = Some(chrono::Utc::now().to_rfc3339());
            self.completed_today.push(run);
        }
    }

    /// Number of currently active runs.
    pub fn active_count(&self) -> usize {
        self.active_runs.len()
    }

    /// Human-readable status summary.
    pub fn status_summary(&self) -> String {
        match self.active_runs.len() {
            0 => "All clear".to_string(),
            1 => "1 task running".to_string(),
            n => format!("{} tasks running", n),
        }
    }
}

impl Default for RunTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_tracker() {
        let t = RunTracker::new();
        assert!(t.active_runs.is_empty());
        assert!(t.completed_today.is_empty());
        assert!(t.history.is_empty());
    }

    #[test]
    fn test_start_run() {
        let mut t = RunTracker::new();
        let run = t.start_run("r1", "deploy app");
        assert_eq!(run.id, "r1");
        assert_eq!(run.intent, "deploy app");
        assert_eq!(run.status, RunStatus::Running);
        assert!(run.completed_at.is_none());
        assert_eq!(t.active_count(), 1);
    }

    #[test]
    fn test_complete_run() {
        let mut t = RunTracker::new();
        t.start_run("r1", "deploy app");
        assert_eq!(t.active_count(), 1);

        t.complete_run("r1");
        assert_eq!(t.active_count(), 0);
        assert_eq!(t.completed_today.len(), 1);
        assert_eq!(t.completed_today[0].status, RunStatus::Completed);
        assert!(t.completed_today[0].completed_at.is_some());
    }

    #[test]
    fn test_complete_nonexistent_run_is_noop() {
        let mut t = RunTracker::new();
        t.complete_run("nope");
        assert_eq!(t.active_count(), 0);
        assert!(t.completed_today.is_empty());
    }

    #[test]
    fn test_status_summary() {
        let mut t = RunTracker::new();
        assert_eq!(t.status_summary(), "All clear");

        t.start_run("r1", "a");
        assert_eq!(t.status_summary(), "1 task running");

        t.start_run("r2", "b");
        assert_eq!(t.status_summary(), "2 tasks running");
    }

    #[test]
    fn test_multiple_runs() {
        let mut t = RunTracker::new();
        t.start_run("r1", "first");
        t.start_run("r2", "second");
        t.start_run("r3", "third");
        assert_eq!(t.active_count(), 3);

        t.complete_run("r2");
        assert_eq!(t.active_count(), 2);
        assert_eq!(t.completed_today.len(), 1);
        assert_eq!(t.completed_today[0].id, "r2");
    }

    #[test]
    fn test_serialization() {
        let mut t = RunTracker::new();
        t.start_run("r1", "test");
        let json = serde_json::to_string(&t).unwrap();
        let back: RunTracker = serde_json::from_str(&json).unwrap();
        assert_eq!(back.active_count(), 1);
        assert_eq!(back.active_runs[0].id, "r1");
    }
}
