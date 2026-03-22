//! Signal audit trail — records every signal action for traceability.
//!
//! The audit trail captures gate checks, routing decisions, queue operations,
//! and delivery outcomes. Entries rotate when the maximum is reached.

use crate::constants::AUDIT_TRAIL_MAX_ENTRIES;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The type of action recorded in the audit trail.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuditAction {
    /// Signal passed the gate.
    GatePassed,
    /// Signal was rejected at the gate.
    GateRejected {
        /// The reason for rejection.
        reason: String,
    },
    /// Signal was routed.
    Routed {
        /// The route decision.
        route: String,
    },
    /// Signal was enqueued.
    Enqueued {
        /// Which tier queue.
        tier: String,
    },
    /// Signal was dispatched to a handler.
    Dispatched {
        /// The handler label.
        handler: String,
    },
    /// Signal was dropped.
    Dropped {
        /// The reason for dropping.
        reason: String,
    },
}

/// A single entry in the signal audit trail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// The signal ID this entry pertains to.
    pub signal_id: String,
    /// The action that occurred.
    pub action: AuditAction,
    /// When the action occurred.
    pub timestamp: DateTime<Utc>,
}

impl AuditEntry {
    /// Create a new audit entry.
    pub fn new(signal_id: &str, action: AuditAction) -> Self {
        Self {
            signal_id: signal_id.to_string(),
            action,
            timestamp: Utc::now(),
        }
    }
}

/// The signal audit trail. Automatically rotates at the configured maximum.
pub struct SignalAuditTrail {
    entries: Vec<AuditEntry>,
}

impl SignalAuditTrail {
    /// Create a new empty audit trail.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Record an audit entry. Rotates if the maximum is reached.
    pub fn record(&mut self, entry: AuditEntry) {
        if self.entries.len() >= AUDIT_TRAIL_MAX_ENTRIES {
            // Rotate: remove the oldest half
            let keep_from = self.entries.len() / 2;
            self.entries.drain(..keep_from);
        }
        self.entries.push(entry);
    }

    /// Returns the total number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if the trail is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns all entries for a given signal ID.
    pub fn entries_for(&self, signal_id: &str) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .filter(|e| e.signal_id == signal_id)
            .collect()
    }

    /// Returns the most recent entry, if any.
    pub fn last_entry(&self) -> Option<&AuditEntry> {
        self.entries.last()
    }
}

impl Default for SignalAuditTrail {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_trail_records_entries() {
        let mut trail = SignalAuditTrail::new();
        trail.record(AuditEntry::new("sig-1", AuditAction::GatePassed));
        trail.record(AuditEntry::new(
            "sig-1",
            AuditAction::Routed {
                route: "queue".to_string(),
            },
        ));
        assert_eq!(trail.len(), 2);
        assert_eq!(trail.entries_for("sig-1").len(), 2);
    }

    #[test]
    fn audit_trail_filters_by_signal() {
        let mut trail = SignalAuditTrail::new();
        trail.record(AuditEntry::new("sig-1", AuditAction::GatePassed));
        trail.record(AuditEntry::new("sig-2", AuditAction::GatePassed));
        assert_eq!(trail.entries_for("sig-1").len(), 1);
        assert_eq!(trail.entries_for("sig-2").len(), 1);
    }
}
