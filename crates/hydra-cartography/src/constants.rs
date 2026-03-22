//! Constants for the cartography crate.

/// Maximum number of system profiles in the atlas.
pub const MAX_SYSTEM_PROFILES: usize = 1_000_000;

/// Similarity threshold for topology neighbor discovery.
pub const TOPOLOGY_SIMILARITY_THRESHOLD: f64 = 0.65;

/// Maximum neighbors per system in the topology map.
pub const MAX_TOPOLOGY_NEIGHBORS: usize = 20;

/// Default confidence for knowledge transferred from a neighbor.
pub const KNOWLEDGE_TRANSFER_CONFIDENCE: f64 = 0.6;
