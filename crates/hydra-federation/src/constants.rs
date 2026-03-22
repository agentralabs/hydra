/// Maximum peers in the local registry.
pub const MAX_REGISTERED_PEERS: usize = 1_000;

/// Maximum active sessions at once.
pub const MAX_ACTIVE_SESSIONS: usize = 50;

/// Default session duration (hours).
pub const DEFAULT_SESSION_HOURS: i64 = 24;

/// Minimum trust score to initiate federation.
pub const MIN_FEDERATION_TRUST: f64 = 0.65;

/// Maximum scope items per trust negotiation.
pub const MAX_SCOPE_ITEMS: usize = 20;

/// Peer identity verification timeout (ms).
pub const IDENTITY_VERIFY_TIMEOUT_MS: u64 = 5_000;

/// Session receipt hash label.
pub const FEDERATION_HASH_LABEL: &str = "sha256-federation";
