//! HTTP client for communicating with the Hydra server.

use std::io::{BufRead, BufReader};
use std::time::Duration;

/// Default server URL — override with HYDRA_SERVER_URL env var.
const DEFAULT_BASE_URL: &str = "http://127.0.0.1:7777";

pub struct HydraClient {
    base_url: String,
    client: ureq::Agent,
}

/// An SSE event parsed from the event stream.
#[derive(Debug, Clone)]
pub struct SseEvent {
    pub event: String,
    pub data: String,
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

    /// Subscribe to SSE event stream and call handler for each event.
    /// Returns when the stream ends or on error.
    pub fn subscribe_sse<F>(&self, path: &str, mut handler: F) -> Result<(), String>
    where
        F: FnMut(SseEvent) -> bool, // return false to stop
    {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .client
            .get(&url)
            .set("Accept", "text/event-stream")
            .call()
            .map_err(|e| format!("SSE connect failed: {e}"))?;

        let reader = BufReader::new(response.into_reader());
        let mut current_event = String::new();
        let mut current_data = String::new();

        for line in reader.lines() {
            let line = line.map_err(|e| format!("SSE read error: {e}"))?;

            if line.is_empty() {
                // Empty line = event boundary
                if !current_data.is_empty() {
                    let event = SseEvent {
                        event: if current_event.is_empty() {
                            "message".to_string()
                        } else {
                            std::mem::take(&mut current_event)
                        },
                        data: std::mem::take(&mut current_data),
                    };
                    if !handler(event) {
                        break;
                    }
                }
                current_event.clear();
                current_data.clear();
            } else if let Some(val) = line.strip_prefix("event: ") {
                current_event = val.to_string();
            } else if let Some(val) = line.strip_prefix("data: ") {
                if !current_data.is_empty() {
                    current_data.push('\n');
                }
                current_data.push_str(val);
            }
        }

        Ok(())
    }

    pub fn health_check(&self) -> bool {
        self.get("/health").is_ok() || self.get("/api/system/status").is_ok()
    }
}
