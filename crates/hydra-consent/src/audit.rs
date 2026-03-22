//! ConsentAuditEntry — every sharing event logged against its consent.

use serde::{Deserialize, Serialize};

/// One logged sharing event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentAuditEntry {
    pub id: String,
    pub grant_id: String,
    pub peer_id: String,
    pub action: String,
    pub description: String,
    pub receipt_id: String,
    pub occurred_at: chrono::DateTime<chrono::Utc>,
}

impl ConsentAuditEntry {
    pub fn new(
        grant_id: &str,
        peer_id: &str,
        action: &str,
        description: &str,
        receipt_id: &str,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            grant_id: grant_id.to_string(),
            peer_id: peer_id.to_string(),
            action: action.to_string(),
            description: description.to_string(),
            receipt_id: receipt_id.to_string(),
            occurred_at: chrono::Utc::now(),
        }
    }
}

/// Append-only audit log.
#[derive(Debug, Default)]
pub struct ConsentAuditLog {
    entries: Vec<ConsentAuditEntry>,
}

impl ConsentAuditLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, entry: ConsentAuditEntry) -> Result<(), crate::errors::ConsentError> {
        if self.entries.len() >= crate::constants::MAX_AUDIT_ENTRIES {
            return Err(crate::errors::ConsentError::StoreFull {
                max: crate::constants::MAX_AUDIT_ENTRIES,
            });
        }
        self.entries.push(entry);
        Ok(())
    }

    pub fn entries_for_peer(&self, peer_id: &str) -> Vec<&ConsentAuditEntry> {
        self.entries
            .iter()
            .filter(|e| e.peer_id == peer_id)
            .collect()
    }

    pub fn count(&self) -> usize {
        self.entries.len()
    }
}
