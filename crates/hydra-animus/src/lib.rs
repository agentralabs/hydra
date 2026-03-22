//! `hydra-animus` — The Animus Prime runtime.
//!
//! Hydra's internal communication language.
//! Every module communicates via Prime graphs — zero information loss.
//!
//! ## The Signal Causal Semiring
//! All inter-module messages form the semiring (S, +, *, 0, 1):
//!   - `*` = causal composition ("a caused b")
//!   - `+` = signal merge ("a and b both contributed")
//!   - `0` = null signal (additive identity)
//!   - `1` = constitutional identity (multiplicative identity)
//!
//! Every signal's causal chain must terminate at 1.
//! Orphan signals (incomplete chains) are rejected at the bus level.

pub mod bridge;
pub mod bus;
pub mod constants;
pub mod errors;
pub mod graph;
pub mod semiring;
pub mod serial;
pub mod vocab;

// Top-level re-exports for convenience
pub use bridge::{graph_to_text, text_to_signal, HumanReadable, ResolvedIntent};
pub use bus::{validate_for_bus, RoutingDecision};
pub use errors::AnimusError;
pub use graph::{Edge, EdgeType, Node, NodeId, NodeType, PrimeGraph, Proof};
pub use semiring::{
    compose, compute_weight, is_orphan, merge, validate_chain, verify_coefficient_sum, Signal,
    SignalId, SignalTier, SignalWeight, WeightInputs,
};
pub use serial::{
    deserialize_graph, deserialize_signal, serialize_graph, serialize_signal, AnimusHeader,
};
pub use vocab::{
    growth_layer, growth_node_type, is_growth_edge_type, is_growth_node_type, DomainVocab,
    GrowthLayer, VocabRegistry, GROWTH_EDGE_TYPES, GROWTH_NODE_TYPES,
};
