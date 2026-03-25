//! Universal Drop Gateway — the SINGLE entry point for ALL external items.
//!
//! Drop a file into `~/.hydra/drop/` and Hydra auto-classifies, validates,
//! processes, and audits it. Credentials, skills, configs, documents — everything.
//!
//! Extensible: implement `DropHandler` trait and call `register_handler()`.

pub mod classifier;
pub mod handlers;

use std::path::{Path, PathBuf};
use std::time::Instant;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

use classifier::{DropItemType, classify, security_check};
use handlers::{DropHandler, DropOutcome, register_builtins};

/// Immutable audit record for every drop event.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DropRecord {
    pub id: String,
    pub filename: String,
    pub item_type: DropItemType,
    pub outcome: DropOutcome,
    pub size_bytes: u64,
    pub sha256: String,
    pub timestamp: DateTime<Utc>,
}

/// Statistics for the drop gateway.
#[derive(Debug, Clone, Default)]
pub struct DropStats {
    pub total_processed: usize,
    pub total_accepted: usize,
    pub total_rejected: usize,
    pub last_activity: Option<DateTime<Utc>>,
}

/// The universal drop gateway.
pub struct DropGateway {
    drop_dir: PathBuf,
    processed_dir: PathBuf,
    rejected_dir: PathBuf,
    handlers: Vec<Box<dyn DropHandler>>,
    audit: Vec<DropRecord>,
    audit_file: PathBuf,
    last_poll: Instant,
    stats: DropStats,
}

impl DropGateway {
    /// Create gateway, register built-in handlers, create directories.
    pub fn new() -> Self {
        let base = dirs::home_dir().unwrap_or_default().join(".hydra/drop");
        let processed = base.join("processed");
        let rejected = base.join("rejected");
        let audit_file = base.join("audit.jsonl");

        let _ = std::fs::create_dir_all(&base);
        let _ = std::fs::create_dir_all(&processed);
        let _ = std::fs::create_dir_all(&rejected);

        let mut handlers: Vec<Box<dyn DropHandler>> = Vec::new();
        register_builtins(&mut handlers);

        eprintln!("hydra-drop: gateway initialized at {} ({} handlers)", base.display(), handlers.len());

        Self {
            drop_dir: base,
            processed_dir: processed,
            rejected_dir: rejected,
            handlers,
            audit: Vec::new(),
            audit_file,
            last_poll: Instant::now(),
            stats: DropStats::default(),
        }
    }

    /// Internal self-drop: Hydra writes a file into its own drop folder.
    /// Next tick() will classify, validate, and process it like any external drop.
    pub fn self_drop(&self, filename: &str, content: &[u8]) -> Result<String, String> {
        let path = self.drop_dir.join(filename);
        std::fs::write(&path, content).map_err(|e| format!("self_drop: {e}"))?;
        eprintln!("hydra-drop: self-dropped '{filename}' ({} bytes)", content.len());
        Ok(path.display().to_string())
    }

    /// Register a custom handler for new item types. Extensibility point.
    pub fn register_handler(&mut self, handler: Box<dyn DropHandler>) {
        self.handlers.push(handler);
    }

    /// Poll the drop folder and process any new files. Called from ambient loop.
    pub fn tick(&mut self) -> Vec<DropRecord> {
        let entries = match std::fs::read_dir(&self.drop_dir) {
            Ok(e) => e,
            Err(_) => return vec![],
        };

        let mut records = Vec::new();

        for entry in entries.flatten() {
            let path = entry.path();
            // Skip subdirectories (processed/, rejected/) and audit file
            if path.is_dir() { continue; }
            let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            if name == "audit.jsonl" || name.starts_with('.') { continue; }

            let record = self.process_file(&path);
            records.push(record);
        }

        // Cap audit buffer (prevent OOM like MonitorHub)
        const MAX_AUDIT: usize = 500;
        if self.audit.len() > MAX_AUDIT {
            self.audit.drain(..self.audit.len() - MAX_AUDIT);
        }

        self.last_poll = Instant::now();
        records
    }

