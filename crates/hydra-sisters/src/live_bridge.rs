use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::io::BufReader;
use tokio::process::Child;
use tokio::sync::Mutex as AsyncMutex;

use crate::bridge::*;
use crate::circuit_breaker::CircuitBreaker;

mod transport;
mod tests;

/// Configuration for a live MCP bridge
#[derive(Debug, Clone)]
pub struct BridgeConfig {
    /// Timeout for individual tool calls
    pub timeout: Duration,
    /// Timeout for complex operations (e.g., codebase_core)
    pub complex_timeout: Duration,
    /// Whether to auto-start the sister process (stdio only)
    pub auto_start: bool,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(5),
            complex_timeout: Duration::from_secs(30),
            auto_start: true,
        }
    }
}

/// Transport layer for communicating with sister MCP servers
pub enum McpTransport {
    /// Spawn sister process, communicate via stdio (JSON-RPC over stdin/stdout)
    Stdio {
        command: String,
        args: Vec<String>,
        process: std::sync::Arc<AsyncMutex<Option<StdioProcess>>>,
    },
    /// Connect to sister HTTP endpoint (JSON-RPC over HTTP POST)
    Http {
        url: String,
        client: reqwest::Client,
    },
}

/// Holds a running stdio child process and its I/O streams
pub struct StdioProcess {
    pub(crate) child: Child,
    pub(crate) stdin: tokio::process::ChildStdin,
    pub(crate) stdout: BufReader<tokio::process::ChildStdout>,
    pub(crate) request_id: u64,
}

/// JSON-RPC 2.0 request
#[derive(Debug, Serialize)]
pub(crate) struct JsonRpcRequest {
    pub(crate) jsonrpc: &'static str,
    pub(crate) id: u64,
    pub(crate) method: String,
    pub(crate) params: serde_json::Value,
}

/// JSON-RPC 2.0 response
#[derive(Debug, Deserialize)]
pub(crate) struct JsonRpcResponse {
    #[allow(dead_code)]
    pub(crate) jsonrpc: String,
    #[allow(dead_code)]
    pub(crate) id: serde_json::Value,
    #[serde(default)]
    pub(crate) result: Option<serde_json::Value>,
    #[serde(default)]
    pub(crate) error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct JsonRpcError {
    pub(crate) code: i64,
    pub(crate) message: String,
    #[allow(dead_code)]
    #[serde(default)]
    pub(crate) data: Option<serde_json::Value>,
}

/// MCP tool definition from tools/list response
#[derive(Debug, Deserialize)]
struct McpToolDef {
    name: String,
    #[allow(dead_code)]
    #[serde(default)]
    description: Option<String>,
}

/// Live MCP bridge that sends real JSON-RPC to sister processes
pub struct LiveMcpBridge {
    pub(crate) sister_id: SisterId,
    pub(crate) name: String,
    version: String,
    pub(crate) capabilities: Vec<String>,
    pub(crate) transport: McpTransport,
    pub(crate) circuit_breaker: CircuitBreaker,
    pub(crate) config: BridgeConfig,
}

impl LiveMcpBridge {
    /// Create a new live bridge with stdio transport
    pub fn stdio(
        sister_id: SisterId,
        command: impl Into<String>,
        args: Vec<String>,
        capabilities: Vec<String>,
        config: BridgeConfig,
    ) -> Self {
        Self {
            sister_id,
            name: sister_id.name().to_string(),
            version: "live".to_string(),
            capabilities,
            transport: McpTransport::Stdio {
                command: command.into(),
                args,
                process: std::sync::Arc::new(AsyncMutex::new(None)),
            },
            circuit_breaker: CircuitBreaker::with_defaults(sister_id),
            config,
        }
    }

    /// Create a new live bridge with HTTP transport
    pub fn http(
        sister_id: SisterId,
        url: impl Into<String>,
        capabilities: Vec<String>,
        config: BridgeConfig,
    ) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(config.timeout)
            .timeout(config.complex_timeout)
            .build()
            .unwrap_or_default();

