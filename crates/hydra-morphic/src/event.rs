//! Morphic events — the building blocks of identity history.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The kind of morphic event that occurred.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MorphicEventKind {
    /// A new capability was added.
    CapabilityAdded {
        /// Name of the added capability.
        name: String,
    },
    /// A capability's status changed.
    CapabilityStatusChanged {
        /// Name of the affected capability.
        name: String,
        /// New status description.
        new_status: String,
    },
    /// A self-modification was applied.
    SelfModificationApplied {
        /// Description of the modification.
        description: String,
    },
    /// A self-modification was rolled back.
    SelfModificationRolledBack {
        /// Description of what was rolled back.
        description: String,
    },
    /// A skill was loaded.
    SkillLoaded {
        /// The skill identifier.
        skill_id: String,
    },
    /// A sister came online.
    SisterConnected {
        /// The sister name.
        sister_name: String,
    },
    /// A sister went offline.
    SisterDisconnected {
        /// The sister name.
        sister_name: String,
    },
    /// The system was restarted.
    SystemRestart {
        /// Reason for restart.
        reason: String,
    },
    /// A genome entry was recorded.
    GenomeRecorded {
        /// The genome entry identifier.
        entry_id: String,
    },
}

/// A single event in the morphic identity history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MorphicEvent {
    /// Unique event identifier.
    pub id: String,
    /// What kind of event.
    pub kind: MorphicEventKind,
    /// When this event occurred.
    pub timestamp: DateTime<Utc>,
    /// Hash chain link — hash of the previous event.
    pub prior_hash: String,
}

impl MorphicEvent {
    /// Create a new morphic event with the given kind and prior hash.
    pub fn new(kind: MorphicEventKind, prior_hash: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            kind,
            timestamp: Utc::now(),
            prior_hash: prior_hash.into(),
        }
    }
}
