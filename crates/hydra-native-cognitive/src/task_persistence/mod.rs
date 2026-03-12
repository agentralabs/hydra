//! Task Persistence & Recovery — survives crashes, network drops, terminal close.
//!
//! Every long-running task (ProjectExec, SelfImplement, agent swarms) saves
//! checkpoints to `~/.hydra/checkpoints/`. On restart, Hydra detects
//! interrupted tasks and offers to resume.

pub mod serializer;
pub mod recovery;

pub use serializer::{TaskCheckpoint, TaskType, TaskPersister};
pub use recovery::{detect_interrupted, RecoveryPlan};

use std::path::PathBuf;

/// Default checkpoint directory.
pub fn checkpoint_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".hydra").join("checkpoints")
}

/// Format a task summary for display.
pub fn format_task_summary(checkpoint: &TaskCheckpoint) -> String {
    let age = chrono::Utc::now()
        .signed_duration_since(checkpoint.last_updated)
        .num_minutes();
    let age_str = if age < 60 {
        format!("{} min ago", age)
    } else if age < 1440 {
        format!("{} hours ago", age / 60)
    } else {
        format!("{} days ago", age / 1440)
    };
    format!(
        "{} — {} (phase: {}, {:.0}% complete, {})",
        checkpoint.task_type.label(),
        checkpoint.task_id,
        checkpoint.phase,
        checkpoint.progress * 100.0,
        age_str,
    )
}

/// Format all incomplete tasks for display.
pub fn format_task_list(tasks: &[TaskCheckpoint]) -> String {
    if tasks.is_empty() {
        return "No active or interrupted tasks.".to_string();
    }
    let mut out = format!("**{} task(s)**\n\n", tasks.len());
    for (i, t) in tasks.iter().enumerate() {
        out.push_str(&format!(
            "{}. `{}` — {} (phase: **{}**, {:.0}%)\n",
            i + 1,
            t.task_id,
            t.task_type.label(),
            t.phase,
            t.progress * 100.0,
        ));
        if !t.completed_steps.is_empty() {
            out.push_str(&format!(
                "   Done: {}\n",
                t.completed_steps.join(", ")
            ));
        }
        if !t.remaining_steps.is_empty() {
            out.push_str(&format!(
                "   Remaining: {}\n",
                t.remaining_steps.join(", ")
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_task_summary() {
        let cp = TaskCheckpoint {
            task_id: "test-123".into(),
            task_type: TaskType::ProjectExec,
            started_at: chrono::Utc::now(),
            last_updated: chrono::Utc::now(),
            phase: "testing".into(),
            progress: 0.6,
            state: serde_json::json!({}),
            completed_steps: vec!["clone".into(), "analyze".into()],
            remaining_steps: vec!["test".into()],
        };
        let s = format_task_summary(&cp);
        assert!(s.contains("Project Execution"));
        assert!(s.contains("60%"));
        assert!(s.contains("testing"));
    }

    #[test]
    fn test_format_task_list_empty() {
        assert!(format_task_list(&[]).contains("No active"));
    }

    #[test]
    fn test_format_task_list_with_items() {
        let cp = TaskCheckpoint {
            task_id: "abc".into(),
            task_type: TaskType::SelfImplement,
            started_at: chrono::Utc::now(),
            last_updated: chrono::Utc::now(),
            phase: "patching".into(),
            progress: 0.5,
            state: serde_json::json!({}),
            completed_steps: vec!["spec_read".into()],
            remaining_steps: vec!["apply".into()],
        };
        let s = format_task_list(&[cp]);
        assert!(s.contains("1 task(s)"));
        assert!(s.contains("Self-Implement"));
        assert!(s.contains("spec_read"));
    }

    #[test]
    fn test_checkpoint_dir() {
        let dir = checkpoint_dir();
        assert!(dir.to_str().unwrap().contains("checkpoints"));
    }
}
