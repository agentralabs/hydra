//! Obstacle classification types.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Classification of an obstacle encountered by Hydra.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ObstacleClass {
    /// An authentication challenge.
    AuthChallenge,
    /// A rate limit was hit.
    RateLimit,
    /// A network block or connectivity issue.
    NetworkBlock,
    /// A protocol mismatch.
    ProtocolMismatch,
    /// A permission was denied.
    PermissionDenied,
    /// A format incompatibility.
    FormatIncompat,
    /// Resource exhaustion (memory, disk, CPU).
    ResourceExhaustion,
    /// A timeout pattern.
    TimeoutPattern,
    /// A concurrency conflict.
    ConcurrencyConflict,
    /// A missing dependency.
    DependencyMissing,
    /// An environment constraint.
    EnvironmentConstraint,
    /// A tool was not found.
    ToolNotFound,
    /// An unknown system type was encountered.
    UnknownSystemType,
    /// Any other obstacle.
    Other,
}

/// A signature describing an obstacle instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObstacleSignature {
    /// The class of obstacle.
    pub class: ObstacleClass,
    /// Human-readable description.
    pub description: String,
    /// Keywords extracted from the obstacle context.
    pub keywords: BTreeSet<String>,
}

impl ObstacleSignature {
    /// Create a new obstacle signature.
    pub fn new(class: ObstacleClass, description: impl Into<String>) -> Self {
        let desc: String = description.into();
        let keywords: BTreeSet<String> = desc
            .split_whitespace()
            .map(|w| w.to_lowercase())
            .filter(|w| w.len() >= 3)
            .collect();
        Self {
            class,
            description: desc,
            keywords,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn obstacle_signature_extracts_keywords() {
        let sig = ObstacleSignature::new(ObstacleClass::RateLimit, "API rate limit exceeded");
        assert!(sig.keywords.contains("api"));
        assert!(sig.keywords.contains("rate"));
        assert!(sig.keywords.contains("limit"));
    }
}
