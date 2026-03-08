use std::sync::Arc;
use std::time::Duration;

use hydra_db::HydraDb;
use hydra_server::{build_router, AppState};

/// Test harness for integration tests — starts a real HTTP server
pub struct TestServer {
    pub addr: std::net::SocketAddr,
    pub state: Arc<AppState>,
    server_handle: tokio::task::JoinHandle<()>,
}

impl TestServer {
    /// Start a test server on a random port
    pub async fn start() -> Self {
        let db = HydraDb::in_memory().unwrap();
        let state = AppState::new(db, false, None);
        let state_arc = Arc::new(state);

        // We need a separate AppState for the router since build_router takes ownership
        let db2 = state_arc.db.clone();
        let router_state = AppState::new(db2, false, None);
        let app = build_router(router_state);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server_handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        // Give server time to start
        tokio::time::sleep(Duration::from_millis(50)).await;

        Self {
            addr,
            state: state_arc,
            server_handle,
        }
    }

    /// Start a test server with shared state access
    pub async fn start_with_state() -> (Self, AppState) {
        let db = HydraDb::in_memory().unwrap();

        // Create the state that the router will use
        let router_state = AppState::new(db.clone(), false, None);
        let event_bus = router_state.event_bus.clone();
        let ledger = router_state.ledger.clone();
        let app = build_router(router_state);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server_handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        tokio::time::sleep(Duration::from_millis(50)).await;

        // Create a parallel state for direct DB/event inspection
        let inspect_state = AppState::new_from_shared(db, event_bus, ledger, false, None);

        let server = Self {
            addr,
            state: Arc::new(AppState::new(HydraDb::in_memory().unwrap(), false, None)),
            server_handle,
        };

        (server, inspect_state)
    }

    pub fn url(&self, path: &str) -> String {
        format!("http://{}{}", self.addr, path)
    }

    /// Send a JSON-RPC request
    pub async fn rpc(&self, method: &str, params: serde_json::Value) -> serde_json::Value {
        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "test-1",
            "method": method,
            "params": params,
        });
        let resp = client
            .post(self.url("/rpc"))
            .json(&body)
            .send()
            .await
            .unwrap();
        resp.json().await.unwrap()
    }

    /// Send hydra.run and return the run_id
    pub async fn run(&self, intent: &str) -> String {
        let resp = self
            .rpc("hydra.run", serde_json::json!({"intent": intent}))
            .await;
        resp["result"]["run_id"].as_str().unwrap().to_string()
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.server_handle.abort();
    }
}