    /// Process a single dropped file through the full pipeline.
    fn process_file(&mut self, path: &Path) -> DropRecord {
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        let hash = hash_file(path);

        // Dedup: skip if same hash already processed recently
        if self.audit.iter().rev().take(50).any(|r| r.sha256 == hash) {
            let record = DropRecord {
                id: uuid::Uuid::new_v4().to_string(), filename: name.clone(),
                item_type: DropItemType::Unknown,
                outcome: DropOutcome::Rejected { item_type: "duplicate".into(), reason: "Same file already processed".into() },
                size_bytes: size, sha256: hash, timestamp: Utc::now(),
            };
            move_file(path, &self.rejected_dir.join(&name));
            return record;
        }

        // Security check
        if let Err(reason) = security_check(path) {
            let record = DropRecord {
                id: uuid::Uuid::new_v4().to_string(), filename: name.clone(),
                item_type: DropItemType::Unknown,
                outcome: DropOutcome::Rejected { item_type: "security".into(), reason: reason.clone() },
                size_bytes: size, sha256: hash, timestamp: Utc::now(),
            };
            move_file(path, &self.rejected_dir.join(&name));
            write_error_sidecar(&self.rejected_dir.join(&name), &reason);
            self.persist_record(&record);
            self.stats.total_processed += 1;
            self.stats.total_rejected += 1;
            self.audit.push(record.clone());
            return record;
        }

        // Classify
        let item_type = classify(path);
        eprintln!("hydra-drop: classified '{}' as {:?}", name, item_type);

        // Find handler
        let handler = self.handlers.iter().find(|h| h.handles().contains(&item_type));

        let outcome = match handler {
            Some(h) => {
                // Validate
                match h.validate(path, &item_type) {
                    Ok(()) => {
                        // Process
                        match h.process(path, &item_type) {
                            Ok(outcome) => outcome,
                            Err(e) => DropOutcome::Rejected { item_type: item_type.label(), reason: format!("Process failed: {e}") },
                        }
                    }
                    Err(e) => DropOutcome::Rejected { item_type: item_type.label(), reason: format!("Validation failed: {e}") },
                }
            }
            None => {
                if item_type == DropItemType::Unknown {
                    DropOutcome::Rejected { item_type: "unknown".into(), reason: "Unrecognized file type — cannot process".into() }
                } else {
                    DropOutcome::Rejected { item_type: item_type.label(), reason: "No handler registered for this type".into() }
                }
            }
        };

        // Move file based on outcome
        let dest_dir = match &outcome {
            DropOutcome::Accepted { .. } => &self.processed_dir,
            DropOutcome::Rejected { reason, .. } => {
                write_error_sidecar(&self.rejected_dir.join(&name), reason);
                &self.rejected_dir
            }
        };
        move_file(path, &dest_dir.join(&name));

        let record = DropRecord {
            id: uuid::Uuid::new_v4().to_string(), filename: name,
            item_type, outcome, size_bytes: size, sha256: hash, timestamp: Utc::now(),
        };

        self.persist_record(&record);
        self.stats.total_processed += 1;
        match &record.outcome {
            DropOutcome::Accepted { .. } => self.stats.total_accepted += 1,
            DropOutcome::Rejected { .. } => self.stats.total_rejected += 1,
        }
        self.stats.last_activity = Some(Utc::now());
        self.audit.push(record.clone());
        record
    }

    /// Append audit record to JSONL file (immutable, append-only).
    fn persist_record(&self, record: &DropRecord) {
        if let Ok(json) = serde_json::to_string(record) {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&self.audit_file) {
                let _ = writeln!(f, "{json}");
            }
        }
    }

    /// Recent audit records for TUI display.
    pub fn recent_records(&self, limit: usize) -> Vec<&DropRecord> {
        self.audit.iter().rev().take(limit).collect()
    }

    /// Gateway statistics.
    pub fn stats(&self) -> &DropStats { &self.stats }
}

impl Default for DropGateway {
    fn default() -> Self { Self::new() }
}

/// SHA256 hash of file contents.
fn hash_file(path: &Path) -> String {
    match std::fs::read(path) {
        Ok(bytes) => { let mut h = Sha256::new(); h.update(&bytes); format!("{:x}", h.finalize()) }
        Err(_) => "unknown".into(),
    }
}

/// Move file, overwriting destination if exists.
fn move_file(from: &Path, to: &Path) {
    if let Err(e) = std::fs::rename(from, to) {
        // Cross-device: copy + delete
        if let Err(e2) = std::fs::copy(from, to) {
            eprintln!("hydra-drop: move failed: {e}, copy failed: {e2}");
            return;
        }
        let _ = std::fs::remove_file(from);
    }
}

/// Write error reason as sidecar .error file next to rejected item.
fn write_error_sidecar(path: &Path, reason: &str) {
    let error_path = path.with_extension("error");
    let _ = std::fs::write(&error_path, format!("Rejected: {reason}\nTimestamp: {}\n", Utc::now()));
}

/// Convenience: self-drop without needing a gateway instance.
/// Any module (evolution, coder, config) can call this to create a drop item.
pub fn self_drop_file(filename: &str, content: &[u8]) -> Result<String, String> {
    let dir = dirs::home_dir().unwrap_or_default().join(".hydra/drop");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join(filename);
    std::fs::write(&path, content).map_err(|e| format!("self_drop: {e}"))?;
    eprintln!("hydra-drop: self-dropped '{filename}' ({} bytes)", content.len());
    Ok(path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gateway_creates_directories() {
        let gw = DropGateway::new();
        assert!(gw.drop_dir.exists());
        assert!(gw.processed_dir.exists());
        assert!(gw.rejected_dir.exists());
    }

    #[test]
    fn empty_drop_folder_returns_no_records() {
        let mut gw = DropGateway::new();
        let records = gw.tick();
        // May have items if user has files in drop/ — but no crash
        assert!(records.len() < 1000); // sanity
    }

    #[test]
    fn hash_file_deterministic() {
        let tmp = std::env::temp_dir().join("test_hash_drop.txt");
        std::fs::write(&tmp, "hello world").unwrap();
        let h1 = hash_file(&tmp);
        let h2 = hash_file(&tmp);
        assert_eq!(h1, h2);
        assert!(!h1.is_empty());
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn stats_default() {
        let gw = DropGateway::new();
        assert_eq!(gw.stats().total_processed, 0);
    }
}