        Self {
            sister_id,
            name: sister_id.name().to_string(),
            version: "live".to_string(),
            capabilities,
            transport: McpTransport::Http {
                url: url.into(),
                client,
            },
            circuit_breaker: CircuitBreaker::with_defaults(sister_id),
            config,
        }
    }

    /// Discover capabilities from a running sister via MCP tools/list.
    /// Updates the internal capabilities list and returns the discovered tools.
    /// If tools/list fails, the static capabilities list remains as fallback.
    pub async fn discover_capabilities(&mut self) -> Result<Vec<String>, SisterError> {
        let response = self
            .send_request("tools/list", serde_json::json!({}))
            .await?;

        // Parse the tools/list response — MCP returns { tools: [{ name, description, ... }] }
        let tools_value = if let Some(tools) = response.get("tools") {
            tools.clone()
        } else {
            // Some implementations return the array directly
            response
        };

        let tool_defs: Vec<McpToolDef> =
            serde_json::from_value(tools_value).map_err(|e| SisterError {
                sister_id: self.sister_id,
                message: format!("Failed to parse tools/list response: {}", e),
                retryable: false,
            })?;

        self.capabilities = tool_defs.iter().map(|t| t.name.clone()).collect();
        Ok(self.capabilities.clone())
    }
}

#[async_trait]
impl SisterBridge for LiveMcpBridge {
    fn sister_id(&self) -> SisterId {
        self.sister_id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    async fn health_check(&self) -> HealthStatus {
        if !self.circuit_breaker.allow_call() {
            return HealthStatus::Unavailable;
        }

        // Try to ensure the process is running (stdio) or reachable (http)
        match &self.transport {
            McpTransport::Stdio { process, .. } => {
                let guard = process.lock().await;
                if guard.is_none() {
                    drop(guard);
                    if self.ensure_process().await.is_err() {
                        return HealthStatus::Unavailable;
                    }
                }
            }
            McpTransport::Http { url, client } => {
                match tokio::time::timeout(
                    Duration::from_secs(2),
                    client.get(format!("{}/health", url)).send(),
                )
                .await
                {
                    Ok(Ok(resp)) if resp.status().is_success() => {}
                    Ok(Ok(_)) => return HealthStatus::Degraded,
                    _ => return HealthStatus::Unavailable,
                }
            }
        }

        HealthStatus::Healthy
    }

    async fn call(&self, action: SisterAction) -> Result<SisterResult, SisterError> {
        // 1. Check circuit breaker
        if !self.circuit_breaker.allow_call() {
            return Err(SisterError {
                sister_id: self.sister_id,
                message: format!(
                    "Circuit breaker open for {}. Too many recent failures.",
                    self.name
                ),
                retryable: false,
            });
        }

        // 2. Ensure process is started (stdio)
        self.ensure_process().await?;

        // 3. Build MCP tools/call request
        let mcp_params = serde_json::json!({
            "name": action.tool,
            "arguments": action.params,
        });

        // 4. Send with appropriate timeout
        let timeout = self.timeout_for_tool(&action.tool);
        let result = tokio::time::timeout(timeout, self.send_request("tools/call", mcp_params))
            .await
            .map_err(|_| {
                self.circuit_breaker.record_failure();
                SisterError {
                    sister_id: self.sister_id,
                    message: format!(
                        "{} tool '{}' timed out after {:?}",
                        self.name, action.tool, timeout
                    ),
                    retryable: true,
                }
            })?;

        // 5. Handle result
        match result {
            Ok(data) => {
                self.circuit_breaker.record_success();
                Ok(SisterResult {
                    data,
                    tokens_used: 0, // MCP doesn't track tokens
                })
            }
            Err(err) => {
                self.circuit_breaker.record_failure();
                Err(err)
            }
        }
    }

    async fn batch_call(
        &self,
        actions: Vec<SisterAction>,
    ) -> Vec<Result<SisterResult, SisterError>> {
        let mut results = Vec::with_capacity(actions.len());
        for action in actions {
            results.push(self.call(action).await);
        }
        results
    }

    fn capabilities(&self) -> Vec<String> {
        self.capabilities.clone()
    }
}
