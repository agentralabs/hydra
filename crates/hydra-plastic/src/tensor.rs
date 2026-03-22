//! Plasticity tensor — all environment profiles.

use crate::constants::MAX_ENVIRONMENTS;
use crate::errors::PlasticError;
use crate::mode::ExecutionMode;
use crate::environment::EnvironmentProfile;
use std::collections::BTreeMap;

/// The plasticity tensor tracking all known environments.
///
/// Append-only: environments are never removed.
#[derive(Debug)]
pub struct PlasticityTensor {
    /// All environment profiles keyed by name.
    environments: BTreeMap<String, EnvironmentProfile>,
    /// Total environments ever added (monotonically increasing).
    total_ever: u64,
}

impl PlasticityTensor {
    /// Create an empty plasticity tensor.
    pub fn new() -> Self {
        Self {
            environments: BTreeMap::new(),
            total_ever: 0,
        }
    }

    /// Add an environment profile to the tensor.
    ///
    /// If an environment with the same name exists, updates it instead.
    /// Returns an error if the tensor is at capacity and the name is new.
    pub fn add(&mut self, profile: EnvironmentProfile) -> Result<(), PlasticError> {
        if !self.environments.contains_key(&profile.name)
            && self.environments.len() >= MAX_ENVIRONMENTS
        {
            return Err(PlasticError::TensorFull {
                max: MAX_ENVIRONMENTS,
            });
        }
        let is_new = !self.environments.contains_key(&profile.name);
        self.environments.insert(profile.name.clone(), profile);
        if is_new {
            self.total_ever += 1;
        }
        Ok(())
    }

    /// Get a reference to an environment profile by name.
    pub fn get(&self, name: &str) -> Option<&EnvironmentProfile> {
        self.environments.get(name)
    }

    /// Get a mutable reference to an environment profile by name.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut EnvironmentProfile> {
        self.environments.get_mut(name)
    }

    /// Find the optimal execution mode for a named environment.
    ///
    /// Returns the mode with the highest confidence. If the environment
    /// is not found, returns `None`.
    pub fn optimal_mode_for(&self, env_name: &str) -> Option<&ExecutionMode> {
        self.environments.get(env_name).map(|p| &p.mode)
    }

    /// Total environments ever added (monotonically increasing).
    pub fn total_ever(&self) -> u64 {
        self.total_ever
    }

    /// Current number of environments.
    pub fn len(&self) -> usize {
        self.environments.len()
    }

    /// Returns true if the tensor is empty.
    pub fn is_empty(&self) -> bool {
        self.environments.is_empty()
    }
}

impl Default for PlasticityTensor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_retrieve() {
        let mut tensor = PlasticityTensor::new();
        tensor
            .add(EnvironmentProfile::new(
                "local",
                ExecutionMode::NativeBinary,
            ))
            .unwrap();
        assert!(tensor.get("local").is_some());
        assert_eq!(tensor.total_ever(), 1);
    }

    #[test]
    fn optimal_mode() {
        let mut tensor = PlasticityTensor::new();
        tensor
            .add(EnvironmentProfile::new(
                "local",
                ExecutionMode::NativeBinary,
            ))
            .unwrap();
        let mode = tensor.optimal_mode_for("local").unwrap();
        assert_eq!(*mode, ExecutionMode::NativeBinary);
    }

    #[test]
    fn update_existing() {
        let mut tensor = PlasticityTensor::new();
        tensor
            .add(EnvironmentProfile::new(
                "local",
                ExecutionMode::NativeBinary,
            ))
            .unwrap();
        tensor
            .add(EnvironmentProfile::new("local", ExecutionMode::WasmRuntime))
            .unwrap();
        assert_eq!(tensor.len(), 1);
        assert_eq!(tensor.total_ever(), 1);
    }

    #[test]
    fn unknown_returns_none() {
        let tensor = PlasticityTensor::new();
        assert!(tensor.optimal_mode_for("nonexistent").is_none());
    }
}
