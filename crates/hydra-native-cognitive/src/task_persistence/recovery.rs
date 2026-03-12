//! Recovery — detect interrupted tasks on startup, build resume plans.

use super::serializer::{TaskCheckpoint, TaskPersister, TaskType};

/// A plan for resuming an interrupted task.
#[derive(Debug, Clone)]
pub struct RecoveryPlan {
    pub checkpoint: TaskCheckpoint,
    pub resume_from: String,
    pub steps_to_run: Vec<String>,
    pub display_prompt: String,
}

impl RecoveryPlan {
    /// Build a recovery plan from a checkpoint.
    pub fn from_checkpoint(cp: TaskCheckpoint) -> Self {
        let resume_from = cp.remaining_steps.first()
            .cloned()
            .unwrap_or_else(|| cp.phase.clone());
        let steps_to_run = cp.remaining_steps.clone();
        let display_prompt = format!(
            "Interrupted task found:\n  {} ({})\n  Phase: {}, {:.0}% complete\n  \
             Completed: {}\n  Remaining: {}\n\nResume? (y/n)",
            cp.task_id,
            cp.task_type.label(),
            cp.phase,
            cp.progress * 100.0,
            if cp.completed_steps.is_empty() { "none".into() } else { cp.completed_steps.join(", ") },
            if steps_to_run.is_empty() { "none".into() } else { steps_to_run.join(", ") },
        );
        Self {
            checkpoint: cp,
            resume_from,
            steps_to_run,
            display_prompt,
        }
    }
}

/// Detect all interrupted tasks and build recovery plans.
pub fn detect_interrupted(persister: &TaskPersister) -> Vec<RecoveryPlan> {
    match persister.list_incomplete() {
        Ok(checkpoints) => checkpoints
            .into_iter()
            .map(RecoveryPlan::from_checkpoint)
            .collect(),
        Err(e) => {
            eprintln!("[hydra:recovery] Failed to list checkpoints: {}", e);
            Vec::new()
        }
    }
}

/// Build a resume command for a task.
pub fn resume_command(task_id: &str, task_type: &TaskType) -> String {
    match task_type {
        TaskType::ProjectExec => format!("/resume-task {}", task_id),
        TaskType::SelfImplement => format!("/resume-task {}", task_id),
        _ => format!("/resume-task {}", task_id),
    }
}

/// Cancel a task: mark complete (removes checkpoint).
pub fn cancel_task(persister: &TaskPersister, task_id: &str) -> Result<String, String> {
    persister.complete(task_id)?;
    Ok(format!("Task `{}` cancelled and checkpoint removed.", task_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn temp_persister() -> (TaskPersister, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let p = TaskPersister::with_dir(dir.path().to_path_buf());
        (p, dir)
    }

    #[test]
    fn test_detect_interrupted() {
        let (p, _dir) = temp_persister();
        let cp = TaskCheckpoint::new("int-1", TaskType::ProjectExec, &["clone", "test"]);
        p.save(&cp).unwrap();
        let plans = detect_interrupted(&p);
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].checkpoint.task_id, "int-1");
        assert!(plans[0].display_prompt.contains("Interrupted task found"));
    }

    #[test]
    fn test_detect_no_interrupted() {
        let (p, _dir) = temp_persister();
        let plans = detect_interrupted(&p);
        assert!(plans.is_empty());
    }

    #[test]
    fn test_recovery_plan_from_checkpoint() {
        let mut cp = TaskCheckpoint::new("rp-1", TaskType::ProjectExec, &["clone", "analyze", "test"]);
        cp.advance("clone", "analyzing", serde_json::json!({}));
        let plan = RecoveryPlan::from_checkpoint(cp);
        assert_eq!(plan.resume_from, "analyze");
        assert_eq!(plan.steps_to_run, vec!["analyze", "test"]);
        assert!(plan.display_prompt.contains("33%"));
    }

    #[test]
    fn test_cancel_task() {
        let (p, _dir) = temp_persister();
        let cp = TaskCheckpoint::new("cancel-me", TaskType::ProjectExec, &["a"]);
        p.save(&cp).unwrap();
        let msg = cancel_task(&p, "cancel-me").unwrap();
        assert!(msg.contains("cancelled"));
        assert!(p.load("cancel-me").is_err());
    }

    #[test]
    fn test_resume_command() {
        let cmd = resume_command("abc", &TaskType::ProjectExec);
        assert_eq!(cmd, "/resume-task abc");
    }

    #[test]
    fn test_recovery_plan_display() {
        let mut cp = TaskCheckpoint {
            task_id: "my-repo".into(),
            task_type: TaskType::ProjectExec,
            started_at: Utc::now(),
            last_updated: Utc::now(),
            phase: "testing".into(),
            progress: 0.6,
            state: serde_json::json!({}),
            completed_steps: vec!["clone".into(), "analyze".into(), "setup".into()],
            remaining_steps: vec!["test".into(), "report".into()],
        };
        let plan = RecoveryPlan::from_checkpoint(cp);
        assert!(plan.display_prompt.contains("clone, analyze, setup"));
        assert!(plan.display_prompt.contains("test, report"));
    }

    #[test]
    fn test_detect_skips_completed_tasks() {
        let (p, _dir) = temp_persister();
        let mut cp1 = TaskCheckpoint::new("done", TaskType::ProjectExec, &["a"]);
        cp1.mark_complete();
        let cp2 = TaskCheckpoint::new("active", TaskType::ProjectExec, &["b"]);
        p.save(&cp1).unwrap();
        p.save(&cp2).unwrap();
        let plans = detect_interrupted(&p);
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].checkpoint.task_id, "active");
    }
}
