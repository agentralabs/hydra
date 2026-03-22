//! PerceptionHorizon — what Hydra can currently sense and receive.
//! Expands with genome entries, connected sisters, and mapped systems.
//! Never contracts.

use crate::constants::*;
use crate::errors::HorizonError;
use serde::{Deserialize, Serialize};

/// What contributes to perception horizon expansion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PerceptionExpansion {
    /// New genome entry recorded — Hydra can recognize more situations.
    GenomeEntry {
        /// Number of new genome entries.
        count_delta: u64,
    },
    /// New system mapped in cartography — new data source available.
    SystemMapped {
        /// Name of the mapped system.
        system_name: String,
    },
    /// New sister connected — new signal type receivable.
    SisterConnected {
        /// Name of the connected sister.
        sister_name: String,
    },
    /// New device surface connected.
    DeviceConnected {
        /// Class of the connected device.
        device_class: String,
    },
    /// New skill loaded with perception capabilities.
    SkillLoaded {
        /// Name of the loaded skill.
        skill_name: String,
    },
}

/// The perception horizon — what Hydra can currently sense.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerceptionHorizon {
    /// Current horizon value (0.0 = blind, 1.0 = theoretical maximum).
    pub value: f64,
    /// Total expansion events recorded.
    pub expansion_count: u64,
    /// Genome entries contributing to perception.
    pub genome_entries: u64,
    /// Systems mapped (cartography).
    pub systems_mapped: u64,
    /// Sisters connected.
    pub sisters_connected: u64,
    /// Devices ever connected.
    pub devices_seen: u64,
}

impl PerceptionHorizon {
    /// Create a new perception horizon at the initial value.
    pub fn new() -> Self {
        Self {
            value: PERCEPTION_HORIZON_INITIAL,
            expansion_count: 0,
            genome_entries: 0,
            systems_mapped: 0,
            sisters_connected: 0,
            devices_seen: 0,
        }
    }

    /// Return the current perception horizon value.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Return the total number of expansions applied.
    pub fn expansion_count(&self) -> u64 {
        self.expansion_count
    }

    /// Expand the horizon. Returns error if contraction attempted.
    pub fn expand(&mut self, reason: PerceptionExpansion) -> Result<f64, HorizonError> {
        let before = self.value;

        let delta = match &reason {
            PerceptionExpansion::GenomeEntry { count_delta } => {
                self.genome_entries += count_delta;
                *count_delta as f64 * PERCEPTION_GENOME_FACTOR
            }
            PerceptionExpansion::SystemMapped { .. } => {
                self.systems_mapped += 1;
                0.005
            }
            PerceptionExpansion::SisterConnected { .. } => {
                self.sisters_connected += 1;
                0.02
            }
            PerceptionExpansion::DeviceConnected { .. } => {
                self.devices_seen += 1;
                0.003
            }
            PerceptionExpansion::SkillLoaded { .. } => 0.01,
        };

        if delta < HORIZON_MIN_DELTA {
            return Err(HorizonError::ContractionAttempted);
        }

        self.value = (self.value + delta).min(HORIZON_MAX);
        self.expansion_count += 1;
        Ok(self.value - before)
    }

    /// Check whether the perception horizon is above a threshold.
    pub fn is_above(&self, threshold: f64) -> bool {
        self.value >= threshold
    }
}

impl Default for PerceptionHorizon {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_at_initial_value() {
        let h = PerceptionHorizon::new();
        assert!((h.value - PERCEPTION_HORIZON_INITIAL).abs() < 1e-10);
    }

    #[test]
    fn genome_entry_expands_horizon() {
        let mut h = PerceptionHorizon::new();
        let before = h.value;
        h.expand(PerceptionExpansion::GenomeEntry { count_delta: 100 })
            .unwrap();
        assert!(h.value > before);
    }

    #[test]
    fn horizon_never_exceeds_max() {
        let mut h = PerceptionHorizon::new();
        for _ in 0..10_000 {
            h.expand(PerceptionExpansion::SystemMapped {
                system_name: "sys".into(),
            })
            .unwrap();
        }
        assert!(h.value <= HORIZON_MAX);
    }

    #[test]
    fn expansion_count_tracked() {
        let mut h = PerceptionHorizon::new();
        h.expand(PerceptionExpansion::SisterConnected {
            sister_name: "memory".into(),
        })
        .unwrap();
        h.expand(PerceptionExpansion::SisterConnected {
            sister_name: "forge".into(),
        })
        .unwrap();
        assert_eq!(h.expansion_count, 2);
    }

    #[test]
    fn tracking_counters_updated() {
        let mut h = PerceptionHorizon::new();
        h.expand(PerceptionExpansion::GenomeEntry { count_delta: 50 })
            .unwrap();
        assert_eq!(h.genome_entries, 50);
        h.expand(PerceptionExpansion::SystemMapped {
            system_name: "fs".into(),
        })
        .unwrap();
        assert_eq!(h.systems_mapped, 1);
        h.expand(PerceptionExpansion::DeviceConnected {
            device_class: "mobile".into(),
        })
        .unwrap();
        assert_eq!(h.devices_seen, 1);
    }
}
