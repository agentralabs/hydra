use std::sync::Arc;

use chrono::Utc;
use hydra_db::{ApprovalStatus, RunRow, RunStatus};
use hydra_runtime::jsonrpc::{JsonRpcRequest, JsonRpcResponse, RpcErrorCodes};
use hydra_runtime::sse::{SseEvent, SseEventType};
use uuid::Uuid;

use crate::executor;
use crate::state::AppState;

/// Handle a JSON-RPC 2.0 request
pub async fn handle_rpc(state: &AppState, body: &str) -> JsonRpcResponse {
    // Parse
    let req: JsonRpcRequest = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(_) => {
            return JsonRpcResponse::error(
                serde_json::Value::Null,
                RpcErrorCodes::PARSE_ERROR,
                "Parse error. Request body is not valid JSON.",
            );
        }
    };

    // Validate
    if !req.is_valid() {
        return JsonRpcResponse::error(
            req.id,
            RpcErrorCodes::INVALID_REQUEST,
            "Invalid request. Must include jsonrpc: \"2.0\" and a non-empty method.",
        );
    }

    // Dispatch
    match req.method.as_str() {
        "hydra.run" => handle_run(state, &req).await,
        "hydra.cancel" => handle_cancel(state, &req).await,
        "hydra.approve" => handle_approve(state, &req).await,
        "hydra.status" => handle_status(state, &req).await,
        "hydra.health" => handle_health(state, &req).await,
        "hydra.kill" => handle_kill(state, &req).await,
        "hydra.profile.list" => handle_profile_list(state, &req),
        "hydra.profile.load" => handle_profile_load(state, &req),
        "hydra.profile.unload" => handle_profile_unload(state, &req),
        "hydra.roi" => handle_roi(state, &req),
        _ => JsonRpcResponse::error(
            req.id,
            RpcErrorCodes::METHOD_NOT_FOUND,
            format!("Method '{}' not found.", req.method),
        ),
    }
}

async fn handle_run(state: &AppState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let intent = match req.params.get("intent").and_then(|v| v.as_str()) {
        Some(i) => i.to_string(),
        None => {
            return JsonRpcResponse::error(
                req.id.clone(),
                RpcErrorCodes::INVALID_PARAMS,
                "Missing required parameter 'intent'. Provide what you want Hydra to do.",
            );
        }
    };

    let run_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    // Create run in DB with Pending status
    let run = RunRow {
        id: run_id.clone(),
        intent: intent.clone(),
        status: RunStatus::Pending,
        created_at: now.clone(),
        updated_at: now.clone(),
        completed_at: None,
        parent_run_id: None,
        metadata: None,
    };

    if let Err(e) = state.db.create_run(&run) {
        return JsonRpcResponse::error(
            req.id.clone(),
            RpcErrorCodes::INTERNAL_ERROR,
            format!("Failed to create run. {e}"),
        );
    }

    // Spawn async cognitive loop execution
    // The executor handles: SSE events, DB updates, receipt generation
    let state_arc = Arc::new(AppState::new_from_shared(
        state.db.clone(),
        state.event_bus.clone(),
        state.ledger.clone(),
        state.server_mode,
        state.auth_token.clone(),
    ));
    let rid = run_id.clone();
    let i = intent.clone();
    tokio::spawn(async move {
        executor::execute_run(state_arc, rid, i).await;
    });

    // Return immediately with run_id
    JsonRpcResponse::success(
        req.id.clone(),
        serde_json::json!({
            "run_id": run_id,
            "status": "accepted",
        }),
    )
}

async fn handle_cancel(state: &AppState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let run_id = match req.params.get("run_id").and_then(|v| v.as_str()) {
        Some(id) => id,
        None => {
            return JsonRpcResponse::error(
                req.id.clone(),
                RpcErrorCodes::INVALID_PARAMS,
                "Missing required parameter 'run_id'.",
            );
        }
    };

    match state.db.get_run(run_id) {
        Ok(run) => {
            if run.status == RunStatus::Completed || run.status == RunStatus::Cancelled {
                return JsonRpcResponse::error(
                    req.id.clone(),
                    RpcErrorCodes::INTERNAL_ERROR,
                    format!("Run is already {}.", run.status.as_str()),
                );
            }
            let now = Utc::now().to_rfc3339();
            let _ = state
                .db
                .update_run_status(run_id, RunStatus::Cancelled, Some(&now));
            JsonRpcResponse::success(req.id.clone(), serde_json::json!({"success": true}))
        }
        Err(_) => JsonRpcResponse::error(
            req.id.clone(),
            RpcErrorCodes::INTERNAL_ERROR,
            format!("Run '{run_id}' not found."),
        ),
    }
}

async fn handle_approve(state: &AppState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let approval_id = match req.params.get("approval_id").and_then(|v| v.as_str()) {
        Some(id) => id,
        None => {
            return JsonRpcResponse::error(
                req.id.clone(),
                RpcErrorCodes::INVALID_PARAMS,
                "Missing required parameter 'approval_id'.",
            );
        }
    };

    let decision = match req.params.get("decision").and_then(|v| v.as_str()) {
        Some(d) => d,
        None => {
            return JsonRpcResponse::error(
                req.id.clone(),
                RpcErrorCodes::INVALID_PARAMS,
                "Missing required parameter 'decision'. Use 'approved' or 'denied'.",
            );
        }
    };

    let status = match decision {
        "approved" | "approve" => ApprovalStatus::Approved,
        "denied" | "deny" => ApprovalStatus::Denied,
        _ => {
            return JsonRpcResponse::error(
                req.id.clone(),
                RpcErrorCodes::INVALID_PARAMS,
                "Invalid decision. Use 'approved' or 'denied'.",
            );
        }
    };

    match state.db.update_approval_status(approval_id, status) {
        Ok(()) => JsonRpcResponse::success(req.id.clone(), serde_json::json!({"success": true})),
        Err(e) => JsonRpcResponse::error(
            req.id.clone(),
            RpcErrorCodes::INTERNAL_ERROR,
            format!("Approval failed. {e}"),
        ),
    }
}

