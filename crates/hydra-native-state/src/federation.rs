//! Federation layer — peer discovery, task delegation, skill sharing.
//!
//! Wraps hydra-federation types into a unified manager that boots
//! at startup and enables distributed Hydra operations.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use hydra_federation::registry::PeerRegistry;
use hydra_federation::discovery::{DiscoveryMethod, PeerDiscovery};
use hydra_federation::delegation::{LoadBalanceStrategy, TaskDelegation};
use hydra_federation::sharing::SkillSharing;
use hydra_federation::sync::{ConflictStrategy, SyncProtocol};

/// Federation manager — coordinates all P2P operations.
///
/// Initialized at app boot (disabled by default). The user can enable
/// federation through settings, at which point peer discovery, task
/// delegation, and skill sharing become active.
pub struct FederationManager {
    pub registry: Arc<PeerRegistry>,
    pub discovery: Arc<PeerDiscovery>,
    pub delegation: Arc<TaskDelegation>,
    pub skills: Arc<SkillSharing>,
    pub sync: Arc<SyncProtocol>,
    enabled: AtomicBool,
}

impl FederationManager {
    /// Boot federation with default configuration
    pub fn new() -> Self {
        let registry = Arc::new(PeerRegistry::new());
        let discovery = Arc::new(PeerDiscovery::new(DiscoveryMethod::Manual(Vec::new())));
        let delegation = Arc::new(TaskDelegation::new(LoadBalanceStrategy::LeastLoaded));
        let skills = Arc::new(SkillSharing::new());
        let sync = Arc::new(SyncProtocol::new(ConflictStrategy::LastWriteWins));

        Self {
            registry,
            discovery,
            delegation,
            skills,
            sync,
            enabled: AtomicBool::new(false),
        }
    }

    /// Enable federation (callable through Arc)
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::Relaxed);
    }

    /// Disable federation (callable through Arc)
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::Relaxed);
    }

    /// Check if federation is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Add a manual peer endpoint
    pub fn add_peer(&self, endpoint: &str) {
        self.discovery.add_manual(endpoint);
    }

    /// Run peer discovery and return count of discovered peers
    pub fn discover_peers(&self) -> usize {
        self.discovery.discover();
        self.discovery.count()
    }

    /// Get registered peer count
    pub fn peer_count(&self) -> usize {
        self.registry.count()
    }

    /// Get status summary
    pub fn status(&self) -> String {
        if !self.is_enabled() {
            return "Federation: disabled".to_string();
        }
        format!(
            "Federation: enabled, {} peers registered, {} available",
            self.registry.count(),
            self.registry.available_peers().len()
        )
    }
}

impl Default for FederationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_federation_new_disabled() {
        let mgr = FederationManager::new();
        assert!(!mgr.is_enabled());
        assert_eq!(mgr.peer_count(), 0);
    }

    #[test]
    fn test_federation_enable_disable() {
        let mut mgr = FederationManager::new();
        assert!(!mgr.is_enabled());

        mgr.enable();
        assert!(mgr.is_enabled());

        mgr.disable();
        assert!(!mgr.is_enabled());
    }

    #[test]
    fn test_federation_status_disabled() {
        let mgr = FederationManager::new();
        assert_eq!(mgr.status(), "Federation: disabled");
    }

    #[test]
    fn test_federation_status_enabled() {
        let mut mgr = FederationManager::new();
        mgr.enable();
        let status = mgr.status();
        assert!(status.starts_with("Federation: enabled"));
        assert!(status.contains("0 peers registered"));
    }

    #[test]
    fn test_add_peer() {
        let mgr = FederationManager::new();
        mgr.add_peer("192.168.1.10:9000");
        // add_peer adds to discovery's discovered list
        assert_eq!(mgr.discovery.count(), 1);
    }

    #[test]
    fn test_discover_peers_empty() {
        let mgr = FederationManager::new();
        // Manual discovery with empty list returns 0
        let count = mgr.discover_peers();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_federation_default() {
        let mgr = FederationManager::default();
        assert!(!mgr.is_enabled());
        assert_eq!(mgr.peer_count(), 0);
    }
}
