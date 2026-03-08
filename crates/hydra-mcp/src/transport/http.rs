//! HTTP+SSE transport for MCP servers.

use serde::{Deserialize, Serialize};

use super::{TransportConfig, TransportStatus};

/// HTTP transport configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConfig {
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub timeout_ms: u64,
}

impl HttpConfig {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.into(),
            headers: Vec::new(),
            timeout_ms: 30000,
        }
    }

    pub fn from_transport(config: &TransportConfig) -> Option<Self> {
        match config {
            TransportConfig::Http { url, headers } => Some(Self {
                url: url.clone(),
                headers: headers.clone(),
                timeout_ms: 30000,
            }),
            _ => None,
        }
    }
}

/// HTTP transport handler (simulated for unit testing)
pub struct HttpTransport {
    config: HttpConfig,
    status: TransportStatus,
    request_log: Vec<(String, String)>, // (method, body)
}

impl HttpTransport {
    pub fn new(config: HttpConfig) -> Self {
        Self {
            config,
            status: TransportStatus::Disconnected,
            request_log: Vec::new(),
        }
    }

    pub fn connect(&mut self) -> Result<(), String> {
        // Validate URL format
        if !self.config.url.starts_with("http://") && !self.config.url.starts_with("https://") {
            return Err("invalid URL: must start with http:// or https://".into());
        }
        self.status = TransportStatus::Connected;
        Ok(())
    }

    pub fn send(&mut self, method: &str, body: &str) -> Result<String, String> {
        if self.status != TransportStatus::Connected {
            return Err("not connected".into());
        }
        self.request_log.push((method.into(), body.into()));

        // Return simulated response
        Ok(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": { "status": "ok" }
        })
        .to_string())
    }

    pub fn disconnect(&mut self) {
        self.status = TransportStatus::Disconnected;
    }

    pub fn status(&self) -> TransportStatus {
        self.status
    }

    pub fn url(&self) -> &str {
        &self.config.url
    }

    pub fn request_count(&self) -> usize {
        self.request_log.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_connect() {
        let config = HttpConfig::new("http://localhost:3000");
        let mut transport = HttpTransport::new(config);
        transport.connect().unwrap();
        assert_eq!(transport.status(), TransportStatus::Connected);
    }

    #[test]
    fn test_http_invalid_url() {
        let config = HttpConfig::new("ftp://invalid");
        let mut transport = HttpTransport::new(config);
        assert!(transport.connect().is_err());
    }
}
