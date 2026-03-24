//! TaskQueue — manages blocked tasks awaiting human intervention.
//! When Hydra gets stuck (CAPTCHA, approval needed, ambiguous), the task
//! is parked here until the human resolves it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Why a task is blocked.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BlockedReason {
    /// CAPTCHA could not be solved automatically.
    CaptchaUnsolvable { domain: String },
    /// Action requires human approval before proceeding.
    NeedsApproval { description: String },
    /// Situation is ambiguous — multiple valid paths.
    Ambiguous { options: Vec<String> },
    /// Unrecoverable error that needs human debugging.
    Error { message: String },
    /// Credential needed but not in vault.
    CredentialMissing { domain: String },
}

impl BlockedReason {
    /// Human-readable summary for TUI display.
    pub fn summary(&self) -> String {
        match self {
            Self::CaptchaUnsolvable { domain } => {
                format!("Cannot solve CAPTCHA on {domain}")
            }
            Self::NeedsApproval { description } => {
                format!("Needs approval: {description}")
            }
            Self::Ambiguous { options } => {
                format!("Ambiguous: {} options", options.len())
            }
            Self::Error { message } => {
                format!("Error: {}", &message[..message.len().min(60)])
            }
            Self::CredentialMissing { domain } => {
                format!("No credentials for {domain}")
            }
        }
    }

    /// TUI symbol for this blocked reason.
    pub fn symbol(&self) -> &'static str {
        match self {
            Self::CaptchaUnsolvable { .. } => "🔒",
            Self::NeedsApproval { .. } => "⏸",
            Self::Error { .. } => "⚠",
            Self::Ambiguous { .. } => "❓",
            Self::CredentialMissing { .. } => "🔑",
        }
    }
}

/// A task that is blocked and waiting for human resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockedTask {
    pub id: String,
    pub description: String,
    pub context: String,
    pub reason: BlockedReason,
    pub screenshot_b64: Option<String>,
    pub created_at: DateTime<Utc>,
    pub priority: u8,
}

/// Resolution provided by the human.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Resolution {
    /// Human provided the answer (e.g., CAPTCHA text).
    Answer(String),
    /// Human approved the action.
    Approved,
    /// Human rejected — cancel the task.
    Rejected,
    /// Human chose one of the ambiguous options.
    ChosenOption(usize),
    /// Human provided credentials.
    Credential { username: String, password: String },
}

/// Queue of blocked tasks.
pub struct TaskQueue {
    tasks: VecDeque<BlockedTask>,
    max_size: usize,
    resolved_count: u64,
}

impl TaskQueue {
    pub fn new() -> Self {
        Self {
            tasks: VecDeque::new(),
            max_size: 100,
            resolved_count: 0,
        }
    }

    /// Add a blocked task to the queue.
    pub fn enqueue(&mut self, task: BlockedTask) {
        if self.tasks.len() >= self.max_size {
            // Remove oldest low-priority task
            if let Some(pos) = self.tasks.iter().position(|t| t.priority == 0) {
                self.tasks.remove(pos);
            }
        }
        eprintln!(
            "hydra-companion: task blocked — {} ({})",
            task.reason.summary(),
            task.id
        );
        self.tasks.push_back(task);
    }

    /// Get all pending blocked tasks for display.
    pub fn pending(&self) -> Vec<&BlockedTask> {
        self.tasks.iter().collect()
    }

    /// Get pending tasks by priority (highest first).
    pub fn pending_by_priority(&self) -> Vec<&BlockedTask> {
        let mut tasks: Vec<&BlockedTask> = self.tasks.iter().collect();
        tasks.sort_by(|a, b| b.priority.cmp(&a.priority));
        tasks
    }

    /// Resolve a blocked task and remove it from the queue.
    pub fn resolve(&mut self, task_id: &str, resolution: Resolution) -> Option<(BlockedTask, Resolution)> {
        if let Some(pos) = self.tasks.iter().position(|t| t.id == task_id) {
            let task = self.tasks.remove(pos).unwrap();
            self.resolved_count += 1;
            eprintln!(
                "hydra-companion: task resolved — {} (total: {})",
                task_id, self.resolved_count
            );
            Some((task, resolution))
        } else {
            None
        }
    }

    /// Cancel a blocked task.
    pub fn cancel(&mut self, task_id: &str) -> Option<BlockedTask> {
        if let Some(pos) = self.tasks.iter().position(|t| t.id == task_id) {
            self.tasks.remove(pos)
        } else {
            None
        }
    }

    pub fn count(&self) -> usize {
        self.tasks.len()
    }

    pub fn has_urgent(&self) -> bool {
        self.tasks.iter().any(|t| t.priority >= 8)
    }

    pub fn resolved_count(&self) -> u64 {
        self.resolved_count
    }
}

impl Default for TaskQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_task(id: &str, priority: u8) -> BlockedTask {
        BlockedTask {
            id: id.into(),
            description: format!("Test task {id}"),
            context: "test".into(),
            reason: BlockedReason::NeedsApproval {
                description: "testing".into(),
            },
            screenshot_b64: None,
            created_at: Utc::now(),
            priority,
        }
    }

    #[test]
    fn enqueue_and_pending() {
        let mut q = TaskQueue::new();
        q.enqueue(make_task("t1", 5));
        q.enqueue(make_task("t2", 8));
        assert_eq!(q.count(), 2);
        assert_eq!(q.pending().len(), 2);
    }

    #[test]
    fn resolve_removes_task() {
        let mut q = TaskQueue::new();
        q.enqueue(make_task("t1", 5));
        let result = q.resolve("t1", Resolution::Approved);
        assert!(result.is_some());
        assert_eq!(q.count(), 0);
        assert_eq!(q.resolved_count(), 1);
    }

    #[test]
    fn resolve_missing_returns_none() {
        let mut q = TaskQueue::new();
        assert!(q.resolve("missing", Resolution::Approved).is_none());
    }

    #[test]
    fn priority_ordering() {
        let mut q = TaskQueue::new();
        q.enqueue(make_task("low", 1));
        q.enqueue(make_task("high", 9));
        q.enqueue(make_task("mid", 5));
        let ordered = q.pending_by_priority();
        assert_eq!(ordered[0].id, "high");
        assert_eq!(ordered[2].id, "low");
    }

    #[test]
    fn has_urgent_detection() {
        let mut q = TaskQueue::new();
        q.enqueue(make_task("normal", 3));
        assert!(!q.has_urgent());
        q.enqueue(make_task("urgent", 9));
        assert!(q.has_urgent());
    }

    #[test]
    fn blocked_reason_summaries() {
        let r = BlockedReason::CaptchaUnsolvable {
            domain: "test.com".into(),
        };
        assert!(r.summary().contains("test.com"));
        assert_eq!(r.symbol(), "🔒");
    }
}
