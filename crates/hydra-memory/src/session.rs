//! Session tracking — groups exchanges into sessions and episodes.
//! A session is a continuous period of interaction.
//! An episode is a thematically coherent unit (may span sessions).

use crate::{
    constants::{SESSION_BOUNDARY_GAP_MINUTES, SESSION_MAX_EXCHANGES},
    layers::{MemoryLayer, MemoryRecord},
};
use hydra_temporal::timestamp::Timestamp;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A session record — a continuous period of interaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    /// Unique session ID.
    pub id: String,
    /// When this session started.
    pub started_at: Timestamp,
    /// When this session was last active.
    pub last_active_at: Timestamp,
    /// Number of exchanges in this session.
    pub exchange_count: u64,
    /// Whether this session has been closed.
    pub is_closed: bool,
    /// Summary of the session (set on close).
    pub summary: Option<String>,
}

impl SessionRecord {
    /// Create a new session.
    pub fn new() -> Self {
        let now = Timestamp::now();
        Self {
            id: Uuid::new_v4().to_string(),
            started_at: now,
            last_active_at: now,
            exchange_count: 0,
            is_closed: false,
            summary: None,
        }
    }

    /// Record an exchange in this session.
    pub fn record_exchange(&mut self) {
        self.exchange_count += 1;
        self.last_active_at = Timestamp::now();
    }

    /// Close this session.
    pub fn close(&mut self, summary: impl Into<String>) {
        self.is_closed = true;
        self.summary = Some(summary.into());
    }

    /// Check if this session should be closed due to inactivity.
    pub fn is_stale(&self) -> bool {
        if self.is_closed {
            return false;
        }
        let gap_nanos = Timestamp::now().delta_nanos(&self.last_active_at);
        let gap_mins = gap_nanos / (60 * 1_000_000_000);
        gap_mins >= SESSION_BOUNDARY_GAP_MINUTES
            || self.exchange_count >= SESSION_MAX_EXCHANGES as u64
    }

    /// Convert to a MemoryRecord for storage.
    pub fn to_memory_record(&self, causal_root: &str) -> MemoryRecord {
        MemoryRecord::new(
            MemoryLayer::Episodic,
            serde_json::to_value(self).unwrap_or(serde_json::Value::Null),
            &self.id,
            causal_root,
        )
    }
}

impl Default for SessionRecord {
    fn default() -> Self {
        Self::new()
    }
}

/// The session manager — tracks the active session and detects boundaries.
pub struct SessionManager {
    /// The current active session.
    pub current: SessionRecord,
}

impl SessionManager {
    /// Create a new session manager with a fresh session.
    pub fn new() -> Self {
        Self {
            current: SessionRecord::new(),
        }
    }

    /// Get the current session ID.
    pub fn session_id(&self) -> &str {
        &self.current.id
    }

    /// Record an exchange. Returns the new session ID if a boundary was crossed.
    pub fn record_exchange(&mut self) -> Option<String> {
        if self.current.is_stale() {
            // Session boundary — start a new session
            self.current.close("auto-closed: inactivity or size limit");
            self.current = SessionRecord::new();
            return Some(self.current.id.clone());
        }
        self.current.record_exchange();
        None
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_session_starts_open() {
        let s = SessionRecord::new();
        assert!(!s.is_closed);
        assert_eq!(s.exchange_count, 0);
    }

    #[test]
    fn session_records_exchanges() {
        let mut s = SessionRecord::new();
        s.record_exchange();
        s.record_exchange();
        assert_eq!(s.exchange_count, 2);
    }

    #[test]
    fn session_closes_with_summary() {
        let mut s = SessionRecord::new();
        s.close("built AgenticData");
        assert!(s.is_closed);
        assert!(s.summary.is_some());
    }

    #[test]
    fn manager_tracks_current_session() {
        let mut mgr = SessionManager::new();
        let id = mgr.session_id().to_string();
        mgr.record_exchange();
        assert_eq!(mgr.session_id(), id);
    }
}
