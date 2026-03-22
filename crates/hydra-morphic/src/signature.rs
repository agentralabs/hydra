//! Morphic signature — an unforgeable, monotonically deepening identity hash.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::constants::{
    MORPHIC_CAPABILITY_WEIGHT, MORPHIC_MEMORY_WEIGHT, MORPHIC_MODIFICATION_WEIGHT, MORPHIC_VERSION,
};

/// The morphic signature: a chain of hashes representing identity evolution.
/// The signature NEVER decreases in depth — it only deepens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MorphicSignature {
    /// Current version of this signature format.
    pub version: u32,
    /// The current hash — chained from all prior states.
    pub current_hash: String,
    /// How many times this signature has been deepened.
    pub depth: u64,
    /// When this signature was created.
    pub created_at: DateTime<Utc>,
    /// When this signature was last deepened.
    pub last_deepened: DateTime<Utc>,
    /// Number of restarts recorded in this signature.
    pub restart_count: u64,
}

impl MorphicSignature {
    /// Create a genesis signature for a new identity.
    pub fn genesis() -> Self {
        let now = Utc::now();
        let genesis_input = format!("hydra-morphic-genesis-v{MORPHIC_VERSION}-{now}");
        let hash = Self::compute_hash(&genesis_input);

        Self {
            version: MORPHIC_VERSION,
            current_hash: hash,
            depth: 0,
            created_at: now,
            last_deepened: now,
            restart_count: 0,
        }
    }

    /// Deepen the signature by chaining a new hash from the prior hash and event data.
    /// This ALWAYS increases depth — the signature never shrinks.
    pub fn deepen(&mut self, event_data: &str) {
        let input = format!("{}:{}", self.current_hash, event_data);
        self.current_hash = Self::compute_hash(&input);
        self.depth += 1;
        self.last_deepened = Utc::now();
    }

    /// Record a system restart in the signature.
    pub fn record_restart(&mut self) {
        self.restart_count += 1;
        self.deepen("system-restart");
    }

    /// Compute the identity distance between this signature and another.
    /// Returns a value in [0.0, 1.0] where 0.0 means identical and 1.0 means
    /// completely different.
    ///
    /// Uses three weighted components:
    /// - Capability divergence (hash difference)
    /// - Modification history depth difference
    /// - Memory continuity (restart difference)
    pub fn distance(&self, other: &MorphicSignature) -> f64 {
        // Component 1: hash divergence (0.0 if same, 1.0 if different)
        let hash_diff = if self.current_hash == other.current_hash {
            0.0
        } else {
            1.0
        };

        // Component 2: depth divergence (normalized)
        let max_depth = self.depth.max(other.depth).max(1) as f64;
        let depth_diff = (self.depth as f64 - other.depth as f64).abs() / max_depth;

        // Component 3: restart divergence (normalized)
        let max_restarts = self.restart_count.max(other.restart_count).max(1) as f64;
        let restart_diff =
            (self.restart_count as f64 - other.restart_count as f64).abs() / max_restarts;

        let distance = MORPHIC_CAPABILITY_WEIGHT * hash_diff
            + MORPHIC_MODIFICATION_WEIGHT * depth_diff
            + MORPHIC_MEMORY_WEIGHT * restart_diff;

        distance.clamp(0.0, 1.0)
    }

    /// Compute a SHA-256 hash of the input, returning a hex string.
    fn compute_hash(input: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        hex::encode(hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genesis_creates_depth_zero() {
        let sig = MorphicSignature::genesis();
        assert_eq!(sig.depth, 0);
        assert_eq!(sig.version, MORPHIC_VERSION);
        assert!(!sig.current_hash.is_empty());
    }

    #[test]
    fn deepen_increases_depth_and_changes_hash() {
        let mut sig = MorphicSignature::genesis();
        let original_hash = sig.current_hash.clone();
        sig.deepen("test-event");
        assert_eq!(sig.depth, 1);
        assert_ne!(sig.current_hash, original_hash);
    }

    #[test]
    fn distance_to_self_is_zero() {
        let sig = MorphicSignature::genesis();
        let distance = sig.distance(&sig);
        assert!((distance - 0.0).abs() < 1e-10);
    }

    #[test]
    fn distance_increases_with_divergence() {
        let mut a = MorphicSignature::genesis();
        let b = a.clone();
        a.deepen("diverging-event");
        let distance = a.distance(&b);
        assert!(distance > 0.0);
    }

    #[test]
    fn restart_recorded_in_signature() {
        let mut sig = MorphicSignature::genesis();
        sig.record_restart();
        assert_eq!(sig.restart_count, 1);
        assert_eq!(sig.depth, 1);
    }
}
