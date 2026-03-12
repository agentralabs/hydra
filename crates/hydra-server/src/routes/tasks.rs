use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use hydra_runtime::tasks::{HydraTaskStatus, Task};

/// Task status with progress information
#[derive(Debug, Serialize)]
pub struct TaskStatusResponse {
    pub id: String,
    pub status: HydraTaskStatus,
    pub title: String,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub run_id: Option<String>,
}

use crate::state::AppState;

/// Route definitions for task management.
pub struct TaskRoutes;

impl TaskRoutes {
    /// GET: list all tasks (with optional status filter)
    pub fn list_tasks() -> &'static str {
        "/api/tasks"
    }

    /// POST: create a new task
    pub fn create_task() -> &'static str {
        "/api/tasks"
    }

    /// GET: retrieve a task by ID
    pub fn get_task() -> &'static str {
        "/api/tasks/:id"
    }

    /// PUT: update a task
    pub fn update_task() -> &'static str {
        "/api/tasks/:id"
    }

    /// DELETE: cancel a task
    pub fn cancel_task() -> &'static str {
        "/api/tasks/:id"
    }

    /// POST: pause a running task
    pub fn pause_task() -> &'static str {
        "/api/tasks/:id/pause"
    }

    /// POST: resume a paused task
    pub fn resume_task() -> &'static str {
        "/api/tasks/:id/resume"
    }

    /// GET: get task progress and status
    pub fn task_status() -> &'static str {
        "/api/tasks/:id/status"
    }

    /// GET: list subtasks of a task
    pub fn list_subtasks() -> &'static str {
        "/api/tasks/:id/subtasks"
    }
}

