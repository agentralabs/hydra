//! All constants for hydra-reach.
//! No magic numbers or strings anywhere else in this crate.

/// Local server default port.
pub const REACH_DEFAULT_PORT: u16 = 7474;

/// Maximum simultaneous device connections.
pub const MAX_DEVICE_CONNECTIONS: usize = 32;

/// Session continuity handoff timeout (seconds).
pub const SESSION_HANDOFF_TIMEOUT_SECONDS: u64 = 30;

/// Device heartbeat interval (seconds).
pub const DEVICE_HEARTBEAT_SECONDS: u64 = 10;

/// Authentication token byte length.
pub const AUTH_TOKEN_BYTES: usize = 32;

/// Maximum context bytes transferred on session handoff.
pub const MAX_HANDOFF_CONTEXT_BYTES: usize = 64 * 1024;
