//! Guardrail audit log — append-only decision trail.
//! Hydra can read but never delete or modify past entries.
//! Same JSONL pattern as drop/mod.rs audit.jsonl.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::PathBuf;

/// Type of guardrail event recorded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEventType {
    KillSignal,
    DeadManSwitch,
    EvolutionProposed,
    EvolutionApproved,
    EvolutionRejected,
    BoundaryViolation,
    StateChange,
    ConfigReloaded,
    OwnerInteraction,
    RemoteKill,
}

/// A single audit entry — immutable once written.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: DateTime<Utc>,
    pub event_type: AuditEventType,
    pub details: String,
    pub actor: String, // "owner", "hydra", "system", "remote"
}

/// Append-only audit log with in-memory buffer for TUI display.
pub struct AuditLog {
    path: PathBuf,
    buffer: Vec<AuditEntry>,
    max_buffer: usize,
}

impl AuditLog {
    pub fn new() -> Self {
        let path = dirs::home_dir().unwrap_or_default()
            .join(".hydra/guardrails/audit.jsonl");
        Self { path, buffer: Vec::new(), max_buffer: 200 }
    }

    /// Record an audit entry. Append-only: never overwrites.
    pub fn record(&mut self, event_type: AuditEventType, details: &str, actor: &str) {
        let entry = AuditEntry {
            timestamp: Utc::now(),
            event_type,
            details: details.into(),
            actor: actor.into(),
        };
        // Persist to disk (append-only)
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match std::fs::OpenOptions::new().create(true).append(true).open(&self.path) {
            Ok(mut file) => {
                if let Ok(json) = serde_json::to_string(&entry) {
                    if let Err(e) = writeln!(file, "{json}") {
                        eprintln!("hydra-guardrail: audit write failed: {e}");
                    }
                }
            }
            Err(e) => eprintln!("hydra-guardrail: audit open failed: {e}"),
        }
        // Buffer for TUI display
        self.buffer.push(entry);
        if self.buffer.len() > self.max_buffer {
            self.buffer.remove(0);
        }
    }

    /// Get last N entries from buffer.
    pub fn recent(&self, n: usize) -> &[AuditEntry] {
        let start = self.buffer.len().saturating_sub(n);
        &self.buffer[start..]
    }

    /// Total entries in buffer.
    pub fn len(&self) -> usize { self.buffer.len() }

    /// Whether buffer is empty.
    pub fn is_empty(&self) -> bool { self.buffer.is_empty() }
}

/// Quick one-shot audit record (no engine needed).
pub fn record_quick(event_type: AuditEventType, details: &str) {
    let mut log = AuditLog::new();
    log.record(event_type, details, "system");
}

impl Default for AuditLog {
    fn default() -> Self { Self::new() }
}