// ═══════════════════════════════════════════════════════════
// REQUEST / RESPONSE TYPES
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaskRequest {
    pub status: Option<HydraTaskStatus>,
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListTasksQuery {
    pub status: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DeletedResponse {
    pub deleted: bool,
}

// ═══════════════════════════════════════════════════════════
// HANDLERS
// ═══════════════════════════════════════════════════════════

/// GET /api/tasks — list all tasks, optionally filtered by status
pub async fn list_tasks(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListTasksQuery>,
) -> Json<Vec<Task>> {
    let mgr = state.task_manager.lock();
    let tasks: Vec<Task> = if let Some(status_str) = &params.status {
        if let Some(status) = HydraTaskStatus::from_str(status_str) {
            mgr.all()
                .iter()
                .filter(|t| t.status == status)
                .cloned()
                .collect()
        } else {
            mgr.all().to_vec()
        }
    } else {
        mgr.all().to_vec()
    };
    Json(tasks)
}

/// POST /api/tasks — create a new task
pub async fn create_task(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateTaskRequest>,
) -> (StatusCode, Json<Task>) {
    let mut mgr = state.task_manager.lock();
    let mut task = mgr.create_task(&req.title);
    if let Some(desc) = req.description {
        task.description = Some(desc);
    }
    (StatusCode::CREATED, Json(task))
}

/// GET /api/tasks/:id — get a single task by ID
pub async fn get_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Task>, (StatusCode, String)> {
    let mgr = state.task_manager.lock();
    match mgr.get_by_id(&id) {
        Some(task) => Ok(Json(task.clone())),
        None => Err((StatusCode::NOT_FOUND, format!("Task {id} not found"))),
    }
}

/// PUT /api/tasks/:id — update a task's status/title
pub async fn update_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateTaskRequest>,
) -> Result<Json<Task>, (StatusCode, String)> {
    let mut mgr = state.task_manager.lock();

    // Verify task exists
    if mgr.get_by_id(&id).is_none() {
        return Err((StatusCode::NOT_FOUND, format!("Task {id} not found")));
    }

    if let Some(status) = req.status {
        mgr.update_status(&id, status);
    }

    match mgr.get_by_id(&id) {
        Some(task) => Ok(Json(task.clone())),
        None => Err((StatusCode::NOT_FOUND, format!("Task {id} not found"))),
    }
}

/// DELETE /api/tasks/:id — remove a task
pub async fn delete_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<DeletedResponse>, (StatusCode, String)> {
    let mut mgr = state.task_manager.lock();
    if mgr.delete_task(&id) {
        Ok(Json(DeletedResponse { deleted: true }))
    } else {
        Err((StatusCode::NOT_FOUND, format!("Task {id} not found")))
    }
}

/// POST /api/tasks/:id/pause — pause a running task
pub async fn pause_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Task>, (StatusCode, String)> {
    let mut mgr = state.task_manager.lock();
    let task = mgr
        .get_by_id(&id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Task {id} not found")))?;

    if task.status != HydraTaskStatus::Active {
        return Err((
            StatusCode::CONFLICT,
            format!("Task {id} is not active (current status: {})", task.status.as_str()),
        ));
    }

    mgr.update_status(&id, HydraTaskStatus::Pending);
    let updated = mgr
        .get_by_id(&id)
        .cloned()
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Task {id} not found")))?;
    Ok(Json(updated))
}

/// POST /api/tasks/:id/resume — resume a paused task
pub async fn resume_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Task>, (StatusCode, String)> {
    let mut mgr = state.task_manager.lock();
    let task = mgr
        .get_by_id(&id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Task {id} not found")))?;

    if task.status != HydraTaskStatus::Pending {
        return Err((
            StatusCode::CONFLICT,
            format!("Task {id} is not paused (current status: {})", task.status.as_str()),
        ));
    }

    mgr.update_status(&id, HydraTaskStatus::Active);
    let updated = mgr
        .get_by_id(&id)
        .cloned()
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Task {id} not found")))?;
    Ok(Json(updated))
}

/// GET /api/tasks/:id/status — get task progress and status
pub async fn task_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<TaskStatusResponse>, (StatusCode, String)> {
    let mgr = state.task_manager.lock();
    let task = mgr
        .get_by_id(&id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Task {id} not found")))?;

    Ok(Json(TaskStatusResponse {
        id: task.id.clone(),
        status: task.status,
        title: task.title.clone(),
        created_at: task.created_at.to_rfc3339(),
        completed_at: task.completed_at.map(|t| t.to_rfc3339()),
        run_id: task.run_id.clone(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_tasks_path() {
        assert_eq!(TaskRoutes::list_tasks(), "/api/tasks");
    }

    #[test]
    fn test_create_task_path() {
        assert_eq!(TaskRoutes::create_task(), "/api/tasks");
    }

    #[test]
    fn test_get_task_path() {
        assert_eq!(TaskRoutes::get_task(), "/api/tasks/:id");
    }

    #[test]
    fn test_update_task_path() {
        assert_eq!(TaskRoutes::update_task(), "/api/tasks/:id");
    }

    #[test]
    fn test_cancel_task_path() {
        assert_eq!(TaskRoutes::cancel_task(), "/api/tasks/:id");
    }

    #[test]
    fn test_pause_task_path() {
        assert_eq!(TaskRoutes::pause_task(), "/api/tasks/:id/pause");
    }

    #[test]
    fn test_resume_task_path() {
        assert_eq!(TaskRoutes::resume_task(), "/api/tasks/:id/resume");
    }

    #[test]
    fn test_task_status_path() {
        assert_eq!(TaskRoutes::task_status(), "/api/tasks/:id/status");
    }

    #[test]
    fn test_list_subtasks_path() {
        assert_eq!(TaskRoutes::list_subtasks(), "/api/tasks/:id/subtasks");
    }

    #[test]
    fn test_create_task_request_deserialization() {
        let json = serde_json::json!({
            "title": "Test Task",
            "description": "A test description"
        });
        let req: CreateTaskRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.title, "Test Task");
        assert_eq!(req.description, Some("A test description".into()));
    }

    #[test]
    fn test_create_task_request_no_description() {
        let json = serde_json::json!({"title": "Minimal"});
        let req: CreateTaskRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.title, "Minimal");
        assert!(req.description.is_none());
    }

    #[test]
    fn test_update_task_request_deserialization() {
        let json = serde_json::json!({"title": "New Title"});
        let req: UpdateTaskRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.title, Some("New Title".into()));
        assert!(req.status.is_none());
    }

    #[test]
    fn test_list_tasks_query_deserialization() {
        let json = serde_json::json!({"status": "active"});
        let q: ListTasksQuery = serde_json::from_value(json).unwrap();
        assert_eq!(q.status, Some("active".into()));
    }

    #[test]
    fn test_list_tasks_query_no_filter() {
        let json = serde_json::json!({});
        let q: ListTasksQuery = serde_json::from_value(json).unwrap();
        assert!(q.status.is_none());
    }

    #[test]
    fn test_deleted_response_serialization() {
        let resp = DeletedResponse { deleted: true };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["deleted"], true);
    }

    #[test]
    fn test_task_status_response_serialization() {
        let resp = TaskStatusResponse {
            id: "t-1".into(),
            status: HydraTaskStatus::Active,
            title: "Test".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            completed_at: None,
            run_id: Some("r-1".into()),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["id"], "t-1");
        assert_eq!(json["title"], "Test");
        assert!(json["completed_at"].is_null());
        assert_eq!(json["run_id"], "r-1");
    }
}

/// GET /api/tasks/:id/subtasks — list subtasks of a task
pub async fn list_subtasks(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<Task>>, (StatusCode, String)> {
    let mgr = state.task_manager.lock();

    // Verify parent task exists
    if mgr.get_by_id(&id).is_none() {
        return Err((StatusCode::NOT_FOUND, format!("Task {id} not found")));
    }

    let subtasks: Vec<Task> = mgr
        .all()
        .iter()
        .filter(|t| t.parent_id.as_deref() == Some(&id))
        .cloned()
        .collect();
    Ok(Json(subtasks))
}
