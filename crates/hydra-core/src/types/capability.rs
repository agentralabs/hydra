use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    FileRead,
    FileWrite,
    FileDelete,
    ShellExecute,
    ShellExecuteUnsafe,
    NetworkAccess,
    NetworkAccessExternal,
    SisterAccess(String),
    SisterAccessAll,
    DeployLive,
    DeployBypassApproval,
    ConfigModify,
    UserManage,
    AuditRead,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityToken {
    pub id: Uuid,
    pub holder_id: Uuid,
    pub capabilities: Vec<Capability>,
    pub expires_at: DateTime<Utc>,
    pub signature: String,
}

impl CapabilityToken {
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    pub fn has_capability(&self, cap: &Capability) -> bool {
        if self.is_expired() {
            return false;
        }
        self.capabilities.contains(cap)
    }
}
