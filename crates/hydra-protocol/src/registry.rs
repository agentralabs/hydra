use dashmap::DashMap;
use uuid::Uuid;

use crate::health::{HealthStatus, HealthTracker};
use crate::types::ProtocolEntry;

/// Protocol registry — stores and manages all known protocols
pub struct ProtocolRegistry {
    protocols: DashMap<Uuid, ProtocolEntry>,
    health: HealthTracker,
}

impl ProtocolRegistry {
    pub fn new() -> Self {
        Self {
            protocols: DashMap::new(),
            health: HealthTracker::new(),
        }
    }

    /// Register a new protocol
    pub fn register(&self, protocol: ProtocolEntry) -> Uuid {
        let id = protocol.id;
        self.health.mark_healthy(id);
        self.protocols.insert(id, protocol);
        id
    }

    /// Get a protocol by ID
    pub fn get(&self, id: Uuid) -> Option<ProtocolEntry> {
        self.protocols.get(&id).map(|e| e.value().clone())
    }

    /// Remove a protocol
    pub fn remove(&self, id: Uuid) -> Option<ProtocolEntry> {
        self.protocols.remove(&id).map(|(_, v)| v)
    }

    /// List all available (healthy + usable) protocols
    pub fn list_available(&self) -> Vec<ProtocolEntry> {
        self.protocols
            .iter()
            .filter(|e| e.is_usable() && self.health.is_available(e.id))
            .map(|e| e.value().clone())
            .collect()
    }

    /// List all protocols regardless of health
    pub fn list_all(&self) -> Vec<ProtocolEntry> {
        self.protocols.iter().map(|e| e.value().clone()).collect()
    }

    /// Number of registered protocols
    pub fn count(&self) -> usize {
        self.protocols.len()
    }

    /// Get health tracker
    pub fn health(&self) -> &HealthTracker {
        &self.health
    }

    /// Mark protocol unhealthy
    pub fn mark_unhealthy(&self, id: Uuid) {
        self.health.mark_unhealthy(id);
    }

    /// Mark protocol healthy
    pub fn mark_healthy(&self, id: Uuid) {
        self.health.mark_healthy(id);
    }

    /// Check health of a protocol
    pub fn check_health(&self, id: Uuid) -> HealthStatus {
        self.health.check_health(id)
    }

    /// Find protocols that can handle a capability
    pub fn find_by_capability(&self, capability: &str) -> Vec<ProtocolEntry> {
        self.protocols
            .iter()
            .filter(|e| e.can_handle(capability) && e.is_usable())
            .map(|e| e.value().clone())
            .collect()
    }

    /// Check for circular dependencies among protocols
    pub fn has_circular_dependency(&self, protocol_id: Uuid) -> bool {
        let mut visited = std::collections::HashSet::new();
        self.detect_cycle(protocol_id, &mut visited)
    }

    fn detect_cycle(&self, id: Uuid, visited: &mut std::collections::HashSet<Uuid>) -> bool {
        if !visited.insert(id) {
            return true; // cycle detected
        }
        if let Some(proto) = self.protocols.get(&id) {
            for dep in &proto.depends_on {
                if self.detect_cycle(*dep, visited) {
                    return true;
                }
            }
        }
        visited.remove(&id);
        false
    }

    /// Invalidate auth for a protocol
    pub fn invalidate_auth(&self, id: Uuid) {
        if let Some(mut entry) = self.protocols.get_mut(&id) {
            entry.auth_valid = false;
        }
    }

    /// Set protocol version
    pub fn set_version(&self, id: Uuid, version: impl Into<String>) {
        if let Some(mut entry) = self.protocols.get_mut(&id) {
            entry.version = Some(version.into());
        }
    }
}

impl Default for ProtocolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
