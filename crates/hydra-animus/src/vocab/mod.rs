//! Vocabulary system — base types and domain-registered extensions.

pub mod base;
pub mod domain;
pub mod growth;

pub use base::{is_base_edge_type, is_base_node_type, BASE_EDGE_TYPES, BASE_NODE_TYPES};
pub use domain::{DomainVocab, VocabEntry, VocabRegistry};
pub use growth::{
    growth_layer, growth_node_type, is_growth_edge_type, is_growth_node_type, GrowthLayer,
    GROWTH_EDGE_TYPES, GROWTH_NODE_TYPES,
};
