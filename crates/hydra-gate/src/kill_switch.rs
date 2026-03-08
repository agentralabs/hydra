use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use parking_lot::Mutex;

/// Kill switch — instant halt, cannot be overridden by agent
pub struct KillSwitch {
    halted: Arc<AtomicBool>,
    halt_reason: Arc<Mutex<Option<HaltRecord>>>,
}

#[derive(Debug, Clone)]
pub struct HaltRecord {
    pub reason: String,
    pub halted_at: DateTime<Utc>,
    pub halted_by: String,
    pub pending_cleared: usize,
}

impl KillSwitch {
    pub fn new() -> Self {
        Self {
            halted: Arc::new(AtomicBool::new(false)),
            halt_reason: Arc::new(Mutex::new(None)),
        }
    }

    /// Instant halt — stops all execution immediately.
    /// Cannot be overridden by agent. Only user/system can call this.
    pub fn instant_halt(
        &self,
        reason: impl Into<String>,
        halted_by: impl Into<String>,
    ) -> HaltRecord {
        self.halted.store(true, Ordering::SeqCst);
        let record = HaltRecord {
            reason: reason.into(),
            halted_at: Utc::now(),
            halted_by: halted_by.into(),
            pending_cleared: 0,
        };
        *self.halt_reason.lock() = Some(record.clone());
        record
    }

    /// Check if the system is halted
    pub fn is_halted(&self) -> bool {
        self.halted.load(Ordering::SeqCst)
    }

    /// Get the halt reason
    pub fn halt_reason(&self) -> Option<HaltRecord> {
        self.halt_reason.lock().clone()
    }

    /// Resume from halt (only after investigation)
    pub fn resume(&self) {
        self.halted.store(false, Ordering::SeqCst);
        *self.halt_reason.lock() = None;
    }
}

impl Default for KillSwitch {
    fn default() -> Self {
        Self::new()
    }
}