async fn handle_status(state: &AppState, req: &JsonRpcRequest) -> JsonRpcResponse {
    if let Some(run_id) = req.params.get("run_id").and_then(|v| v.as_str()) {
        match state.db.get_run(run_id) {
            Ok(run) => {
                let steps = state.db.list_steps(run_id).unwrap_or_default();
                JsonRpcResponse::success(
                    req.id.clone(),
                    serde_json::json!({
                        "runs": [{
                            "id": run.id,
                            "intent": run.intent,
                            "status": run.status.as_str(),
                            "created_at": run.created_at,
                            "steps": steps.iter().map(|s| serde_json::json!({
                                "id": s.id,
                                "description": s.description,
                                "status": s.status.as_str(),
                            })).collect::<Vec<_>>(),
                        }]
                    }),
                )
            }
            Err(_) => JsonRpcResponse::error(
                req.id.clone(),
                RpcErrorCodes::INTERNAL_ERROR,
                format!("Run '{run_id}' not found."),
            ),
        }
    } else {
        let runs = state.db.list_runs(None).unwrap_or_default();
        let runs_json: Vec<_> = runs
            .iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.id,
                    "intent": r.intent,
                    "status": r.status.as_str(),
                    "created_at": r.created_at,
                })
            })
            .collect();
        JsonRpcResponse::success(req.id.clone(), serde_json::json!({"runs": runs_json}))
    }
}

async fn handle_kill(state: &AppState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let level = req
        .params
        .get("level")
        .and_then(|v| v.as_str())
        .unwrap_or("graceful");

    let reason = req
        .params
        .get("reason")
        .and_then(|v| v.as_str())
        .unwrap_or("User requested kill");

    // Emit SSE event for desktop UI
    state.event_bus.publish(SseEvent::new(
        SseEventType::RunError,
        serde_json::json!({
            "type": "kill_switch",
            "level": level,
            "reason": reason,
        }),
    ));

    // Cancel all runs in DB
    let runs = state.db.list_runs(None).unwrap_or_default();
    let mut cancelled = 0;
    for run in &runs {
        if run.status == RunStatus::Running || run.status == RunStatus::Pending {
            let now = Utc::now().to_rfc3339();
            let _ = state
                .db
                .update_run_status(&run.id, RunStatus::Cancelled, Some(&now));
            cancelled += 1;
        }
    }

    JsonRpcResponse::success(
        req.id.clone(),
        serde_json::json!({
            "success": true,
            "level": level,
            "cancelled_runs": cancelled,
        }),
    )
}

async fn handle_health(state: &AppState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let uptime = state.uptime().as_secs();
    let sisters_status = if let Some(ref sh) = state.sisters {
        sh.status_summary()
    } else { "not initialized".into() };
    let profile_name = state.active_profile.lock().as_ref().map(|p| p.name.clone());
    let beliefs_count = state.active_profile.lock().as_ref().map(|p| p.beliefs.len()).unwrap_or(0);
    JsonRpcResponse::success(req.id.clone(), serde_json::json!({
        "status": "ok", "uptime_seconds": uptime, "engine": "full",
        "sisters": sisters_status, "sisters_count": 17,
        "profile": profile_name, "beliefs_loaded": beliefs_count,
    }))
}

fn handle_profile_list(_state: &AppState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let profiles = hydra_native::operational_profile::list_profiles();
    let active = _state.active_profile.lock().as_ref().map(|p| p.name.clone());
    let list: Vec<serde_json::Value> = profiles.iter().map(|name| {
        let counts = hydra_native::cognitive::profile_loader::load_profile(name)
            .map(|p| (p.beliefs.len(), p.skills.len())).unwrap_or((0, 0));
        serde_json::json!({"name": name, "active": active.as_deref() == Some(name.as_str()),
            "beliefs": counts.0, "skills": counts.1})
    }).collect();
    JsonRpcResponse::success(req.id.clone(), serde_json::json!({"profiles": list}))
}

fn handle_profile_load(state: &AppState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let name = match req.params.get("name").and_then(|v| v.as_str()) {
        Some(n) => n,
        None => return JsonRpcResponse::error(req.id.clone(), RpcErrorCodes::INVALID_PARAMS, "Missing 'name'"),
    };
    match state.load_profile(name) {
        Ok(()) => {
            let beliefs = state.active_profile.lock().as_ref().map(|p| p.beliefs.len()).unwrap_or(0);
            JsonRpcResponse::success(req.id.clone(), serde_json::json!({"loaded": name, "beliefs": beliefs}))
        }
        Err(e) => JsonRpcResponse::error(req.id.clone(), RpcErrorCodes::INTERNAL_ERROR, e),
    }
}

fn handle_profile_unload(state: &AppState, req: &JsonRpcRequest) -> JsonRpcResponse {
    *state.active_profile.lock() = None;
    *state.prompt_overlay.lock() = None;
    JsonRpcResponse::success(req.id.clone(), serde_json::json!({"unloaded": true}))
}

fn handle_roi(_state: &AppState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let summary = hydra_native::knowledge::economics_tracker::roi_summary();
    JsonRpcResponse::success(req.id.clone(), serde_json::json!({"roi": summary}))
}
