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
