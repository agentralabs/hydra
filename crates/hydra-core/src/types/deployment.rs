use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use uuid::Uuid;

use super::receipt::Receipt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentStatus {
    Pending,
    Running,
    Complete,
    Failed,
    RolledBack,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolUsed {
    pub protocol_id: Uuid,
    pub protocol_name: String,
    pub was_fallback: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    SourceCode,
    Config,
    Test,
    Documentation,
    Binary,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactOperation {
    Created,
    Modified,
    Deleted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: Uuid,
    pub artifact_type: ArtifactType,
    pub path: Option<PathBuf>,
    pub content_hash: String,
    pub operation: ArtifactOperation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStep {
    pub index: usize,
    pub description: String,
    pub status: StepStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub output: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Pending,
    Running,
    Complete,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    FileCreated,
    FileModified,
    FileDeleted,
    DirectoryCreated,
    DirectoryDeleted,
    ConfigChanged,
    PackageInstalled,
    ServiceStarted,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeState {
    pub content_hash: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Change {
    pub id: Uuid,
    pub change_type: ChangeType,
    pub target: String,
    pub before: Option<ChangeState>,
    pub after: ChangeState,
    pub rollbackable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployedSolution {
    pub id: Uuid,
    pub intent_id: Uuid,
    pub status: DeploymentStatus,
    pub protocol_used: ProtocolUsed,
    pub artifacts: Vec<Artifact>,
    pub steps: Vec<ExecutionStep>,
    pub receipts: Vec<Receipt>,
    pub changes: Vec<Change>,
    pub rollback_available: bool,
    #[serde(with = "crate::types::duration_serde")]
    pub duration: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RollbackAction {
    RestoreFile { path: PathBuf, from_backup: PathBuf },
    DeleteFile { path: PathBuf },
    RestoreDirectory { path: PathBuf, from_backup: PathBuf },
    Manual { instructions: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackStep {
    pub change_id: Uuid,
    pub action: RollbackAction,
    pub order: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackPlan {
    pub steps: Vec<RollbackStep>,
    pub non_rollbackable: Vec<Uuid>,
}

pub(crate) mod duration_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
        d.as_millis().serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
        let ms = u128::deserialize(d)?;
        Ok(Duration::from_millis(ms as u64))
    }
}
