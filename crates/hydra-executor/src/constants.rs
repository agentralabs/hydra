//! Constants for the executor crate.
//!
//! All tunables live here — no magic numbers elsewhere.

/// Maximum approach attempts before HardDenied.
pub const MAX_APPROACH_ATTEMPTS: u32 = 13;

/// Maximum wait time for a PatienceStrategy retry (seconds).
pub const MAX_PATIENCE_WAIT_SECONDS: u64 = 300;

/// Execution receipt hash algorithm label.
pub const RECEIPT_HASH_LABEL: &str = "sha256";

/// Maximum actions in the registry.
pub const MAX_REGISTERED_ACTIONS: usize = 10_000;

/// Shadow execution timeout (ms) — dry run before real execution.
pub const SHADOW_EXECUTION_TIMEOUT_MS: u64 = 500;

/// Maximum concurrent executions per engine instance.
pub const MAX_CONCURRENT_EXECUTIONS: usize = 50;

/// HardDenied evidence requirement strings.
pub const HARD_DENIED_AUTH: &str = "EXPLICIT_AUTH_DENIAL";
pub const HARD_DENIED_PRINCIPAL: &str = "PRINCIPAL_CANCELLATION";
pub const HARD_DENIED_CONSTITUTIONAL: &str = "CONSTITUTIONAL_VIOLATION";

// ── Bridge connector constants ──

/// Bridge health check interval (ms).
pub const BRIDGE_HEALTH_CHECK_INTERVAL_MS: u64 = 30_000;

/// Maximum bridge restart attempts before giving up.
pub const BRIDGE_MAX_RESTART_ATTEMPTS: u32 = 5;

/// Bridge startup timeout (ms).
pub const BRIDGE_STARTUP_TIMEOUT_MS: u64 = 10_000;

/// Base delay for bridge restart backoff (ms).
pub const BRIDGE_RESTART_BACKOFF_BASE_MS: u64 = 1_000;

/// Maximum delay for bridge restart backoff (ms).
pub const BRIDGE_RESTART_BACKOFF_MAX_MS: u64 = 60_000;

/// Maximum active bridges.
pub const MAX_ACTIVE_BRIDGES: usize = 20;

// ── Local connector constants ──

/// Maximum file size for local filesystem reads (10MB).
pub const LOCAL_FS_MAX_FILE_SIZE_BYTES: usize = 10_485_760;

/// AppleScript execution timeout (ms).
pub const LOCAL_APPLESCRIPT_TIMEOUT_MS: u64 = 5_000;

/// Local HTTP request timeout (ms).
pub const LOCAL_HTTP_TIMEOUT_MS: u64 = 10_000;

/// Maximum directory depth for recursive filesystem operations.
pub const LOCAL_FS_MAX_DEPTH: usize = 10;
