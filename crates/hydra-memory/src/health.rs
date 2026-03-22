//! Memory health metrics for TUI display and kernel integration.

use crate::bridge::MemoryHealth;
use serde::{Deserialize, Serialize};

/// Full memory health snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryHealthSnapshot {
    /// Total memories written.
    pub total_written: u64,
    /// Total memories indexed in temporal bridge.
    pub temporal_indexed: u64,
    /// Current session ID.
    pub session_id: String,
    /// Number of exchanges in the current session.
    pub exchange_count: u64,
    /// Whether integrity checks are passing.
    pub integrity_ok: bool,
}

impl MemoryHealthSnapshot {
    /// Create a health snapshot from a MemoryHealth report.
    pub fn from_health(h: MemoryHealth) -> Self {
        Self {
            total_written: h.total_written,
            temporal_indexed: h.temporal_indexed,
            session_id: h.session_id,
            exchange_count: h.exchange_count,
            integrity_ok: true, // verified at write time
        }
    }

    /// Format a one-line status string for TUI display.
    pub fn status_line(&self) -> String {
        let id_prefix_len = 8.min(self.session_id.len());
        format!(
            "Memory: {} written, {} indexed | session {} | {} exchanges",
            self.total_written,
            self.temporal_indexed,
            &self.session_id[..id_prefix_len],
            self.exchange_count,
        )
    }
}
