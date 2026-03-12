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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_kill_switch_is_inactive() {
        let ks = KillSwitch::new();
        assert!(!ks.is_active());
        assert!(!ks.is_frozen());
        assert!(!ks.should_block());
        assert!(ks.reason().is_none());
        assert!(ks.activated_at().is_none());
        assert!(ks.current_signal().is_none());
    }

    #[test]
    fn test_default_is_inactive() {
        let ks = KillSwitch::default();
        assert!(!ks.is_active());
        assert!(!ks.is_frozen());
    }

    #[test]
    fn test_instant_halt_activates() {
        let ks = KillSwitch::new();
        ks.instant_halt("test reason");
        assert!(ks.is_active());
        assert!(!ks.is_frozen());
        assert!(ks.should_block());
        assert_eq!(ks.reason(), Some("test reason".to_string()));
        assert!(ks.activated_at().is_some());
        assert_eq!(ks.current_signal(), Some(KillSignal::InstantHalt));
    }

    #[test]
    fn test_graceful_stop_activates() {
        let ks = KillSwitch::new();
        ks.graceful_stop("graceful reason");
        assert!(ks.is_active());
        assert!(!ks.is_frozen());
        assert!(ks.should_block());
        assert_eq!(ks.reason(), Some("graceful reason".to_string()));
        assert_eq!(ks.current_signal(), Some(KillSignal::GracefulStop));
    }

    #[test]
    fn test_freeze_sets_frozen() {
        let ks = KillSwitch::new();
        ks.freeze("freeze reason");
        assert!(!ks.is_active());
        assert!(ks.is_frozen());
        assert!(ks.should_block());
        assert_eq!(ks.current_signal(), Some(KillSignal::Freeze));
    }

    #[test]
    fn test_resume_clears_frozen() {
        let ks = KillSwitch::new();
        ks.freeze("freeze");
        assert!(ks.is_frozen());
        ks.resume();
        assert!(!ks.is_frozen());
        assert!(!ks.should_block());
        assert_eq!(ks.current_signal(), Some(KillSignal::Resume));
    }

    #[test]
    fn test_reset_clears_everything() {
        let ks = KillSwitch::new();
        ks.instant_halt("halt");
        assert!(ks.is_active());
        ks.reset();
        assert!(!ks.is_active());
        assert!(!ks.is_frozen());
        assert!(!ks.should_block());
        assert!(ks.reason().is_none());
        assert!(ks.activated_at().is_none());
        assert!(ks.current_signal().is_none());
    }

    #[test]
    fn test_should_block_active_or_frozen() {
        let ks = KillSwitch::new();
        assert!(!ks.should_block());
        ks.freeze("f");
        assert!(ks.should_block());
        ks.resume();
        assert!(!ks.should_block());
        ks.instant_halt("h");
        assert!(ks.should_block());
    }

    #[test]
    fn test_subscribe_receives_signals() {
        let ks = KillSwitch::new();
        let mut rx = ks.subscribe();
        ks.instant_halt("test");
        let signal = rx.try_recv().unwrap();
        assert_eq!(signal, KillSignal::InstantHalt);
    }

    #[test]
    fn test_clone_shares_state() {
        let ks = KillSwitch::new();
        let ks2 = ks.clone();
        ks.instant_halt("shared");
        assert!(ks2.is_active());
        assert_eq!(ks2.reason(), Some("shared".to_string()));
    }

    #[test]
    fn test_kill_signal_serde() {
        let signal = KillSignal::GracefulStop;
        let json = serde_json::to_string(&signal).unwrap();
        let restored: KillSignal = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, KillSignal::GracefulStop);
    }

    #[test]
    fn test_instant_halt_clears_frozen() {
        let ks = KillSwitch::new();
        ks.freeze("frozen");
        assert!(ks.is_frozen());
        ks.instant_halt("halt overrides freeze");
        assert!(ks.is_active());
        assert!(!ks.is_frozen());
    }
}
