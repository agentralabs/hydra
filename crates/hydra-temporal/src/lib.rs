//! `hydra-temporal` — Time as a first-class constraint.
//!
//! The structural foundation of Hydra memory. Every memory has a
//! nanosecond timestamp, a position in the manifold, and a causal
//! root. Decisions decay exponentially. The B+ tree is append-only.

pub mod btree;
pub mod causal_index;
pub mod constants;
pub mod constraint;
pub mod decay;
pub mod decision_graph;
pub mod errors;
pub mod query;
pub mod spatial;
pub mod timestamp;

// Re-exports for convenience
pub use btree::{ChronoSpatialBTree, ManifoldCoord, MemoryId, TemporalEntry};
pub use causal_index::CausalChainIndex;
pub use constraint::{ConstraintKind, DecisionConstraint, DecisionId};
pub use decay::ConstraintDecay;
pub use decision_graph::{ConstraintConflict, DecisionGraph};
pub use errors::TemporalError;
pub use query::{QueryResult, QueryType, TemporalQueryEngine};
pub use spatial::SpatialPartitionIndex;
pub use timestamp::Timestamp;
