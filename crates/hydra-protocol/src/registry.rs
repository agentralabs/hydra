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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ProtocolKind;

    fn make_entry(name: &str, kind: ProtocolKind, caps: &[&str]) -> ProtocolEntry {
        ProtocolEntry::new(name, kind).with_capabilities(caps.to_vec())
    }

    #[test]
    fn test_registry_new_empty() {
        let reg = ProtocolRegistry::new();
        assert_eq!(reg.count(), 0);
    }

    #[test]
    fn test_registry_register_and_get() {
        let reg = ProtocolRegistry::new();
        let entry = make_entry("test", ProtocolKind::Sister, &["memory"]);
        let id = entry.id;
        reg.register(entry);
        assert_eq!(reg.count(), 1);
        let got = reg.get(id);
        assert!(got.is_some());
        assert_eq!(got.unwrap().name, "test");
    }

    #[test]
    fn test_registry_get_nonexistent() {
        let reg = ProtocolRegistry::new();
        assert!(reg.get(Uuid::new_v4()).is_none());
    }

    #[test]
    fn test_registry_remove() {
        let reg = ProtocolRegistry::new();
        let entry = make_entry("test", ProtocolKind::Sister, &[]);
        let id = entry.id;
        reg.register(entry);
        let removed = reg.remove(id);
        assert!(removed.is_some());
        assert_eq!(reg.count(), 0);
    }

    #[test]
    fn test_registry_remove_nonexistent() {
        let reg = ProtocolRegistry::new();
        assert!(reg.remove(Uuid::new_v4()).is_none());
    }

    #[test]
    fn test_registry_list_all() {
        let reg = ProtocolRegistry::new();
        reg.register(make_entry("a", ProtocolKind::Sister, &[]));
        reg.register(make_entry("b", ProtocolKind::McpTool, &[]));
        assert_eq!(reg.list_all().len(), 2);
    }

    #[test]
    fn test_registry_list_available() {
        let reg = ProtocolRegistry::new();
        let mut entry = make_entry("unavail", ProtocolKind::Sister, &[]);
        entry.available = false;
        reg.register(entry);
        reg.register(make_entry("avail", ProtocolKind::Sister, &[]));
        let available = reg.list_available();
        assert_eq!(available.len(), 1);
        assert_eq!(available[0].name, "avail");
    }

    #[test]
    fn test_registry_find_by_capability() {
        let reg = ProtocolRegistry::new();
        reg.register(make_entry("mem", ProtocolKind::Sister, &["memory", "query"]));
        reg.register(make_entry("vis", ProtocolKind::Sister, &["vision"]));
        let found = reg.find_by_capability("memory");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "mem");
    }

    #[test]
    fn test_registry_find_by_capability_none() {
        let reg = ProtocolRegistry::new();
        reg.register(make_entry("mem", ProtocolKind::Sister, &["memory"]));
        let found = reg.find_by_capability("codebase");
        assert!(found.is_empty());
    }

    #[test]
    fn test_registry_mark_unhealthy() {
        let reg = ProtocolRegistry::new();
        let entry = make_entry("test", ProtocolKind::Sister, &[]);
        let id = entry.id;
        reg.register(entry);
        reg.mark_unhealthy(id);
        assert_eq!(reg.check_health(id), HealthStatus::Degraded);
    }

    #[test]
    fn test_registry_mark_healthy() {
        let reg = ProtocolRegistry::new();
        let entry = make_entry("test", ProtocolKind::Sister, &[]);
        let id = entry.id;
        reg.register(entry);
        reg.mark_unhealthy(id);
        reg.mark_healthy(id);
        assert_eq!(reg.check_health(id), HealthStatus::Healthy);
    }

    #[test]
    fn test_registry_no_circular_dependency() {
        let reg = ProtocolRegistry::new();
        let entry = make_entry("test", ProtocolKind::Sister, &[]);
        let id = entry.id;
        reg.register(entry);
        assert!(!reg.has_circular_dependency(id));
    }

    #[test]
    fn test_registry_circular_dependency_self() {
        let reg = ProtocolRegistry::new();
        let mut entry = make_entry("self-dep", ProtocolKind::Sister, &[]);
        entry.depends_on.push(entry.id);
        let id = entry.id;
        reg.register(entry);
        assert!(reg.has_circular_dependency(id));
    }

    #[test]
    fn test_registry_circular_dependency_two_way() {
        let reg = ProtocolRegistry::new();
        let mut a = make_entry("a", ProtocolKind::Sister, &[]);
        let mut b = make_entry("b", ProtocolKind::Sister, &[]);
        a.depends_on.push(b.id);
        b.depends_on.push(a.id);
        let a_id = a.id;
        reg.register(a);
        reg.register(b);
        assert!(reg.has_circular_dependency(a_id));
    }

    #[test]
    fn test_registry_invalidate_auth() {
        let reg = ProtocolRegistry::new();
        let entry = make_entry("test", ProtocolKind::RestApi, &[]);
        let id = entry.id;
        reg.register(entry);
        reg.invalidate_auth(id);
        let got = reg.get(id).unwrap();
        assert!(!got.auth_valid);
    }

    #[test]
    fn test_registry_set_version() {
        let reg = ProtocolRegistry::new();
        let entry = make_entry("test", ProtocolKind::Sister, &[]);
        let id = entry.id;
        reg.register(entry);
        reg.set_version(id, "2.0.0");
        let got = reg.get(id).unwrap();
        assert_eq!(got.version, Some("2.0.0".into()));
    }

    #[test]
    fn test_registry_health_accessor() {
        let reg = ProtocolRegistry::new();
        let entry = make_entry("test", ProtocolKind::Sister, &[]);
        let id = entry.id;
        reg.register(entry);
        let health = reg.health();
        assert!(health.is_available(id));
    }
}
