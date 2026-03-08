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
