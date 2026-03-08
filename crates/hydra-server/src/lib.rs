pub mod executor;
pub mod routes;
mod rpc;
mod sse;
mod state;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use futures::stream::Stream;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

pub use rpc::handle_rpc;
pub use sse::sse_stream;
pub use state::AppState;

/// Build the Axum router
pub fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(health_handler))
        .route("/rpc", post(rpc_handler))
        .route("/events", get(sse_handler))
        .merge(routes::api_routes())
        .layer(cors)
        .with_state(Arc::new(state))
}

/// Start the server on the given port
pub async fn start_server(state: AppState, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    // Spawn SSE heartbeat every 30 seconds
    let heartbeat_handle = state.event_bus.spawn_heartbeat(Duration::from_secs(30));

    let app = build_router(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Hydra server listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    heartbeat_handle.abort();
    Ok(())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c().await.ok();
    info!("Shutdown signal received");
}

// ═══════════════════════════════════════════════════════════
// HANDLERS
// ═══════════════════════════════════════════════════════════

async fn health_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let uptime = state.uptime().as_secs();
    Json(serde_json::json!({
        "status": "ok",
        "version": "0.1.0",
        "uptime_seconds": uptime,
    }))
}

async fn rpc_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: String,
) -> (StatusCode, Json<serde_json::Value>) {
    // Auth check
    if state.server_mode {
        if let Err(resp) = check_auth(&state, &headers) {
            return resp;
        }
    }

    let response = handle_rpc(&state, &body).await;
    let json_value = serde_json::to_value(&response).unwrap_or(serde_json::json!({}));
    (StatusCode::OK, Json(json_value))
}

async fn sse_handler(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    Sse::new(sse_stream(state)).keep_alive(KeepAlive::new().interval(Duration::from_secs(30)))
}

fn check_auth(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    // Bypass auth for localhost connections
    // In production: check X-Forwarded-For, but for now we check the token
    if let Some(expected) = &state.auth_token {
        let provided = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.strip_prefix("Bearer ").unwrap_or(s));

        if provided != Some(expected.as_str()) {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": {
                        "code": -32603,
                        "message": "Authentication required. Provide a valid AGENTIC_TOKEN."
                    }
                })),
            ));
        }
    }
    Ok(())
}
