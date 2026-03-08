use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Kill signal broadcast to all listeners
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KillSignal {
    /// Stop everything NOW, no cleanup
    InstantHalt,
    /// Complete current phase, then stop
    GracefulStop,
    /// Pause execution, resumable
    Freeze,
    /// Resume from freeze
    Resume,
}

/// Kill switch with 3 severity levels
///
/// Safety: anyone can activate (fail-safe). Only authorized users can deactivate.
#[derive(Clone)]
pub struct KillSwitch {
    active: Arc<AtomicBool>,
    frozen: Arc<AtomicBool>,
    activated_at: Arc<RwLock<Option<DateTime<Utc>>>>,
    reason: Arc<RwLock<Option<String>>>,
    signal: Arc<RwLock<Option<KillSignal>>>,
    sender: broadcast::Sender<KillSignal>,
}

impl KillSwitch {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(16);
        Self {
            active: Arc::new(AtomicBool::new(false)),
            frozen: Arc::new(AtomicBool::new(false)),
            activated_at: Arc::new(RwLock::new(None)),
            reason: Arc::new(RwLock::new(None)),
            signal: Arc::new(RwLock::new(None)),
            sender,
        }
    }

    /// Level 1: Stop everything NOW
    pub fn instant_halt(&self, reason: &str) {
        self.active.store(true, Ordering::SeqCst);
        self.frozen.store(false, Ordering::SeqCst);
        *self.activated_at.write() = Some(Utc::now());
        *self.reason.write() = Some(reason.into());
        *self.signal.write() = Some(KillSignal::InstantHalt);
        let _ = self.sender.send(KillSignal::InstantHalt);
        tracing::error!(target: "security", reason = reason, "KILL SWITCH: instant halt");
    }

    /// Level 2: Complete current phase, then stop
    pub fn graceful_stop(&self, reason: &str) {
        self.active.store(true, Ordering::SeqCst);
        self.frozen.store(false, Ordering::SeqCst);
        *self.activated_at.write() = Some(Utc::now());
        *self.reason.write() = Some(reason.into());
        *self.signal.write() = Some(KillSignal::GracefulStop);
        let _ = self.sender.send(KillSignal::GracefulStop);
        tracing::warn!(target: "security", reason = reason, "KILL SWITCH: graceful stop");
    }

    /// Level 3: Pause execution, resumable
    pub fn freeze(&self, reason: &str) {
        self.frozen.store(true, Ordering::SeqCst);
        *self.activated_at.write() = Some(Utc::now());
        *self.reason.write() = Some(reason.into());
        *self.signal.write() = Some(KillSignal::Freeze);
        let _ = self.sender.send(KillSignal::Freeze);
        tracing::warn!(target: "security", reason = reason, "KILL SWITCH: freeze");
    }

    /// Resume from freeze (requires authorization)
    pub fn resume(&self) {
        self.frozen.store(false, Ordering::SeqCst);
        *self.signal.write() = Some(KillSignal::Resume);
        let _ = self.sender.send(KillSignal::Resume);
        tracing::info!(target: "security", "KILL SWITCH: resumed");
    }

    /// Reset (full deactivation, requires authorization)
    pub fn reset(&self) {
        self.active.store(false, Ordering::SeqCst);
        self.frozen.store(false, Ordering::SeqCst);
        *self.activated_at.write() = None;
        *self.reason.write() = None;
        *self.signal.write() = None;
        tracing::info!(target: "security", "KILL SWITCH: reset");
    }

    /// Is the kill switch active (halted or graceful stop)?
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }

    /// Is the system frozen?
    pub fn is_frozen(&self) -> bool {
        self.frozen.load(Ordering::SeqCst)
    }

    /// Should execution be blocked? (active OR frozen)
    pub fn should_block(&self) -> bool {
        self.is_active() || self.is_frozen()
    }

    /// Get the current signal type
    pub fn current_signal(&self) -> Option<KillSignal> {
        self.signal.read().clone()
    }

    /// Get activation reason
    pub fn reason(&self) -> Option<String> {
        self.reason.read().clone()
    }

    /// Get activation time
    pub fn activated_at(&self) -> Option<DateTime<Utc>> {
        *self.activated_at.read()
    }

    /// Subscribe to kill signals
    pub fn subscribe(&self) -> broadcast::Receiver<KillSignal> {
        self.sender.subscribe()
    }
}

impl Default for KillSwitch {
    fn default() -> Self {
        Self::new()
    }
}
