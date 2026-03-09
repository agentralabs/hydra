use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use hydra_db::{ApprovalRow, StepRow};
use hydra_runtime::approval::ApprovalDecision;

use crate::state::AppState;

// ═══════════════════════════════════════════════════════════
// ROUTE PATHS
// ═══════════════════════════════════════════════════════════

pub struct SystemRoutes;

impl SystemRoutes {
    /// GET: system status overview
    pub fn system_status() -> &'static str {
        "/api/system/status"
    }

    /// GET: list steps for a run
    pub fn list_steps() -> &'static str {
        "/api/steps"
    }

    /// GET: list pending approvals
    pub fn list_approvals() -> &'static str {
        "/api/approvals"
    }

    /// POST: approve an approval request
    pub fn approve() -> &'static str {
        "/api/approvals/:id/approve"
    }

    /// POST: deny an approval request
    pub fn deny() -> &'static str {
        "/api/approvals/:id/deny"
    }

    /// POST: cancel a run
    pub fn cancel_run() -> &'static str {
        "/api/runs/:id/cancel"
    }

    /// POST: approve a run
    pub fn approve_run() -> &'static str {
        "/api/runs/:id/approve"
    }

    /// GET: get run status
    pub fn run_status() -> &'static str {
        "/api/runs/:id/status"
    }

    /// POST: kill a run
    pub fn kill_run() -> &'static str {
        "/api/runs/:id/kill"
    }

    /// GET: current trust levels
    pub fn trust() -> &'static str {
        "/api/system/trust"
    }

    /// GET: invention stats
    pub fn inventions() -> &'static str {
        "/api/system/inventions"
    }

    /// GET: budget usage stats
    pub fn budget() -> &'static str {
        "/api/system/budget"
    }

    /// GET: receipt ledger stats
    pub fn receipts() -> &'static str {
        "/api/system/receipts"
    }

    /// GET: offline status
    pub fn offline() -> &'static str {
        "/api/system/offline"
    }
}

// ═══════════════════════════════════════════════════════════
// REQUEST / RESPONSE TYPES
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Serialize)]
pub struct SystemStatusResponse {
    pub uptime_secs: u64,
    pub degradation_level: String,
    pub kill_switch_active: bool,
    pub kill_switch_frozen: bool,
    pub kill_switch_reason: Option<String>,
    pub pending_approvals: usize,
    pub server_mode: bool,
    pub active_runs: usize,
    pub total_runs: usize,
    pub sisters: SistersStatus,
    pub autonomy_level: String,
    pub federation: FederationStatus,
    pub events_published: u64,
}

#[derive(Debug, Serialize)]
pub struct SistersStatus {
    pub memory: &'static str,
    pub identity: &'static str,
    pub codebase: &'static str,
    pub vision: &'static str,
    pub time: &'static str,
}

#[derive(Debug, Serialize)]
pub struct FederationStatus {
    pub enabled: bool,
    pub peers_connected: usize,
}

#[derive(Debug, Deserialize)]
pub struct StepsQuery {
    pub run_id: String,
}

