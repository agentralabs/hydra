//! HTTP client for communicating with the Hydra server.

use std::time::Duration;

const DEFAULT_BASE_URL: &str = "http://127.0.0.1:3100";

pub struct HydraClient {
    base_url: String,
    client: ureq::Agent,
}

impl HydraClient {
    pub fn new() -> Self {
        let base_url =
            std::env::var("HYDRA_SERVER_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
        let client = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(30))
            .build();
        Self { base_url, client }
    }

    pub fn get(&self, path: &str) -> Result<serde_json::Value, String> {
        let url = format!("{}{}", self.base_url, path);
        self.client
            .get(&url)
            .call()
            .map_err(|e| format!("Request failed: {e}"))?
            .into_json()
            .map_err(|e| format!("JSON parse failed: {e}"))
    }

    pub fn post(&self, path: &str, body: &serde_json::Value) -> Result<serde_json::Value, String> {
        let url = format!("{}{}", self.base_url, path);
        self.client
            .post(&url)
            .send_json(body)
            .map_err(|e| format!("Request failed: {e}"))?
            .into_json()
            .map_err(|e| format!("JSON parse failed: {e}"))
    }

    pub fn put(&self, path: &str, body: &serde_json::Value) -> Result<serde_json::Value, String> {
        let url = format!("{}{}", self.base_url, path);
        self.client
            .put(&url)
            .send_json(body)
            .map_err(|e| format!("Request failed: {e}"))?
            .into_json()
            .map_err(|e| format!("JSON parse failed: {e}"))
    }

    #[allow(dead_code)]
    pub fn delete(&self, path: &str) -> Result<serde_json::Value, String> {
        let url = format!("{}{}", self.base_url, path);
        self.client
            .delete(&url)
            .call()
            .map_err(|e| format!("Request failed: {e}"))?
            .into_json()
            .map_err(|e| format!("JSON parse failed: {e}"))
    }

    pub fn health_check(&self) -> bool {
        self.get("/health").is_ok()
    }
}
