use super::task::{HydraTaskStatus, Task};

/// Filter tasks by status
pub fn by_status<'a>(tasks: &'a [Task], status: HydraTaskStatus) -> Vec<&'a Task> {
    tasks.iter().filter(|t| t.status == status).collect()
}

/// Get subtasks of a given parent task
pub fn subtasks<'a>(tasks: &'a [Task], parent_id: &str) -> Vec<&'a Task> {
    tasks
        .iter()
        .filter(|t| t.parent_id.as_deref() == Some(parent_id))
        .collect()
}

/// Get tasks linked to a specific run
pub fn by_run<'a>(tasks: &'a [Task], run_id: &str) -> Vec<&'a Task> {
    tasks
        .iter()
        .filter(|t| t.run_id.as_deref() == Some(run_id))
        .collect()
}

/// Get tasks ordered by creation time (newest first)
pub fn newest_first(tasks: &mut [Task]) {
    tasks.sort_by(|a, b| b.created_at.cmp(&a.created_at));
}

/// Get tasks ordered by creation time (oldest first)
pub fn oldest_first(tasks: &mut [Task]) {
    tasks.sort_by(|a, b| a.created_at.cmp(&b.created_at));
}