#[derive(Debug, Serialize)]
pub struct RunStatusResponse {
    pub id: String,
    pub status: String,
    pub intent: String,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DenyRequest {
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ApprovalActionResponse {
    pub id: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct RunActionResponse {
    pub run_id: String,
    pub status: String,
}

// ═══════════════════════════════════════════════════════════
// HANDLERS
// ═══════════════════════════════════════════════════════════

fn map_db_err(e: hydra_db::DbError) -> (StatusCode, String) {
    match &e {
        hydra_db::DbError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

/// GET /api/system/status — comprehensive system status overview
pub async fn system_status(
    State(state): State<Arc<AppState>>,
) -> Json<SystemStatusResponse> {
    let degradation_level = format!("{}", state.degradation_manager.level());
    let pending_approvals = state.approval_manager.pending_count();

    // Count active and total runs from DB
    let all_runs = state.db.list_runs(None).unwrap_or_default();
    let total_runs = all_runs.len();
    let active_runs = all_runs
        .iter()
        .filter(|r| {
            r.status == hydra_db::RunStatus::Running || r.status == hydra_db::RunStatus::Pending
        })
        .count();

    Json(SystemStatusResponse {
        uptime_secs: state.uptime().as_secs(),
        degradation_level,
        kill_switch_active: state.kill_switch.is_active(),
        kill_switch_frozen: state.kill_switch.is_frozen(),
        kill_switch_reason: state.kill_switch.reason(),
        pending_approvals,
        server_mode: state.server_mode,
        active_runs,
        total_runs,
        sisters: SistersStatus {
            memory: "not_connected",
            identity: "not_connected",
            codebase: "not_connected",
            vision: "not_connected",
            time: "not_connected",
        },
        autonomy_level: "supervised".into(),
        federation: FederationStatus {
            enabled: false,
            peers_connected: 0,
        },
        events_published: state.event_bus.total_published(),
    })
}

/// GET /api/steps?run_id=X — list steps for a run
pub async fn list_steps(
    State(state): State<Arc<AppState>>,
    Query(params): Query<StepsQuery>,
) -> Result<Json<Vec<StepRow>>, (StatusCode, String)> {
    let steps = state.db.list_steps(&params.run_id).map_err(map_db_err)?;
    Ok(Json(steps))
}

/// GET /api/approvals — list pending approvals from DB
pub async fn list_approvals(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ApprovalRow>>, (StatusCode, String)> {
    let approvals = state.db.list_pending_approvals().map_err(map_db_err)?;
    Ok(Json(approvals))
}

/// POST /api/approvals/:id/approve — approve an approval request
pub async fn approve_approval(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApprovalActionResponse>, (StatusCode, String)> {
    // Update in DB
    state
        .db
        .update_approval_status(&id, hydra_db::ApprovalStatus::Approved)
        .map_err(map_db_err)?;

    // Submit to runtime approval manager if pending
    let _ = state
        .approval_manager
        .submit_decision(&id, ApprovalDecision::Approved);

    Ok(Json(ApprovalActionResponse {
        id,
        status: "approved".into(),
    }))
}

/// POST /api/approvals/:id/deny — deny an approval request
pub async fn deny_approval(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    body: Option<Json<DenyRequest>>,
) -> Result<Json<ApprovalActionResponse>, (StatusCode, String)> {
    // Update in DB
    state
        .db
        .update_approval_status(&id, hydra_db::ApprovalStatus::Denied)
        .map_err(map_db_err)?;

    // Submit to runtime approval manager if pending
    let reason = body
        .and_then(|b| b.reason.clone())
        .unwrap_or_else(|| "Denied via REST API".into());
    let _ = state
        .approval_manager
        .submit_decision(&id, ApprovalDecision::Denied { reason });

    Ok(Json(ApprovalActionResponse {
        id,
        status: "denied".into(),
    }))
}

/// POST /api/runs/:id/cancel — cancel a run
pub async fn cancel_run(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<RunActionResponse>, (StatusCode, String)> {
    let now = Utc::now().to_rfc3339();
    state
        .db
        .update_run_status(&id, hydra_db::RunStatus::Cancelled, Some(&now))
        .map_err(map_db_err)?;

    Ok(Json(RunActionResponse {
        run_id: id,
        status: "cancelled".into(),
    }))
}

/// POST /api/runs/:id/approve — approve a run (auto-approve all pending approvals for this run)
pub async fn approve_run(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<RunActionResponse>, (StatusCode, String)> {
    // Verify run exists
    state
        .db
        .get_run(&id)
        .map_err(|_| (StatusCode::NOT_FOUND, format!("Run {id} not found")))?;

    // Approve all pending approvals for this run in the runtime manager
    let pending = state.approval_manager.list_pending();
    for req in pending {
        if req.run_id == id {
            let _ = state
                .approval_manager
                .submit_decision(&req.id, ApprovalDecision::Approved);
        }
    }

    Ok(Json(RunActionResponse {
        run_id: id,
        status: "approved".into(),
    }))
}

/// GET /api/runs/:id/status — get run status
pub async fn run_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<RunStatusResponse>, (StatusCode, String)> {
    let run = state
        .db
        .get_run(&id)
        .map_err(|_| (StatusCode::NOT_FOUND, format!("Run {id} not found")))?;

    Ok(Json(RunStatusResponse {
        id: run.id,
        status: run.status.as_str().into(),
        intent: run.intent,
        created_at: run.created_at,
        updated_at: run.updated_at,
        completed_at: run.completed_at,
    }))
}

/// POST /api/runs/:id/kill — kill a run via kill switch
#[cfg(test)]
mod tests {
    use super::*;

    // ── Route path tests ───────────────────────────────────

    #[test]
    fn test_system_status_path() {
        assert_eq!(SystemRoutes::system_status(), "/api/system/status");
    }

    #[test]
    fn test_list_steps_path() {
        assert_eq!(SystemRoutes::list_steps(), "/api/steps");
    }

    #[test]
    fn test_list_approvals_path() {
        assert_eq!(SystemRoutes::list_approvals(), "/api/approvals");
    }

    #[test]
    fn test_approve_path() {
        assert_eq!(SystemRoutes::approve(), "/api/approvals/:id/approve");
    }

    #[test]
    fn test_deny_path() {
        assert_eq!(SystemRoutes::deny(), "/api/approvals/:id/deny");
    }

    #[test]
    fn test_cancel_run_path() {
        assert_eq!(SystemRoutes::cancel_run(), "/api/runs/:id/cancel");
    }

    #[test]
    fn test_approve_run_path() {
        assert_eq!(SystemRoutes::approve_run(), "/api/runs/:id/approve");
    }

    #[test]
    fn test_run_status_path() {
        assert_eq!(SystemRoutes::run_status(), "/api/runs/:id/status");
    }

    #[test]
    fn test_kill_run_path() {
        assert_eq!(SystemRoutes::kill_run(), "/api/runs/:id/kill");
    }

    // ── Response type tests ────────────────────────────────

    #[test]
    fn test_system_status_response_serialization() {
        let resp = SystemStatusResponse {
            uptime_secs: 120,
            degradation_level: "normal".into(),
            kill_switch_active: false,
            kill_switch_frozen: false,
            kill_switch_reason: None,
            pending_approvals: 3,
            server_mode: true,
            active_runs: 2,
            total_runs: 10,
            sisters: SistersStatus {
                memory: "not_connected",
                identity: "not_connected",
                codebase: "not_connected",
                vision: "not_connected",
                time: "not_connected",
            },
            autonomy_level: "supervised".into(),
            federation: FederationStatus {
                enabled: false,
                peers_connected: 0,
            },
            events_published: 42,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["uptime_secs"], 120);
        assert_eq!(json["pending_approvals"], 3);
        assert_eq!(json["server_mode"], true);
        assert_eq!(json["active_runs"], 2);
        assert_eq!(json["total_runs"], 10);
        assert_eq!(json["sisters"]["memory"], "not_connected");
        assert_eq!(json["autonomy_level"], "supervised");
        assert_eq!(json["federation"]["enabled"], false);
        assert_eq!(json["events_published"], 42);
    }

    #[test]
    fn test_run_status_response_serialization() {
        let resp = RunStatusResponse {
            id: "run-123".into(),
            status: "running".into(),
            intent: "test".into(),
            created_at: "2026-01-01".into(),
            updated_at: "2026-01-01".into(),
            completed_at: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["id"], "run-123");
        assert!(json["completed_at"].is_null());
    }

    #[test]
    fn test_approval_action_response_serialization() {
        let resp = ApprovalActionResponse {
            id: "appr-1".into(),
            status: "approved".into(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["id"], "appr-1");
        assert_eq!(json["status"], "approved");
    }

    #[test]
    fn test_run_action_response_serialization() {
        let resp = RunActionResponse {
            run_id: "run-456".into(),
            status: "cancelled".into(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["run_id"], "run-456");
        assert_eq!(json["status"], "cancelled");
    }

    #[test]
    fn test_steps_query_deserialization() {
        let json = serde_json::json!({"run_id": "run-789"});
        let q: StepsQuery = serde_json::from_value(json).unwrap();
        assert_eq!(q.run_id, "run-789");
    }

    #[test]
    fn test_deny_request_deserialization() {
        let json = serde_json::json!({"reason": "too risky"});
        let d: DenyRequest = serde_json::from_value(json).unwrap();
        assert_eq!(d.reason, Some("too risky".into()));
    }

    #[test]
    fn test_deny_request_no_reason() {
        let json = serde_json::json!({});
        let d: DenyRequest = serde_json::from_value(json).unwrap();
        assert!(d.reason.is_none());
    }

    #[test]
    fn test_budget_path() {
        assert_eq!(SystemRoutes::budget(), "/api/system/budget");
    }

    #[test]
    fn test_receipts_path() {
        assert_eq!(SystemRoutes::receipts(), "/api/system/receipts");
    }

    #[test]
    fn test_offline_path() {
        assert_eq!(SystemRoutes::offline(), "/api/system/offline");
    }
}

/// GET /api/system/trust — return current trust levels
pub async fn get_trust(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let trust_score = state.decide_engine.current_trust();
    let autonomy_level = state.decide_engine.current_level();
    Json(serde_json::json!({
        "trust_score": trust_score,
        "autonomy_level": format!("{:?}", autonomy_level),
    }))
}

/// GET /api/system/inventions — return comprehensive invention stats
pub async fn get_inventions(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let inv = &state.invention_engine;
    Json(serde_json::json!({
        "idle_time": inv.idle_time(),
        "skills_crystallized": inv.skill_count(),
        "patterns_tracked": inv.pattern_count(),
        "reflections": inv.reflection_count(),
        "dream_active": inv.idle_time() >= 60,
        "shadow_validator": "active",
        "future_echo": "active",
        "context_compression": "active",
        "semantic_dedup": "active",
        "temporal_memory": "active",
        "pattern_mutation": "active",
        "evolution_engine": "active",
        "crystallization": "active",
        "metacognition": "active",
    }))
}

/// GET /api/system/budget — return budget usage stats
pub async fn get_budget(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(serde_json::json!({
        "total_budget": 100000,
        "conservation_mode": false,
        "active_runs": state.db.list_runs(Some(hydra_db::RunStatus::Running)).unwrap_or_default().len(),
    }))
}

/// GET /api/system/receipts — return receipt ledger stats
pub async fn get_receipts(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let total = state.ledger.len();
    let verification = state.ledger.verify_chain();
    Json(serde_json::json!({
        "total_receipts": total,
        "chain_valid": verification.is_valid(),
        "chain_status": format!("{:?}", verification.status),
        "verified_receipts": verification.verified_receipts,
        "ledger_active": true,
    }))
}

/// GET /api/system/offline — return offline/connectivity status
pub async fn get_offline(_state: State<Arc<AppState>>) -> impl IntoResponse {
    Json(serde_json::json!({
        "online": true,
        "offline_mode_available": true,
        "pending_sync_count": 0,
    }))
}

pub async fn kill_run(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<RunActionResponse>, (StatusCode, String)> {
    // Verify run exists
    state
        .db
        .get_run(&id)
        .map_err(|_| (StatusCode::NOT_FOUND, format!("Run {id} not found")))?;

    // Update run status to cancelled
    let now = Utc::now().to_rfc3339();
    state
        .db
        .update_run_status(&id, hydra_db::RunStatus::Cancelled, Some(&now))
        .map_err(map_db_err)?;

    // Cancel any pending approvals for this run
    let pending = state.approval_manager.list_pending();
    for req in pending {
        if req.run_id == id {
            let _ = state.approval_manager.submit_decision(
                &req.id,
                ApprovalDecision::Denied {
                    reason: "Run killed".into(),
                },
            );
        }
    }

    Ok(Json(RunActionResponse {
        run_id: id,
        status: "killed".into(),
    }))
}
