use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════
// TYPES
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Pending,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

impl RunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "running" => Some(Self::Running),
            "paused" => Some(Self::Paused),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

impl StepStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "running" => Some(Self::Running),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "skipped" => Some(Self::Skipped),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Denied,
    Expired,
}

impl ApprovalStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Denied => "denied",
            Self::Expired => "expired",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "approved" => Some(Self::Approved),
            "denied" => Some(Self::Denied),
            "expired" => Some(Self::Expired),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptRow {
    pub id: String,
    pub receipt_type: String,
    pub action: String,
    pub actor: String,
    pub tokens_used: i64,
    pub risk_level: Option<String>,
    pub hash: String,
    pub prev_hash: Option<String>,
    pub sequence: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowValidationRow {
    pub action_description: String,
    pub safe: bool,
    pub divergence_count: i32,
    pub critical_divergences: i32,
    pub recommendation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyEventRow {
    pub event_type: String,
    pub command: String,
    pub detail: Option<String>,
    pub severity: String,
    pub kill_switch_engaged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustScoreRow {
    pub domain: String,
    pub score: f64,
    pub total_actions: i64,
    pub successful_actions: i64,
    pub failed_actions: i64,
    pub autonomy_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorSessionRow {
    pub id: String,
    pub task_id: String,
    pub mode: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub event_count: i64,
    pub total_duration_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorEventRow {
    pub timestamp_ms: i64,
    pub event_type: String,
    pub x: f64,
    pub y: f64,
    pub payload: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeliefRow {
    pub id: String,
    pub category: String,
    pub subject: String,
    pub content: String,
    pub confidence: f64,
    pub source: String,
    pub confirmations: i64,
    pub contradictions: i64,
    pub active: bool,
    pub supersedes: Option<String>,
    pub superseded_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpDiscoveredSkillRow {
    pub id: String,
    pub server_name: String,
    pub tool_name: String,
    pub description: Option<String>,
    pub input_schema: Option<String>,
    pub discovered_at: String,
    pub last_used_at: Option<String>,
    pub use_count: i64,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationStateRow {
    pub peer_id: String,
    pub peer_name: Option<String>,
    pub endpoint: String,
    pub trust_level: String,
    pub capabilities: Option<String>,
    pub federation_type: String,
    pub last_sync_version: i64,
    pub last_seen: String,
    pub active_tasks: i64,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairRunRow {
    pub id: String,
    pub spec_file: String,
    pub task: String,
    pub status: String,
    pub iteration: i64,
    pub max_iterations: i64,
    pub checks_total: i64,
    pub checks_passed: i64,
    pub failure_log: Option<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairCheckRow {
    pub run_id: String,
    pub iteration: i64,
    pub check_name: String,
    pub check_command: String,
    pub passed: bool,
    pub output: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRow {
    pub id: String,
    pub intent: String,
    pub status: RunStatus,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
    pub parent_run_id: Option<String>,
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepRow {
    pub id: String,
    pub run_id: String,
    pub sequence: i32,
    pub description: String,
    pub status: StepStatus,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub result: Option<String>,
    pub evidence_refs: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointRow {
    pub id: String,
    pub run_id: String,
    pub step_id: Option<String>,
    pub created_at: String,
    pub state_snapshot: Vec<u8>,
    pub rollback_commands: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRow {
    pub id: String,
    pub run_id: String,
    pub action: String,
    pub target: Option<String>,
    pub risk_score: f64,
    pub created_at: String,
    pub expires_at: String,
    pub status: ApprovalStatus,
}

// ═══════════════════════════════════════════════════════════
// ERROR
// ═══════════════════════════════════════════════════════════

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("Database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Invalid status: {0}")]
    InvalidStatus(String),
}
