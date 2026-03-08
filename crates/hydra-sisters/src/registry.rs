use std::collections::HashMap;
use std::sync::Arc;

use crate::bridge::{HealthStatus, SisterBridge, SisterId};

/// Registry of all sister bridges
pub struct SisterRegistry {
    bridges: HashMap<SisterId, Arc<dyn SisterBridge>>,
}

impl SisterRegistry {
    pub fn new() -> Self {
        Self {
            bridges: HashMap::new(),
        }
    }

    /// Register a sister bridge
    pub fn register(&mut self, bridge: impl SisterBridge + 'static) {
        let id = bridge.sister_id();
        self.bridges.insert(id, Arc::new(bridge));
    }

    /// Get a sister bridge by ID
    pub fn get(&self, id: SisterId) -> Option<Arc<dyn SisterBridge>> {
        self.bridges.get(&id).cloned()
    }

    /// List all registered sister IDs
    pub fn list_registered(&self) -> Vec<SisterId> {
        self.bridges.keys().copied().collect()
    }

    /// List all available (healthy) sisters
    pub async fn list_available(&self) -> Vec<SisterId> {
        let mut available = Vec::new();
        for (id, bridge) in &self.bridges {
            if bridge.health_check().await == HealthStatus::Healthy {
                available.push(*id);
            }
        }
        available
    }

    /// Health check all sisters
    pub async fn health_check_all(&self) -> HashMap<SisterId, HealthStatus> {
        let mut results = HashMap::new();
        for (id, bridge) in &self.bridges {
            results.insert(*id, bridge.health_check().await);
        }
        results
    }

    /// Number of registered sisters
    pub fn count(&self) -> usize {
        self.bridges.len()
    }
}

impl Default for SisterRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridges;

    #[test]
    fn test_registry_new_is_empty() {
        let reg = SisterRegistry::new();
        assert_eq!(reg.count(), 0);
    }

    #[test]
    fn test_registry_default_is_empty() {
        let reg = SisterRegistry::default();
        assert_eq!(reg.count(), 0);
    }

    #[test]
    fn test_registry_register_one() {
        let mut reg = SisterRegistry::new();
        reg.register(bridges::memory_bridge());
        assert_eq!(reg.count(), 1);
    }

    #[test]
    fn test_registry_register_all_14() {
        let mut reg = SisterRegistry::new();
        for b in bridges::all_bridges() {
            reg.register(b);
        }
        assert_eq!(reg.count(), 14);
    }

    #[test]
    fn test_registry_get_existing() {
        let mut reg = SisterRegistry::new();
        reg.register(bridges::memory_bridge());
        let bridge = reg.get(SisterId::Memory);
        assert!(bridge.is_some());
        assert_eq!(bridge.unwrap().name(), "agentic-memory");
    }

    #[test]
    fn test_registry_get_nonexistent() {
        let reg = SisterRegistry::new();
        assert!(reg.get(SisterId::Memory).is_none());
    }

    #[test]
    fn test_registry_list_registered() {
        let mut reg = SisterRegistry::new();
        reg.register(bridges::memory_bridge());
        reg.register(bridges::vision_bridge());
        let list = reg.list_registered();
        assert_eq!(list.len(), 2);
        assert!(list.contains(&SisterId::Memory));
        assert!(list.contains(&SisterId::Vision));
    }

    #[test]
    fn test_registry_list_registered_empty() {
        let reg = SisterRegistry::new();
        assert!(reg.list_registered().is_empty());
    }

    #[tokio::test]
    async fn test_registry_list_available_all_healthy() {
        let mut reg = SisterRegistry::new();
        reg.register(bridges::memory_bridge());
        reg.register(bridges::vision_bridge());
        let available = reg.list_available().await;
        assert_eq!(available.len(), 2);
    }

    #[tokio::test]
    async fn test_registry_list_available_some_unhealthy() {
        let mut reg = SisterRegistry::new();
        let mem_bridge = bridges::memory_bridge();
        mem_bridge.set_available(false);
        reg.register(mem_bridge);
        reg.register(bridges::vision_bridge());
        let available = reg.list_available().await;
        assert_eq!(available.len(), 1);
    }

    #[tokio::test]
    async fn test_registry_health_check_all() {
        let mut reg = SisterRegistry::new();
        reg.register(bridges::memory_bridge());
        let unavailable = bridges::vision_bridge();
        unavailable.set_available(false);
        reg.register(unavailable);
        let health = reg.health_check_all().await;
        assert_eq!(health.len(), 2);
        assert_eq!(health[&SisterId::Memory], HealthStatus::Healthy);
        assert_eq!(health[&SisterId::Vision], HealthStatus::Unavailable);
    }

    #[tokio::test]
    async fn test_registry_health_check_all_empty() {
        let reg = SisterRegistry::new();
        let health = reg.health_check_all().await;
        assert!(health.is_empty());
    }

    #[test]
    fn test_registry_overwrite_same_id() {
        let mut reg = SisterRegistry::new();
        reg.register(bridges::memory_bridge());
        reg.register(bridges::memory_bridge());
        assert_eq!(reg.count(), 1);
    }
}
