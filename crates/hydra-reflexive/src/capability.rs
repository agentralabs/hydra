//! Capability representation for the self-model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Where a capability originated from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CapabilitySource {
    /// Built into a core Hydra crate.
    CoreCrate {
        /// The crate name providing this capability.
        crate_name: String,
    },
    /// Loaded as a skill at runtime.
    Skill {
        /// The skill identifier.
        skill_id: String,
    },
    /// Provided by an MCP sister.
    Sister {
        /// The sister name.
        sister_name: String,
    },
    /// Recorded in the genome (learned capability).
    GenomeEntry {
        /// The genome entry identifier.
        entry_id: String,
    },
    /// Synthesized by combining other capabilities.
    Synthesized {
        /// IDs of the capabilities this was synthesized from.
        source_ids: Vec<String>,
    },
}

/// Current operational status of a capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CapabilityStatus {
    /// Fully operational.
    Active,
    /// Operational but with reduced performance or reliability.
    Degraded {
        /// Reason for degradation.
        reason: String,
    },
    /// Currently not usable.
    Unavailable {
        /// Reason for unavailability.
        reason: String,
    },
}

/// A single capability known to Hydra's self-model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityNode {
    /// Unique identifier for this capability.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Where this capability came from.
    pub source: CapabilitySource,
    /// Current operational status.
    pub status: CapabilityStatus,
    /// When this capability was first registered.
    pub registered_at: DateTime<Utc>,
    /// When this capability was last used.
    pub last_used: Option<DateTime<Utc>>,
    /// Number of times this capability has been invoked.
    pub invocation_count: u64,
}

impl CapabilityNode {
    /// Create a new capability node with the given name and source.
    pub fn new(name: impl Into<String>, source: CapabilitySource) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            source,
            status: CapabilityStatus::Active,
            registered_at: Utc::now(),
            last_used: None,
            invocation_count: 0,
        }
    }

    /// Returns true if this capability is currently active.
    pub fn is_active(&self) -> bool {
        matches!(self.status, CapabilityStatus::Active)
    }

    /// Record an invocation of this capability.
    pub fn record_invocation(&mut self) {
        self.invocation_count += 1;
        self.last_used = Some(Utc::now());
    }

    /// Mark this capability as degraded.
    pub fn degrade(&mut self, reason: impl Into<String>) {
        self.status = CapabilityStatus::Degraded {
            reason: reason.into(),
        };
    }

    /// Mark this capability as unavailable.
    pub fn mark_unavailable(&mut self, reason: impl Into<String>) {
        self.status = CapabilityStatus::Unavailable {
            reason: reason.into(),
        };
    }

    /// Restore this capability to active status.
    pub fn restore(&mut self) {
        self.status = CapabilityStatus::Active;
    }
}
