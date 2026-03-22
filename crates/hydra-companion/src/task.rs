//! Task types and executor for the companion system.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::constants;
use crate::errors::CompanionError;

/// Autonomy level for a companion task (set per task, default: Confirm).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AutonomyLevel {
    /// Observes and reports, never acts.
    Report,
    /// Acts only after user approval (DEFAULT).
    Confirm,
    /// Acts, shows summary afterward.
    Summarize,
    /// Acts and logs — requires explicit principal authorization.
    Auto,
}

impl Default for AutonomyLevel {
    fn default() -> Self {
        Self::Confirm
    }
}

impl std::fmt::Display for AutonomyLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Report => write!(f, "report"),
            Self::Confirm => write!(f, "confirm"),
            Self::Summarize => write!(f, "summarize"),
            Self::Auto => write!(f, "auto"),
        }
    }
}

/// Status of a companion task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Task is waiting to be executed.
    Pending,
    /// Task is currently running.
    Running,
    /// Task completed successfully.
    Complete,
    /// Task failed with a reason.
    Failed {
        /// Why the task failed.
        reason: String,
    },
    /// Task was cancelled.
    Cancelled,
    /// Task is blocked — needs user input before continuing.
    Blocked {
        /// What the task needs from the user.
        reason: String,
    },
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Running => write!(f, "running"),
            Self::Complete => write!(f, "complete"),
            Self::Failed { reason } => write!(f, "failed: {reason}"),
            Self::Cancelled => write!(f, "cancelled"),
            Self::Blocked { reason } => write!(f, "blocked: {reason}"),
        }
    }
}

/// A task managed by the companion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanionTask {
    /// Unique ID for this task.
    pub id: Uuid,
    /// Description of what the task does.
    pub description: String,
    /// Current status.
    pub status: TaskStatus,
    /// Autonomy level for this task.
    pub autonomy: AutonomyLevel,
    /// When the task was created.
    pub created_at: DateTime<Utc>,
    /// When the task started running (if it has).
    pub started_at: Option<DateTime<Utc>>,
    /// When the task completed (if it has).
    pub completed_at: Option<DateTime<Utc>>,
}

impl CompanionTask {
    /// Create a new pending task with default autonomy (Confirm).
    pub fn new(description: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            description,
            status: TaskStatus::Pending,
            autonomy: AutonomyLevel::default(),
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
        }
    }

    /// Create a new pending task with specified autonomy level.
    pub fn with_autonomy(description: String, autonomy: AutonomyLevel) -> Self {
        Self {
            autonomy,
            ..Self::new(description)
        }
    }

    /// Mark the task as running.
    pub fn start(&mut self) {
        self.status = TaskStatus::Running;
        self.started_at = Some(Utc::now());
    }

    /// Mark the task as complete.
    pub fn complete(&mut self) {
        self.status = TaskStatus::Complete;
        self.completed_at = Some(Utc::now());
    }

    /// Mark the task as failed.
    pub fn fail(&mut self, reason: String) {
        self.status = TaskStatus::Failed { reason };
        self.completed_at = Some(Utc::now());
    }

    /// Cancel the task.
    pub fn cancel(&mut self) {
        self.status = TaskStatus::Cancelled;
        self.completed_at = Some(Utc::now());
    }

    /// Block the task — needs user input.
    pub fn block(&mut self, reason: String) {
        self.status = TaskStatus::Blocked { reason };
    }

    /// Unblock a blocked task back to running.
    pub fn unblock(&mut self) {
        if matches!(self.status, TaskStatus::Blocked { .. }) {
            self.status = TaskStatus::Running;
        }
    }

    /// Return whether the task is terminal (complete, failed, or cancelled).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            TaskStatus::Complete | TaskStatus::Failed { .. } | TaskStatus::Cancelled
        )
    }

    /// Return whether the task is currently running.
    pub fn is_running(&self) -> bool {
        matches!(self.status, TaskStatus::Running)
    }

    /// Return whether the task is blocked.
    pub fn is_blocked(&self) -> bool {
        matches!(self.status, TaskStatus::Blocked { .. })
    }

    /// Return the TUI symbol for the current status.
    pub fn status_symbol(&self) -> &'static str {
        match self.status {
            TaskStatus::Pending => "⏵",
            TaskStatus::Running => "⏵",
            TaskStatus::Complete => "✓",
            TaskStatus::Failed { .. } => "✗",
            TaskStatus::Cancelled => "✗",
            TaskStatus::Blocked { .. } => "⚠",
        }
    }
}

