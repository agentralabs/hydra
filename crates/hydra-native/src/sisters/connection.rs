//! MCP connection management — JSON-RPC over stdio transport.

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

#[derive(Debug, Serialize)]
pub(crate) struct JsonRpcRequest {
    pub jsonrpc: &'static str,
    pub id: u64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct JsonRpcResponse {
    #[allow(dead_code)]
    pub jsonrpc: String,
    #[allow(dead_code)]
    pub id: serde_json::Value,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<serde_json::Value>,
}

pub(crate) struct McpProcess {
    pub _child: Child,
    pub stdin: tokio::process::ChildStdin,
    pub stdout: BufReader<tokio::process::ChildStdout>,
    pub request_id: u64,
}

/// A connection to a single sister MCP server
pub struct SisterConnection {
    pub name: String,
    pub(crate) process: Mutex<McpProcess>,
    pub tools: Vec<String>,
}

impl SisterConnection {
    /// Spawn a sister MCP process and initialize it
    pub async fn spawn(
        name: &str,
        command: &str,
        args: &[&str],
    ) -> Result<Self, String> {
        let mut child = Command::new(command)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to spawn {}: {}", name, e))?;

        let stdin = child.stdin.take().ok_or("No stdin")?;
        let stdout = child.stdout.take().ok_or("No stdout")?;

        let proc = McpProcess {
            _child: child,
            stdin,
            stdout: BufReader::new(stdout),
            request_id: 0,
        };

        let conn = Self {
            name: name.to_string(),
            process: Mutex::new(proc),
            tools: Vec::new(),
        };

        // Send initialize
        let init_result = conn
            .send("initialize", Some(serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "hydra-native", "version": "1.0.0" }
            })))
            .await?;

        // Send initialized notification (no response expected)
        {
            let mut proc = conn.process.lock().await;
            let notif = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "notifications/initialized"
            });
            let bytes = serde_json::to_vec(&notif).unwrap();
            let _ = proc.stdin.write_all(&bytes).await;
            let _ = proc.stdin.write_all(b"\n").await;
            let _ = proc.stdin.flush().await;
        }

        // Discover tools
        let tools_result = conn
            .send("tools/list", Some(serde_json::json!({})))
            .await
            .unwrap_or_default();

        let tool_names: Vec<String> = tools_result
            .get("tools")
            .and_then(|t| t.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|t| t.get("name").and_then(|n| n.as_str()).map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        // Log discovery
        let _server_name = init_result
            .get("serverInfo")
            .and_then(|s| s.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("unknown");

        Ok(Self {
            name: name.to_string(),
            process: conn.process,
            tools: tool_names,
        })
    }

    /// Send a JSON-RPC request and read the response
    pub async fn send(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        let mut proc = self.process.lock().await;
        proc.request_id += 1;

        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            id: proc.request_id,
            method: method.to_string(),
            params,
        };

        let bytes = serde_json::to_vec(&request)
            .map_err(|e| format!("Serialize error: {}", e))?;

        proc.stdin
            .write_all(&bytes)
            .await
            .map_err(|e| format!("Write error: {}", e))?;
        proc.stdin
            .write_all(b"\n")
            .await
            .map_err(|e| format!("Write newline error: {}", e))?;
        proc.stdin
            .flush()
            .await
            .map_err(|e| format!("Flush error: {}", e))?;

        // Read response, skipping any notification lines (no "id" field)
        let mut line = String::new();
        loop {
            line.clear();
            let read_result = tokio::time::timeout(
                std::time::Duration::from_secs(15),
                proc.stdout.read_line(&mut line),
            )
            .await
            .map_err(|_| format!("{} timed out", method))?
            .map_err(|e| format!("Read error: {}", e))?;

            if read_result == 0 {
                return Err("EOF from process".into());
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Skip server-initiated notifications (no "id" field)
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
                if val.get("id").is_some() {
                    break; // This is our response
                }
                // Otherwise it's a notification, skip it
                continue;
            }
            break; // Non-JSON line, try to parse anyway
        }

        let response: JsonRpcResponse = serde_json::from_str(line.trim())
            .map_err(|e| format!("Parse error: {} (line: {})", e, line.trim()))?;

        if let Some(err) = response.error {
            return Err(format!("RPC error: {}", err));
        }

        Ok(response.result.unwrap_or(serde_json::Value::Null))
    }

    /// Call a tool on this sister
    pub async fn call_tool(
        &self,
        tool: &str,
        arguments: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        self.send(
            "tools/call",
            Some(serde_json::json!({
                "name": tool,
                "arguments": arguments,
            })),
        )
        .await
    }
}

/// Extract text content from MCP tool response
pub fn extract_text(result: &serde_json::Value) -> String {
    result
        .get("content")
        .and_then(|c| c.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                        item.get("text").and_then(|t| t.as_str()).map(String::from)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}
