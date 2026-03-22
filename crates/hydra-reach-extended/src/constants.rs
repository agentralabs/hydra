/// Maximum path attempts before declaring HardDenied.
pub const MAX_PATH_ATTEMPTS: u32 = 8;

/// Connection attempt timeout (ms).
pub const CONNECTION_TIMEOUT_MS: u64 = 10_000;

/// Patience strategy base wait (ms) — doubles each retry.
pub const PATIENCE_BASE_MS: u64 = 1_000;

/// Maximum concurrent reach sessions.
pub const MAX_REACH_SESSIONS: usize = 200;

/// Rate limit detection: requests per minute threshold.
pub const RATE_LIMIT_THRESHOLD_RPM: u32 = 100;

/// Maximum sessions per target.
pub const MAX_SESSIONS_PER_TARGET: usize = 5;

/// Cartography: minimum encounters before topology inference.
pub const MIN_ENCOUNTERS_FOR_TOPOLOGY: usize = 2;
