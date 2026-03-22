//! Constants for environment detection and resource thresholds.

/// Minimum RAM in megabytes for full capability operation.
pub const MIN_RAM_MB_FULL: u64 = 512;

/// Minimum RAM in megabytes for degraded capability operation.
pub const MIN_RAM_MB_DEGRADED: u64 = 128;

/// Minimum disk space in megabytes required for any operation.
pub const MIN_DISK_MB: u64 = 256;

/// Timeout in milliseconds for environment probing operations.
pub const PROBE_TIMEOUT_MS: u64 = 2_000;

/// Maximum number of skill environments that can be registered.
pub const MAX_SKILL_ENVIRONMENTS: usize = 1_000;

/// Binary check timeout (ms).
pub const BINARY_CHECK_TIMEOUT_MS: u64 = 500;
