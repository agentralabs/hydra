use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use chrono::Utc;

use hydra_db::{ApprovalRow, StepRow};
use hydra_runtime::approval::ApprovalDecision;

use crate::state::AppState;

use super::{
    ApprovalActionResponse, DenyRequest, FederationStatus, RunActionResponse, RunStatusResponse,
    SistersStatus, StepsQuery, SystemStatusResponse,
};

// ═══════════════════════════════════════════════════════════
// HANDLERS
// ═══════════════════════════════════════════════════════════

pub(crate) fn map_db_err(e: hydra_db::DbError) -> (StatusCode, String) {
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

/// POST /api/runs/:id/kill — kill a run via kill switch
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
