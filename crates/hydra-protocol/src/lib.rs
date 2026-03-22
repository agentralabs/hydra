//! `hydra-protocol` — Any protocol. Hydra discovers, adapts, reaches.
//!
//! REST, GraphQL, gRPC, WebSocket, FIX, MQTT, AMQP,
//! Modbus, CAN bus, COBOL/JCL, custom binary.
//! If it speaks a protocol — Hydra can reach it.
//! Every protocol event is receipted (constitutional).

pub mod adapter;
pub mod constants;
pub mod engine;
pub mod errors;
pub mod family;
pub mod lifecycle;

pub use adapter::{adapt_to_protocol, AdaptationResult, AdaptedRequest};
pub use engine::{ProtocolEngine, ProtocolResult};
pub use errors::ProtocolError;
pub use family::{infer_from_target, ProtocolFamily, ProtocolHint};
pub use lifecycle::{ConnectionLifecycle, ConnectionState};
