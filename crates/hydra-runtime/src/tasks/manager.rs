use chrono::{Duration, Utc};
use uuid::Uuid;

use super::task::{HydraTaskStatus, Task};

/// In-memory task manager for Hydra
pub struct TaskManager {
    tasks: Vec<Task>,
}

impl TaskManager {
    pub fn new() -> Self {
        Self { tasks: Vec::new() }
    }

    /// Create a new task with the given title, returning the created task
    pub fn create_task(&mut self, title: &str) -> Task {
        let task = Task {
            id: Uuid::new_v4().to_string(),
            title: title.into(),
            description: None,
            status: HydraTaskStatus::Pending,
            created_at: Utc::now(),
            completed_at: None,
            run_id: None,
            parent_id: None,
        };
        self.tasks.push(task.clone());
        task
    }

    /// Update the status of a task by ID
    pub fn update_status(&mut self, id: &str, status: HydraTaskStatus) -> bool {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
            task.status = status;
            if status.is_terminal() {
                task.completed_at = Some(Utc::now());
            }
            true
        } else {
            false
        }
    }

    /// Mark a task as completed
    pub fn complete_task(&mut self, id: &str) -> bool {
        self.update_status(id, HydraTaskStatus::Completed)
    }

    /// Get tasks created today
    pub fn get_today(&self) -> Vec<&Task> {
        let today = Utc::now().date_naive();
        self.tasks
            .iter()
            .filter(|t| t.created_at.date_naive() == today)
            .collect()
    }

    /// Get tasks created yesterday
    pub fn get_yesterday(&self) -> Vec<&Task> {
        let yesterday = (Utc::now() - Duration::days(1)).date_naive();
        self.tasks
            .iter()
            .filter(|t| t.created_at.date_naive() == yesterday)
            .collect()
    }

    /// Get tasks created in the last N days
    pub fn get_history(&self, days: i64) -> Vec<&Task> {
        let cutoff = Utc::now() - Duration::days(days);
        self.tasks
            .iter()
            .filter(|t| t.created_at >= cutoff)
            .collect()
    }

    /// Search tasks by title (case-insensitive substring match)
    pub fn search(&self, query: &str) -> Vec<&Task> {
        let query_lower = query.to_lowercase();
        self.tasks
            .iter()
            .filter(|t| t.title.to_lowercase().contains(&query_lower))
            .collect()
    }

    /// Link a task to a run ID
    pub fn link_to_run(&mut self, task_id: &str, run_id: &str) -> bool {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == task_id) {
            task.run_id = Some(run_id.into());
            true
        } else {
            false
        }
    }

    /// Delete a task by ID, returning true if it existed
    pub fn delete_task(&mut self, id: &str) -> bool {
        let before = self.tasks.len();
        self.tasks.retain(|t| t.id != id);
        self.tasks.len() < before
    }

    /// Get a task by ID
    pub fn get_by_id(&self, id: &str) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }

    /// Get all tasks
    pub fn all(&self) -> &[Task] {
        &self.tasks
    }

    /// Get count of tasks
    pub fn count(&self) -> usize {
        self.tasks.len()
    }
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
}
