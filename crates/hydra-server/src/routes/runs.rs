use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use hydra_db::{RunRow, RunStatus};

use crate::executor;
use crate::state::AppState;

// ═══════════════════════════════════════════════════════════
// ROUTE PATHS
// ═══════════════════════════════════════════════════════════

pub struct RunRoutes;

impl RunRoutes {
    pub fn list_runs() -> &'static str {
        "/api/runs"
    }

    pub fn get_run() -> &'static str {
        "/api/runs/:id"
    }

    pub fn list_models() -> &'static str {
        "/api/models"
    }
}

// ═══════════════════════════════════════════════════════════
// REQUEST / RESPONSE TYPES
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct ExecuteRunRequest {
    pub intent: String,
    pub auto_approve: Option<bool>,
    pub model: Option<String>,
    pub dry_run: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct RunResponse {
    pub run_id: String,
    pub status: String,
    pub intent: String,
    pub phases_completed: Vec<String>,
    pub result: Option<String>,
}

// ═══════════════════════════════════════════════════════════
// HANDLERS
// ═══════════════════════════════════════════════════════════

/// POST /api/runs — execute an intent through the cognitive loop
pub async fn execute_run_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ExecuteRunRequest>,
) -> Result<Json<RunResponse>, (StatusCode, String)> {
    let run_id = Uuid::new_v4().to_string();
    let model = req.model.unwrap_or_else(|| "claude-sonnet-4-6".into());
    let now = Utc::now().to_rfc3339();

    // Dry-run mode: return immediately without executing
    if req.dry_run.unwrap_or(false) {
        return Ok(Json(RunResponse {
            run_id,
            status: "dry_run".into(),
            intent: req.intent,
            phases_completed: vec![
                "perceive".into(),
                "think".into(),
                "decide".into(),
                "act".into(),
                "learn".into(),
            ],
            result: Some("Dry run — no actions taken".into()),
        }));
    }

    // Create run in DB
    let run = RunRow {
        id: run_id.clone(),
        intent: req.intent.clone(),
        status: RunStatus::Pending,
        created_at: now.clone(),
        updated_at: now,
        completed_at: None,
        parent_run_id: None,
        metadata: Some(
            serde_json::json!({
                "model": model,
                "auto_approve": req.auto_approve.unwrap_or(false),
                "source": "rest_api",
            })
            .to_string(),
        ),
    };

    if let Err(e) = state.db.create_run(&run) {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to create run: {e}"),
        ));
    }

    // Spawn async cognitive loop execution (same pattern as RPC handler)
    let state_arc = Arc::new(AppState::new_from_shared(
        state.db.clone(),
        state.event_bus.clone(),
        state.ledger.clone(),
        state.server_mode,
        state.auth_token.clone(),
    ));
    let rid = run_id.clone();
    let intent = req.intent.clone();
    tokio::spawn(async move {
        executor::execute_run(state_arc, rid, intent).await;
    });

    // Return immediately — client should listen on /events SSE for progress
    Ok(Json(RunResponse {
        run_id,
        status: "pending".into(),
        intent: req.intent,
        phases_completed: vec![],
        result: Some(format!(
            "Run queued. Listen on /events for SSE progress (model: {})",
            model
        )),
    }))
}

/// GET /api/runs — list recent runs from database
pub async fn list_runs(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<serde_json::Value>>, (StatusCode, String)> {
    let runs = state.db.list_runs(None).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to list runs: {e}"),
        )
    })?;

    let values: Vec<serde_json::Value> = runs
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "intent": r.intent,
                "status": format!("{:?}", r.status),
                "created_at": r.created_at,
                "updated_at": r.updated_at,
                "completed_at": r.completed_at,
            })
        })
        .collect();

    Ok(Json(values))
}

/// GET /api/runs/:id — get a specific run
pub async fn get_run(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let run = state
        .db
        .get_run(&id)
        .map_err(|_| (StatusCode::NOT_FOUND, format!("Run {id} not found")))?;

    Ok(Json(serde_json::json!({
        "id": run.id,
        "intent": run.intent,
        "status": format!("{:?}", run.status),
        "created_at": run.created_at,
        "updated_at": run.updated_at,
        "completed_at": run.completed_at,
        "parent_run_id": run.parent_run_id,
        "metadata": run.metadata,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_runs_path() {
        assert_eq!(RunRoutes::list_runs(), "/api/runs");
    }

    #[test]
    fn test_get_run_path() {
        assert_eq!(RunRoutes::get_run(), "/api/runs/:id");
    }

    #[test]
    fn test_list_models_path() {
        assert_eq!(RunRoutes::list_models(), "/api/models");
    }

    #[test]
    fn test_execute_run_request_deserialization() {
        let json = serde_json::json!({
            "intent": "build something",
            "auto_approve": true,
            "model": "claude-opus-4-6",
            "dry_run": false
        });
        let req: ExecuteRunRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.intent, "build something");
        assert_eq!(req.auto_approve, Some(true));
        assert_eq!(req.model, Some("claude-opus-4-6".into()));
        assert_eq!(req.dry_run, Some(false));
    }

    #[test]
    fn test_execute_run_request_minimal() {
        let json = serde_json::json!({"intent": "hello"});
        let req: ExecuteRunRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.intent, "hello");
        assert!(req.auto_approve.is_none());
        assert!(req.model.is_none());
        assert!(req.dry_run.is_none());
    }

    #[test]
    fn test_run_response_serialization() {
        let resp = RunResponse {
            run_id: "r-1".into(),
            status: "pending".into(),
            intent: "do something".into(),
            phases_completed: vec!["perceive".into(), "think".into()],
            result: Some("success".into()),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["run_id"], "r-1");
        assert_eq!(json["phases_completed"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_run_response_no_result() {
        let resp = RunResponse {
            run_id: "r-2".into(),
            status: "running".into(),
            intent: "test".into(),
            phases_completed: vec![],
            result: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert!(json["result"].is_null());
    }
}

/// GET /api/models — list available models
pub async fn list_models() -> Json<Vec<serde_json::Value>> {
    Json(vec![
        serde_json::json!({"id": "claude-sonnet-4-6", "name": "Claude Sonnet 4.6", "provider": "anthropic", "default": true}),
        serde_json::json!({"id": "claude-opus-4-6", "name": "Claude Opus 4.6", "provider": "anthropic"}),
        serde_json::json!({"id": "claude-haiku-4-5", "name": "Claude Haiku 4.5", "provider": "anthropic"}),
        serde_json::json!({"id": "gpt-4o", "name": "GPT-4o", "provider": "openai"}),
        serde_json::json!({"id": "gpt-4o-mini", "name": "GPT-4o Mini", "provider": "openai"}),
        serde_json::json!({"id": "gemini-2.0-flash", "name": "Gemini 2.0 Flash", "provider": "google"}),
    ])
}
