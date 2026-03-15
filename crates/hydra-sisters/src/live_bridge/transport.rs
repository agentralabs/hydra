use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::process::Command;

use crate::bridge::*;

use super::{JsonRpcRequest, JsonRpcResponse, LiveMcpBridge, McpTransport, StdioProcess};

impl LiveMcpBridge {
    /// Start the stdio process if not already running
    pub(crate) async fn ensure_process(&self) -> Result<(), SisterError> {
        if let McpTransport::Stdio {
            command,
            args,
            process,
        } = &self.transport
        {
            let mut guard = process.lock().await;
            if let Some(ref mut proc) = *guard {
                // Check if process is still alive
                if proc.child.try_wait().ok().flatten().is_some() {
                    // Process exited, restart
                    *guard = None;
                    drop(guard);
                    return self.start_process(command, args).await;
                }
                return Ok(());
            }
            drop(guard);
            return self.start_process(command, args).await;
        }
        Ok(())
    }

    pub(crate) async fn start_process(&self, command: &str, args: &[String]) -> Result<(), SisterError> {
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

        let proc = StdioProcess {
            child,
            stdin,
            stdout: tokio::io::BufReader::new(stdout),
            request_id: 0,
        };

        // Store the process via Arc<AsyncMutex<Option<>>>
        if let McpTransport::Stdio { process, .. } = &self.transport {
            let mut guard = process.lock().await;
            *guard = Some(proc);
        }
        Ok(())
    }

    /// Send a JSON-RPC request via the appropriate transport
    pub(crate) async fn send_request(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, SisterError> {
        match &self.transport {
            McpTransport::Stdio { process, .. } => {
                let mut guard = process.lock().await;
                let proc = guard.as_mut().ok_or_else(|| SisterError {
                    sister_id: self.sister_id,
                    message: "Stdio process not started".to_string(),
                    retryable: true,
                })?;
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

    pub(crate) fn parse_response(&self, response: JsonRpcResponse) -> Result<serde_json::Value, SisterError> {
        if let Some(error) = response.error {
            return Err(SisterError {
                sister_id: self.sister_id,
                message: format!("[{}] {}", error.code, error.message),
                retryable: error.code == -32603, // Internal error is retryable
            });
        }

        Ok(response.result.unwrap_or(serde_json::Value::Null))
    }

    /// Get timeout for a specific tool (some tools get the complex timeout)
    pub(crate) fn timeout_for_tool(&self, tool: &str) -> Duration {
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
