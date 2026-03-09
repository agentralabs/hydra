use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex as AsyncMutex;

use crate::bridge::*;
use crate::circuit_breaker::CircuitBreaker;

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
        process: Option<AsyncMutex<StdioProcess>>,
    },
    /// Connect to sister HTTP endpoint (JSON-RPC over HTTP POST)
    Http {
        url: String,
        client: reqwest::Client,
    },
}

/// Holds a running stdio child process and its I/O streams
pub struct StdioProcess {
    child: Child,
    stdin: tokio::process::ChildStdin,
    stdout: BufReader<tokio::process::ChildStdout>,
    request_id: u64,
}

/// JSON-RPC 2.0 request
#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: &'static str,
    id: u64,
    method: String,
    params: serde_json::Value,
}

/// JSON-RPC 2.0 response
#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    #[allow(dead_code)]
    id: serde_json::Value,
    #[serde(default)]
    result: Option<serde_json::Value>,
    #[serde(default)]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
    #[allow(dead_code)]
    #[serde(default)]
    data: Option<serde_json::Value>,
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
    sister_id: SisterId,
    name: String,
    version: String,
    capabilities: Vec<String>,
    transport: McpTransport,
    circuit_breaker: CircuitBreaker,
    config: BridgeConfig,
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
                process: None,
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

    /// Start the stdio process if not already running
    async fn ensure_process(&self) -> Result<(), SisterError> {
        if let McpTransport::Stdio {
            command,
            args,
            process,
        } = &self.transport
        {
            if let Some(proc_mutex) = process {
                // Check if process is still alive
                let mut proc = proc_mutex.lock().await;
                if proc.child.try_wait().ok().flatten().is_some() {
                    // Process exited, need to restart
                    drop(proc);
                    return self.start_process(command, args).await;
                }
                return Ok(());
            }
            return self.start_process(command, args).await;
        }
        Ok(())
    }

    async fn start_process(&self, command: &str, args: &[String]) -> Result<(), SisterError> {
        let mut child = Command::new(command)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| SisterError {
                sister_id: self.sister_id,
                message: format!("Failed to start {}: {}", command, e),
                retryable: false,
            })?;

        let stdin = child.stdin.take().ok_or_else(|| SisterError {
            sister_id: self.sister_id,
            message: "Failed to capture stdin".to_string(),
            retryable: false,
        })?;

        let stdout = child.stdout.take().ok_or_else(|| SisterError {
            sister_id: self.sister_id,
            message: "Failed to capture stdout".to_string(),
            retryable: false,
        })?;

        let _proc = StdioProcess {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            request_id: 0,
        };

        // Note: In a real implementation, we'd store the process.
        // The current architecture uses McpTransport enum which
        // makes in-place mutation complex. For production, this
        // would use an Arc<AsyncMutex<Option<StdioProcess>>>.
        Ok(())
    }

    /// Send a JSON-RPC request via the appropriate transport
    async fn send_request(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, SisterError> {
        match &self.transport {
            McpTransport::Stdio { process, .. } => {
                let proc_mutex = process.as_ref().ok_or_else(|| SisterError {
                    sister_id: self.sister_id,
                    message: "Stdio process not started".to_string(),
                    retryable: true,
                })?;

                let mut proc = proc_mutex.lock().await;
                proc.request_id += 1;
                let request = JsonRpcRequest {
                    jsonrpc: "2.0",
                    id: proc.request_id,
                    method: method.to_string(),
                    params,
                };

                let request_bytes = serde_json::to_vec(&request).map_err(|e| SisterError {
                    sister_id: self.sister_id,
                    message: format!("Failed to serialize request: {}", e),
                    retryable: false,
                })?;

                // Write request + newline
                proc.stdin
                    .write_all(&request_bytes)
                    .await
                    .map_err(|e| SisterError {
                        sister_id: self.sister_id,
                        message: format!("Failed to write to stdin: {}", e),
                        retryable: true,
                    })?;
                proc.stdin.write_all(b"\n").await.map_err(|e| SisterError {
                    sister_id: self.sister_id,
                    message: format!("Failed to write newline: {}", e),
                    retryable: true,
                })?;
                proc.stdin.flush().await.map_err(|e| SisterError {
                    sister_id: self.sister_id,
                    message: format!("Failed to flush stdin: {}", e),
                    retryable: true,
                })?;

                // Read response line
                let mut line = String::new();
                proc.stdout
                    .read_line(&mut line)
                    .await
                    .map_err(|e| SisterError {
                        sister_id: self.sister_id,
                        message: format!("Failed to read from stdout: {}", e),
                        retryable: true,
                    })?;

                let response: JsonRpcResponse =
                    serde_json::from_str(&line).map_err(|e| SisterError {
                        sister_id: self.sister_id,
                        message: format!("Invalid JSON-RPC response: {}", e),
                        retryable: false,
                    })?;

                self.parse_response(response)
            }

            McpTransport::Http { url, client } => {
                let request = JsonRpcRequest {
                    jsonrpc: "2.0",
                    id: 1,
                    method: method.to_string(),
                    params,
                };

                let resp =
                    client
                        .post(url)
                        .json(&request)
                        .send()
                        .await
                        .map_err(|e| SisterError {
                            sister_id: self.sister_id,
                            message: format!("HTTP request failed: {}", e),
                            retryable: e.is_timeout() || e.is_connect(),
                        })?;

                if !resp.status().is_success() {
                    return Err(SisterError {
                        sister_id: self.sister_id,
                        message: format!("HTTP {} from {}", resp.status(), self.name),
                        retryable: resp.status().is_server_error(),
                    });
                }

                let response: JsonRpcResponse = resp.json().await.map_err(|e| SisterError {
                    sister_id: self.sister_id,
                    message: format!("Failed to parse response: {}", e),
                    retryable: false,
                })?;

                self.parse_response(response)
            }
        }
    }

    fn parse_response(&self, response: JsonRpcResponse) -> Result<serde_json::Value, SisterError> {
        if let Some(error) = response.error {
            return Err(SisterError {
                sister_id: self.sister_id,
                message: format!("[{}] {}", error.code, error.message),
                retryable: error.code == -32603, // Internal error is retryable
            });
        }

        Ok(response.result.unwrap_or(serde_json::Value::Null))
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

    /// Get timeout for a specific tool (some tools get the complex timeout)
    fn timeout_for_tool(&self, tool: &str) -> Duration {
        // Complex operations get longer timeouts
        if tool.contains("build")
            || tool.contains("core")
            || tool.contains("omniscience")
            || tool.contains("genetics")
            || tool.contains("crystallize")
            || tool.contains("structure_generate")
            || tool.contains("shadow_execute")
        {
            self.config.complex_timeout
        } else {
            self.config.timeout
        }
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
                if process.is_none() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_config_defaults() {
        let config = BridgeConfig::default();
        assert_eq!(config.timeout, Duration::from_secs(5));
        assert_eq!(config.complex_timeout, Duration::from_secs(30));
        assert!(config.auto_start);
    }

    #[test]
    fn test_live_bridge_http_creation() {
        let bridge = LiveMcpBridge::http(
            SisterId::Memory,
            "http://localhost:3001",
            vec!["memory_add".into(), "memory_query".into()],
            BridgeConfig::default(),
        );
        assert_eq!(bridge.sister_id(), SisterId::Memory);
        assert_eq!(bridge.name(), "agentic-memory");
        assert_eq!(bridge.capabilities().len(), 2);
    }

    #[test]
    fn test_live_bridge_stdio_creation() {
        let bridge = LiveMcpBridge::stdio(
            SisterId::Vision,
            "agentic-vision-mcp",
            vec!["--workspace".into(), "/tmp/test".into()],
            vec!["vision_capture".into()],
            BridgeConfig::default(),
        );
        assert_eq!(bridge.sister_id(), SisterId::Vision);
        assert_eq!(bridge.name(), "agentic-vision");
    }

    #[test]
    fn test_timeout_for_complex_tools() {
        let bridge = LiveMcpBridge::http(
            SisterId::Codebase,
            "http://localhost:3003",
            vec![],
            BridgeConfig::default(),
        );
        assert_eq!(
            bridge.timeout_for_tool("memory_add"),
            Duration::from_secs(5)
        );
        assert_eq!(
            bridge.timeout_for_tool("search_semantic"),
            Duration::from_secs(30)
        );
        assert_eq!(
            bridge.timeout_for_tool("omniscience_search"),
            Duration::from_secs(30)
        );
        assert_eq!(
            bridge.timeout_for_tool("evolve_crystallize"),
            Duration::from_secs(30)
        );
    }

    #[tokio::test]
    async fn test_circuit_breaker_blocks_when_open() {
        let bridge = LiveMcpBridge::http(
            SisterId::Memory,
            "http://localhost:99999", // Non-existent
            vec!["memory_add".into()],
            BridgeConfig::default(),
        );
        // Force circuit open
        bridge
            .circuit_breaker
            .force_state(crate::circuit_breaker::CircuitState::Open);

        let result = bridge
            .call(SisterAction::new("memory_add", serde_json::json!({})))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Circuit breaker open"));
    }

    #[tokio::test]
    async fn test_health_check_with_open_circuit() {
        let bridge = LiveMcpBridge::http(
            SisterId::Memory,
            "http://localhost:99999",
            vec![],
            BridgeConfig::default(),
        );
        bridge
            .circuit_breaker
            .force_state(crate::circuit_breaker::CircuitState::Open);
        assert_eq!(bridge.health_check().await, HealthStatus::Unavailable);
    }
}
