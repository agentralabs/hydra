//! Constants for protocol adaptation and connection management.

/// Protocol discovery timeout (ms).
pub const DISCOVERY_TIMEOUT_MS: u64 = 5_000;

/// Maximum protocol adapters registered.
pub const MAX_PROTOCOL_ADAPTERS: usize = 500;

/// Connection retry attempts before escalating.
pub const MAX_CONNECTION_RETRIES: u32 = 3;

/// Connection backoff base (ms) — doubles each retry.
pub const CONNECTION_BACKOFF_BASE_MS: u64 = 500;

/// Maximum connection pool size.
pub const MAX_CONNECTION_POOL_SIZE: usize = 100;

/// Receipt required for every protocol event.
pub const PROTOCOL_EVENTS_RECEIPTED: bool = true;
