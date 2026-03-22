//! The self-model: Hydra's runtime map of its own capabilities.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::capability::{CapabilityNode, CapabilitySource, CapabilityStatus};
use crate::constants::MAX_CAPABILITIES;
use crate::errors::ReflexiveError;

/// Hydra's self-model — a runtime map of all known capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfModel {
    /// All capabilities indexed by name.
    pub capabilities: HashMap<String, CapabilityNode>,
    /// Total number of capabilities ever registered (monotonically increasing).
    pub total_ever: usize,
    /// When this model was created.
    pub created_at: DateTime<Utc>,
    /// Current tick count for periodic updates.
    pub tick_count: u64,
}

impl SelfModel {
    /// Create a new empty self-model.
    pub fn new() -> Self {
        Self {
            capabilities: HashMap::new(),
            total_ever: 0,
            created_at: Utc::now(),
            tick_count: 0,
        }
    }

    /// Bootstrap the self-model with Layer 1 core crate capabilities.
    pub fn bootstrap_layer1() -> Self {
        let mut model = Self::new();
        let core_crates = [
            "hydra-constitution",
            "hydra-animus",
            "hydra-kernel",
            "hydra-signals",
            "hydra-temporal",
        ];

        for crate_name in &core_crates {
            let cap = CapabilityNode::new(
                *crate_name,
                CapabilitySource::CoreCrate {
                    crate_name: (*crate_name).to_string(),
                },
            );
            // Safe: bootstrap always has room for 5 entries
            let _ = model.insert_capability(cap);
        }
        model
    }

    /// Add a capability to the self-model.
    pub fn add_capability(
        &mut self,
        name: impl Into<String>,
        source: CapabilitySource,
    ) -> Result<(), ReflexiveError> {
        let cap = CapabilityNode::new(name, source);
        self.insert_capability(cap)
    }

    /// Insert an already-constructed capability node.
    fn insert_capability(&mut self, cap: CapabilityNode) -> Result<(), ReflexiveError> {
        if self.capabilities.len() >= MAX_CAPABILITIES {
            return Err(ReflexiveError::SelfModelFull {
                count: self.capabilities.len(),
                max: MAX_CAPABILITIES,
            });
        }
        let name = cap.name.clone();
        self.capabilities.insert(name, cap);
        self.total_ever += 1;
        Ok(())
    }

    /// Look up a capability by name.
    pub fn get(&self, name: &str) -> Option<&CapabilityNode> {
        self.capabilities.get(name)
    }

    /// Look up a capability by name (mutable).
    pub fn get_mut(&mut self, name: &str) -> Option<&mut CapabilityNode> {
        self.capabilities.get_mut(name)
    }

    /// Return all currently active capabilities.
    pub fn active_capabilities(&self) -> Vec<&CapabilityNode> {
        self.capabilities
            .values()
            .filter(|c| matches!(c.status, CapabilityStatus::Active))
            .collect()
    }

    /// Advance the model tick counter.
    pub fn tick(&mut self) {
        self.tick_count += 1;
    }

    /// Return a human-readable summary of the self-model.
    pub fn summary(&self) -> String {
        let active = self.active_capabilities().len();
        let total = self.capabilities.len();
        format!(
            "SelfModel: {active}/{total} active capabilities, \
             {total_ever} total ever, tick {tick}",
            total_ever = self.total_ever,
            tick = self.tick_count,
        )
    }
}

impl Default for SelfModel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_creates_five_core_crates() {
        let model = SelfModel::bootstrap_layer1();
        assert_eq!(model.capabilities.len(), 5);
        assert_eq!(model.total_ever, 5);
        assert!(model.get("hydra-constitution").is_some());
    }

    #[test]
    fn add_capability_increments_total_ever() {
        let mut model = SelfModel::new();
        model
            .add_capability(
                "test-cap",
                CapabilitySource::Skill {
                    skill_id: "s1".into(),
                },
            )
            .expect("should add");
        assert_eq!(model.total_ever, 1);
        assert_eq!(model.capabilities.len(), 1);
    }

    #[test]
    fn tick_increments() {
        let mut model = SelfModel::new();
        model.tick();
        model.tick();
        assert_eq!(model.tick_count, 2);
    }
}
