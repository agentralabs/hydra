//! Cartography atlas — the full digital map.

use crate::constants::{
    KNOWLEDGE_TRANSFER_CONFIDENCE, MAX_SYSTEM_PROFILES, TOPOLOGY_SIMILARITY_THRESHOLD,
};
use crate::errors::CartographyError;
use crate::profile::SystemProfile;
use crate::system_class::SystemClass;
use crate::topology::TopologyMap;
use std::collections::BTreeMap;

/// The full digital atlas tracking all known systems.
///
/// Append-only: systems are never removed.
#[derive(Debug)]
pub struct CartographyAtlas {
    /// All system profiles keyed by name.
    profiles: BTreeMap<String, SystemProfile>,
    /// The topology map of system relationships.
    topology: TopologyMap,
    /// Total profiles ever added (monotonically increasing).
    total_ever: u64,
}

impl CartographyAtlas {
    /// Create an empty atlas.
    pub fn new() -> Self {
        Self {
            profiles: BTreeMap::new(),
            topology: TopologyMap::new(),
            total_ever: 0,
        }
    }

    /// Add a system profile to the atlas.
    ///
    /// Automatically discovers topology neighbors from existing profiles.
    /// Returns an error if a profile with the same name already exists
    /// or if the atlas is at capacity.
    pub fn add(&mut self, profile: SystemProfile) -> Result<(), CartographyError> {
        if self.profiles.len() >= MAX_SYSTEM_PROFILES {
            return Err(CartographyError::AtlasFull {
                max: MAX_SYSTEM_PROFILES,
            });
        }
        if self.profiles.contains_key(&profile.name) {
            return Err(CartographyError::ProfileAlreadyExists {
                name: profile.name.clone(),
            });
        }

        // Auto-discover topology neighbors.
        let neighbors: Vec<(String, f64)> = self
            .profiles
            .values()
            .map(|existing| (existing.name.clone(), existing.similarity(&profile)))
            .filter(|(_, sim)| *sim >= TOPOLOGY_SIMILARITY_THRESHOLD)
            .collect();

        for (neighbor_name, sim) in neighbors {
            self.topology.add_neighbor(
                &profile.name,
                &neighbor_name,
                sim,
                TOPOLOGY_SIMILARITY_THRESHOLD,
            );
        }

        self.profiles.insert(profile.name.clone(), profile);
        self.total_ever += 1;
        Ok(())
    }

    /// Transfer knowledge from similar systems to a target system.
    ///
    /// Copies approaches from topology neighbors with confidence scaling.
    /// Returns the number of approaches transferred.
    pub fn transfer_knowledge(&mut self, target_name: &str) -> Result<usize, CartographyError> {
        let neighbors: Vec<(String, f64)> = self
            .topology
            .neighbors(target_name)
            .iter()
            .map(|(name, sim)| (name.clone(), *sim))
            .collect();

        if neighbors.is_empty() {
            return Ok(0);
        }

        // Collect approaches from neighbors.
        let mut transferred_approaches: Vec<String> = Vec::new();
        for (neighbor_name, sim) in &neighbors {
            if let Some(neighbor) = self.profiles.get(neighbor_name) {
                if *sim >= KNOWLEDGE_TRANSFER_CONFIDENCE {
                    for approach in &neighbor.approaches {
                        transferred_approaches.push(approach.clone());
                    }
                }
            }
        }

        // Apply to target.
        let target = self.profiles.get_mut(target_name).ok_or_else(|| {
            CartographyError::ProfileNotFound {
                name: target_name.to_string(),
            }
        })?;

        let count = transferred_approaches.len();
        for approach in transferred_approaches {
            if !target.approaches.contains(&approach) {
                target.approaches.push(approach);
            }
        }

        Ok(count)
    }

    /// Get profiles by system class.
    pub fn by_class(&self, class: &SystemClass) -> Vec<&SystemProfile> {
        self.profiles
            .values()
            .filter(|p| p.class == *class)
            .collect()
    }

    /// Get a mutable reference to a profile by name.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut SystemProfile> {
        self.profiles.get_mut(name)
    }

    /// Get a reference to a profile by name.
    pub fn get(&self, name: &str) -> Option<&SystemProfile> {
        self.profiles.get(name)
    }

    /// Total profiles ever added (monotonically increasing).
    pub fn total_ever(&self) -> u64 {
        self.total_ever
    }

    /// Current number of profiles in the atlas.
    pub fn len(&self) -> usize {
        self.profiles.len()
    }

    /// Returns true if the atlas is empty.
    pub fn is_empty(&self) -> bool {
        self.profiles.is_empty()
    }

    /// Get the topology map.
    pub fn topology(&self) -> &TopologyMap {
        &self.topology
    }
}

impl Default for CartographyAtlas {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_retrieve() {
        let mut atlas = CartographyAtlas::new();
        atlas
            .add(SystemProfile::new("api-a", SystemClass::RestApi))
            .unwrap();
        assert!(atlas.get("api-a").is_some());
        assert_eq!(atlas.total_ever(), 1);
    }

    #[test]
    fn duplicate_rejected() {
        let mut atlas = CartographyAtlas::new();
        atlas
            .add(SystemProfile::new("api-a", SystemClass::RestApi))
            .unwrap();
        let result = atlas.add(SystemProfile::new("api-a", SystemClass::RestApi));
        assert!(result.is_err());
    }

    #[test]
    fn auto_topology_discovery() {
        let mut atlas = CartographyAtlas::new();
        let mut p1 = SystemProfile::new("api-a", SystemClass::RestApi);
        p1.add_hint("json");
        atlas.add(p1).unwrap();

        let mut p2 = SystemProfile::new("api-b", SystemClass::RestApi);
        p2.add_hint("json");
        atlas.add(p2).unwrap();

        let neighbors = atlas.topology().neighbors("api-a");
        assert!(!neighbors.is_empty());
    }

    #[test]
    fn by_class_filters() {
        let mut atlas = CartographyAtlas::new();
        atlas
            .add(SystemProfile::new("api-a", SystemClass::RestApi))
            .unwrap();
        atlas
            .add(SystemProfile::new("db-a", SystemClass::RelationalDatabase))
            .unwrap();

        let apis = atlas.by_class(&SystemClass::RestApi);
        assert_eq!(apis.len(), 1);
        assert_eq!(apis[0].name, "api-a");
    }

    #[test]
    fn transfer_knowledge_works() {
        let mut atlas = CartographyAtlas::new();
        let mut p1 = SystemProfile::new("api-a", SystemClass::RestApi);
        p1.add_hint("json");
        p1.add_approach("use-bearer-token");
        atlas.add(p1).unwrap();

        let mut p2 = SystemProfile::new("api-b", SystemClass::RestApi);
        p2.add_hint("json");
        atlas.add(p2).unwrap();

        let count = atlas.transfer_knowledge("api-b").unwrap();
        assert!(count > 0);
        let target = atlas.get("api-b").unwrap();
        assert!(target.approaches.contains(&"use-bearer-token".to_string()));
    }
}
