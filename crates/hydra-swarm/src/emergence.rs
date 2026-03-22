//! Emergence store — append-only record of emergent swarm behaviors.
//! Entries are permanent and never deleted.

use crate::constants::EMERGENCE_MAX_ENTRIES;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single emergence event observed in the swarm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergenceEntry {
    /// Human-readable description of the emergent behavior.
    pub description: String,
    /// When this emergence was observed.
    pub observed_at: DateTime<Utc>,
    /// How many agents were involved.
    pub agent_count: usize,
    /// The consensus similarity when this emerged.
    pub similarity: f64,
}

impl EmergenceEntry {
    /// Create a new emergence entry.
    pub fn new(description: impl Into<String>, agent_count: usize, similarity: f64) -> Self {
        Self {
            description: description.into(),
            observed_at: Utc::now(),
            agent_count,
            similarity,
        }
    }
}

/// Append-only store for emergence entries.
/// Count only grows — entries are never deleted.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EmergenceStore {
    entries: Vec<EmergenceEntry>,
}

impl EmergenceStore {
    /// Create a new empty emergence store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append an emergence entry. Returns false if at capacity.
    pub fn append(&mut self, entry: EmergenceEntry) -> bool {
        if self.entries.len() >= EMERGENCE_MAX_ENTRIES {
            eprintln!(
                "[swarm] emergence store at capacity ({}/{})",
                self.entries.len(),
                EMERGENCE_MAX_ENTRIES
            );
            return false;
        }
        self.entries.push(entry);
        true
    }

    /// Return the total number of entries (only grows).
    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// Return a reference to all entries.
    pub fn entries(&self) -> &[EmergenceEntry] {
        &self.entries
    }

    /// Return the most recent entry, if any.
    pub fn latest(&self) -> Option<&EmergenceEntry> {
        self.entries.last()
    }
}
