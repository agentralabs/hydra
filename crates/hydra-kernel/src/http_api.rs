//! HTTP API — REST + Remote Presence server for Hydra.
//! Port 3141: REST API (daemon mode). Port 7476: Remote Presence (O18).
//! Bearer token auth for API, PIN auth for remote WebSocket.

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Json};
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
    /// O18: Remote presence server (PIN auth, client pool, WebSocket).
    pub remote: Arc<crate::remote::RemoteServer>,
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
        // Authenticated REST endpoints
        .route("/api/status", get(status_handler))
        .route("/api/cycle", post(cycle_handler))
        // O18: Remote presence
        .route("/remote", get(remote_page_handler))
        .route("/ws", get(ws_upgrade_handler))
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

// ── O18 Remote Presence Handlers ──

/// GET /remote — serve the web chat interface.
async fn remote_page_handler() -> Html<&'static str> {
    Html(crate::remote::remote_page_html())
}

/// GET /ws — WebSocket upgrade for remote clients.
async fn ws_upgrade_handler(
    ws: WebSocketUpgrade,
    State(state): State<ApiState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws_connection(socket, state.remote))
}

/// Handle a single WebSocket connection lifecycle.
async fn handle_ws_connection(mut socket: WebSocket, remote: Arc<crate::remote::RemoteServer>) {
    let client_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let client_ip = "unknown".to_string(); // Axum doesn't expose IP easily without ConnectInfo
    let mut authenticated = false;

    while let Some(Ok(msg)) = socket.recv().await {
        let text = match msg {
            Message::Text(t) => t,
            Message::Close(_) => break,
            _ => continue,
        };

        let client_msg: crate::remote::ClientMessage = match serde_json::from_str(&text) {
            Ok(m) => m,
            Err(_) => {
                let err = crate::remote::ServerMessage::Error { message: "Invalid message format".into() };
                let _ = socket.send(Message::Text(err.to_json().into())).await;
                continue;
            }
        };

        // Auth gate: first message must be Auth
        if !authenticated {
            if let crate::remote::ClientMessage::Auth { ref pin } = client_msg {
                match remote.verify_pin(pin, &client_ip) {
                    Ok(()) => {
                        authenticated = true;
                        let added = remote.add_client(crate::remote::RemoteClient {
                            id: client_id.clone(),
                            ip: client_ip.clone(),
                            connected_at: chrono::Utc::now(),
                            authenticated: true,
                        });
                        let resp = if added {
                            crate::remote::ServerMessage::AuthResult { success: true, reason: None }
                        } else {
                            crate::remote::ServerMessage::AuthResult {
                                success: false,
                                reason: Some("Server at capacity".into()),
                            }
                        };
                        let _ = socket.send(Message::Text(resp.to_json().into())).await;
                        if !added { break; }
                    }
                    Err(reason) => {
                        let resp = crate::remote::ServerMessage::AuthResult { success: false, reason: Some(reason) };
                        let _ = socket.send(Message::Text(resp.to_json().into())).await;
                    }
                }
            } else {
                let resp = crate::remote::ServerMessage::Error { message: "Authenticate first".into() };
                let _ = socket.send(Message::Text(resp.to_json().into())).await;
            }
            continue;
        }

        // Authenticated — handle message
        let response = remote.handle_message(&client_msg, &client_ip);
        let _ = socket.send(Message::Text(response.to_json().into())).await;
    }

    remote.remove_client(&client_id);
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

    let remote = Arc::new(crate::remote::RemoteServer::new(port));
    eprintln!("hydra-remote: PIN {} — access at {}", remote.pin(), remote.url());

    let state = ApiState {
        api_token: token,
        request_count: Arc::new(Mutex::new(0)),
        boot_time: chrono::Utc::now(),
        remote,
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
