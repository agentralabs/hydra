use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptType {
    IntentCompiled,
    ProtocolSelected,
    GateApproved,
    GateDenied,
    ExecutionStarted,
    StepCompleted,
    DeploymentComplete,
    DeploymentFailed,
    RollbackStarted,
    RollbackComplete,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ReceiptId(pub Uuid);

impl ReceiptId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ReceiptId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    pub id: Uuid,
    pub deployment_id: Uuid,
    pub receipt_type: ReceiptType,
    pub timestamp: DateTime<Utc>,
    pub content: serde_json::Value,
    pub content_hash: String,
    pub signature: String,
    pub previous_hash: Option<String>,
    pub sequence: u64,
}

impl Receipt {
    pub fn is_chain_valid(&self, previous: Option<&Receipt>) -> bool {
        match (&self.previous_hash, previous) {
            (None, None) => self.sequence == 0,
            (Some(prev_hash), Some(prev_receipt)) => {
                prev_hash == &prev_receipt.content_hash
                    && self.sequence == prev_receipt.sequence + 1
            }
            _ => false,
        }
    }
}
