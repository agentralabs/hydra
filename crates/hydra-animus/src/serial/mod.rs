//! Serialization and deserialization for Animus Prime.

pub mod binary;
pub mod schema;

pub use binary::{deserialize_graph, deserialize_signal, serialize_graph, serialize_signal};
pub use schema::AnimusHeader;
