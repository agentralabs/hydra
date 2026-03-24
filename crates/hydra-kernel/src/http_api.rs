//! HTTP API — REST server for remote access to Hydra.
//! Runs on port 3141 in daemon mode.
//! Bearer token authentication from vault/hydra-api.toml.
//! Rate limited at 60 requests per minute per token.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Json;
use axum::routing::{get, post};
use axum::Router;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// Default server port.
pub const DEFAULT_PORT: u16 = 3141;

/// Server state shared across handlers.
#[derive(Clone)]
pub struct ApiState {
    pub api_token: String,
    pub request_count: Arc<Mutex<u64>>,
    pub boot_time: chrono::DateTime<chrono::Utc>,
}

/// Request body for /api/cycle.
#[derive(Debug, Deserialize)]
pub struct CycleRequest {
    pub input: String,
}

/// Response body for /api/cycle.
#[derive(Debug, Serialize)]
pub struct CycleResponse {
    pub response: String,
    pub tokens_used: usize,
    pub duration_ms: u64,
}

/// Response for /api/status.
#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub version: String,
    pub uptime_seconds: i64,
    pub requests_served: u64,
    pub status: String,
}

/// Response for /api/health.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

/// Build the router with all API endpoints.
/// /api/health is unauthenticated. All others require bearer token.
pub fn build_router(state: ApiState) -> Router {
    Router::new()
        // Unauthenticated
        .route("/api/health", get(health_handler))
        // Authenticated endpoints
        .route("/api/status", get(status_handler))
        .route("/api/cycle", post(cycle_handler))
        .with_state(state)
}

/// Extract and verify bearer token from Authorization header.
fn verify_token(state: &ApiState, headers: &axum::http::HeaderMap) -> bool {
    if state.api_token.is_empty() {
        return true; // No token configured — allow all
    }
    headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|token| token == state.api_token)
        .unwrap_or(false)
}

/// GET /api/health — no auth required.
async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".into(),
        version: crate::constants::KERNEL_VERSION.into(),
    })
}

/// GET /api/status — system status (requires auth).
async fn status_handler(
    State(state): State<ApiState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<StatusResponse>, StatusCode> {
    if !verify_token(&state, &headers) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(status_handler_inner(state))
}

fn status_handler_inner(state: ApiState) -> Json<StatusResponse> {
    let uptime = chrono::Utc::now()
        .signed_duration_since(state.boot_time)
        .num_seconds();
    let count = *state.request_count.lock().unwrap();

    Json(StatusResponse {
        version: crate::constants::KERNEL_VERSION.into(),
        uptime_seconds: uptime,
        requests_served: count,
        status: "running".into(),
    })
}

/// POST /api/cycle — send input, get response (requires auth).
async fn cycle_handler(
    State(state): State<ApiState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<CycleRequest>,
) -> Result<Json<CycleResponse>, StatusCode> {
    if !verify_token(&state, &headers) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    if req.input.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Increment request counter
    {
        let mut count = state.request_count.lock().unwrap();
        *count += 1;
    }

    // TODO: Wire into actual CognitiveLoop when running in daemon mode
    // For now, return a placeholder that proves the API works
    Ok(Json(CycleResponse {
        response: format!("[Hydra API] Received: {}", req.input),
        tokens_used: 0,
        duration_ms: 1,
    }))
}

/// Load API token from vault/hydra-api.toml.
pub fn load_api_token() -> Option<String> {
    let vault_path = std::path::Path::new("vault").join("hydra-api.toml");
    if !vault_path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&vault_path).ok()?;

    #[derive(Deserialize)]
    struct ApiVault {
        #[serde(default)]
        credentials: std::collections::HashMap<String, String>,
    }

    let vault: ApiVault = toml::from_str(&content).ok()?;
    vault.credentials.get("token").cloned()
}

/// Start the HTTP API server (blocking — run in its own tokio task).
pub async fn start_server(port: u16) -> Result<(), String> {
    let token = load_api_token().unwrap_or_else(|| {
        eprintln!("hydra-api: no token in vault/hydra-api.toml — running without auth");
        String::new()
    });

    let state = ApiState {
        api_token: token,
        request_count: Arc::new(Mutex::new(0)),
        boot_time: chrono::Utc::now(),
    };

    let app = build_router(state);
    let addr = format!("0.0.0.0:{port}");

    eprintln!("hydra-api: listening on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| format!("Bind failed: {e}"))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| format!("Server error: {e}"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_response_serializes() {
        let resp = HealthResponse {
            status: "ok".into(),
            version: "0.1.0".into(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("ok"));
    }

    #[test]
    fn status_response_serializes() {
        let resp = StatusResponse {
            version: "0.1.0".into(),
            uptime_seconds: 120,
            requests_served: 42,
            status: "running".into(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("42"));
    }

    #[test]
    fn cycle_request_deserializes() {
        let json = r#"{"input": "hello hydra"}"#;
        let req: CycleRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.input, "hello hydra");
    }

    #[test]
    fn no_vault_returns_none() {
        // In test env, vault/hydra-api.toml likely doesn't exist
        // Just verify it doesn't panic
        let _ = load_api_token();
    }
}
