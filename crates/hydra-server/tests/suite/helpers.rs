use hydra_db::HydraDb;
use hydra_server::AppState;

pub fn test_state() -> AppState {
    let db = HydraDb::in_memory().unwrap();
    AppState::new(db, false, None)
}

pub fn test_state_with_auth(token: &str) -> AppState {
    let db = HydraDb::in_memory().unwrap();
    AppState::new(db, true, Some(token.into()))
}

// Helper to make JSON-RPC request body
pub fn rpc_body(method: &str, params: serde_json::Value) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": "test-1",
        "method": method,
        "params": params,
    })
    .to_string()
}
