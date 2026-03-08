use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::capability::Capability;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IdentityType {
    Human,
    Agent,
    System,
    Service,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum TrustLevel {
    Untrusted = 0,
    Basic = 1,
    Verified = 2,
    Trusted = 3,
    Full = 4,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HydraIdentity {
    pub id: Uuid,
    pub identity_type: IdentityType,
    pub name: String,
    pub capabilities: Vec<Capability>,
    pub trust_level: TrustLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    Active,
    Idle,
    Suspended,
    Terminated,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionContext {
    pub working_directory: Option<String>,
    pub environment: std::collections::HashMap<String, String>,
    pub preferences: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HydraSession {
    pub id: Uuid,
    pub identity_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub state: SessionState,
    pub context: SessionContext,
}
