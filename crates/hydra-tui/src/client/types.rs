use serde::{Deserialize, Serialize};

/// JSON-RPC 2.0 request
#[derive(Debug, Serialize)]
pub struct RpcRequest {
    pub jsonrpc: &'static str,
    pub method: String,
    pub params: serde_json::Value,
    pub id: u64,
}

impl RpcRequest {
    pub fn new(method: &str, params: serde_json::Value) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static ID: AtomicU64 = AtomicU64::new(1);
        Self {
            jsonrpc: "2.0",
            method: method.to_string(),
            params,
            id: ID.fetch_add(1, Ordering::Relaxed),
        }
    }
}

/// JSON-RPC 2.0 response
#[derive(Debug, Deserialize)]
pub struct RpcResponse {
    pub result: Option<serde_json::Value>,
    pub error: Option<RpcError>,
    pub id: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

/// Server health response
#[derive(Debug, Clone, Deserialize)]
pub struct HealthInfo {
    pub sisters_connected: u32,
    pub sisters_total: u32,
    pub uptime_secs: u64,
    pub profile: Option<String>,
    pub beliefs_loaded: u32,
    pub model: Option<String>,
}

/// Sister status
#[derive(Debug, Clone, Deserialize)]
pub struct SisterStatus {
    pub name: String,
    pub tools: u32,
    pub connected: bool,
    pub last_activity: Option<String>,
}

/// Profile info
#[derive(Debug, Clone, Deserialize)]
pub struct ProfileInfo {
    pub name: String,
    pub identity: Option<String>,
    pub beliefs_count: u32,
    pub skills_count: u32,
    pub category: Option<String>,
    pub active: bool,
}

/// Run result from hydra.run
#[derive(Debug, Clone, Deserialize)]
pub struct RunResult {
    pub run_id: String,
    pub status: String,
    pub output: Option<String>,
}

/// SSE event from the server
#[derive(Debug, Clone)]
pub struct SseEvent {
    pub event: String,
    pub data: String,
}

/// Streaming chunk from SSE
#[derive(Debug, Clone, Deserialize)]
pub struct StreamChunk {
    pub run_id: Option<String>,
    #[serde(rename = "type")]
    pub chunk_type: String, // "text", "tool_start", "tool_end", "thinking", "done", "error"
    pub content: Option<String>,
    pub sister: Option<String>,
    pub tool: Option<String>,
    pub duration_ms: Option<u64>,
}

/// Chat message for display
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub id: String,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub tool_results: Vec<ToolResult>,
    pub beliefs_cited: Vec<BeliefCitation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

/// A tool/sister call result for display
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub sister: String,
    pub action: String,
    pub output: String,
    pub duration_ms: u64,
    pub success: bool,
    pub expanded: bool,
    pub dot_category: crate::theme::DotCategory,
}

/// Belief citation in a response
#[derive(Debug, Clone)]
pub struct BeliefCitation {
    pub text: String,
    pub confidence: f64,
    pub times_tested: u32,
}

/// ROI summary
#[derive(Debug, Clone, Deserialize)]
pub struct RoiSummary {
    pub value_delivered: f64,
    pub llm_cost: f64,
    pub roi_multiple: f64,
}

/// Morning briefing item
#[derive(Debug, Clone)]
pub struct BriefingItem {
    pub priority: BriefingPriority,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BriefingPriority {
    Urgent,    // ▲ red
    Important, // ● yellow
    Info,      // ○ dim
}