/// Executor that manages companion tasks.
#[derive(Debug, Clone)]
pub struct TaskExecutor {
    /// All tasks (active and completed).
    tasks: Vec<CompanionTask>,
}

impl TaskExecutor {
    /// Create a new empty task executor.
    pub fn new() -> Self {
        Self { tasks: Vec::new() }
    }

    /// Submit a new task for execution. Returns the task ID.
    pub fn submit(&mut self, description: String) -> Result<Uuid, CompanionError> {
        self.submit_with_autonomy(description, AutonomyLevel::default())
    }

    /// Submit a new task with a specific autonomy level.
    pub fn submit_with_autonomy(
        &mut self,
        description: String,
        autonomy: AutonomyLevel,
    ) -> Result<Uuid, CompanionError> {
        let active_count = self.tasks.iter().filter(|t| t.is_running()).count();
        if active_count >= constants::MAX_CONCURRENT_TASKS {
            return Err(CompanionError::TaskLimitReached {
                max: constants::MAX_CONCURRENT_TASKS,
            });
        }

        let task = CompanionTask::with_autonomy(description, autonomy);
        let id = task.id;
        self.tasks.push(task);
        Ok(id)
    }

    /// Start a pending task by ID.
    pub fn start_task(&mut self, task_id: Uuid) -> Result<(), CompanionError> {
        let task = self.find_task_mut(task_id)?;
        task.start();
        Ok(())
    }

    /// Complete a task by ID.
    pub fn complete_task(&mut self, task_id: Uuid) -> Result<(), CompanionError> {
        let task = self.find_task_mut(task_id)?;
        task.complete();
        Ok(())
    }

    /// Fail a task by ID.
    pub fn fail_task(&mut self, task_id: Uuid, reason: String) -> Result<(), CompanionError> {
        let task = self.find_task_mut(task_id)?;
        task.fail(reason);
        Ok(())
    }

    /// Cancel a task by ID.
    pub fn cancel_task(&mut self, task_id: Uuid) -> Result<(), CompanionError> {
        let task = self.find_task_mut(task_id)?;
        task.cancel();
        Ok(())
    }

    /// Block a task by ID.
    pub fn block_task(&mut self, task_id: Uuid, reason: String) -> Result<(), CompanionError> {
        let task = self.find_task_mut(task_id)?;
        task.block(reason);
        Ok(())
    }

    /// Unblock a blocked task by ID.
    pub fn unblock_task(&mut self, task_id: Uuid) -> Result<(), CompanionError> {
        let task = self.find_task_mut(task_id)?;
        task.unblock();
        Ok(())
    }

    /// Return all tasks.
    pub fn tasks(&self) -> &[CompanionTask] {
        &self.tasks
    }

    /// Return the number of currently running tasks.
    pub fn active_count(&self) -> usize {
        self.tasks.iter().filter(|t| t.is_running()).count()
    }

    /// Return the number of completed tasks (including failed/cancelled).
    pub fn completed_count(&self) -> usize {
        self.tasks.iter().filter(|t| t.is_terminal()).count()
    }

    /// Return a task by ID.
    pub fn get_task(&self, task_id: Uuid) -> Option<&CompanionTask> {
        self.tasks.iter().find(|t| t.id == task_id)
    }

    /// Find a mutable task by ID or return TaskNotFound error.
    fn find_task_mut(&mut self, task_id: Uuid) -> Result<&mut CompanionTask, CompanionError> {
        self.tasks
            .iter_mut()
            .find(|t| t.id == task_id)
            .ok_or_else(|| CompanionError::TaskNotFound {
                task_id: task_id.to_string(),
            })
    }
}

impl Default for TaskExecutor {
    fn default() -> Self {
        Self::new()
    }
}
