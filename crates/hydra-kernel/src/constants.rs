//! All constants for hydra-kernel.
//! No magic numbers anywhere else in this crate.

/// Alive loop integration step — how often AMBIENT thread runs (ms).
pub const AMBIENT_INTERVAL_MS: u64 = 100;

/// Dream thread runs during idle periods at this interval (ms).
pub const DREAM_INTERVAL_MS: u64 = 500;

/// Constitutional invariant check interval — same as AMBIENT (ms).
pub const INVARIANT_CHECK_INTERVAL_MS: u64 = 100;

/// Maximum time allowed for boot sequence before timeout (seconds).
pub const BOOT_TIMEOUT_SECONDS: u64 = 30;

/// Maximum queued signals in the active thread before backpressure.
pub const ACTIVE_SIGNAL_QUEUE_CAPACITY: usize = 4_096;

/// Maximum queued signals in the ambient thread.
pub const AMBIENT_SIGNAL_QUEUE_CAPACITY: usize = 8_192;

/// Lyapunov stability threshold — below this V(Psi) triggers alert.
pub const LYAPUNOV_ALERT_THRESHOLD: f64 = 0.0;

/// Lyapunov critical threshold — below this triggers degraded mode.
pub const LYAPUNOV_CRITICAL_THRESHOLD: f64 = -0.5;

/// Maximum tasks in flight simultaneously before backpressure.
pub const MAX_CONCURRENT_TASKS: usize = 256;

/// Task approach cycle limit before surfacing to principal (not a stop).
pub const TASK_PROGRESS_REPORT_INTERVAL: u32 = 10;

/// Kernel health report interval for TUI (ms).
pub const HEALTH_REPORT_INTERVAL_MS: u64 = 1_000;

/// Dream thread CPU priority (0 = lowest, 10 = highest).
pub const DREAM_THREAD_PRIORITY: u8 = 2;

/// Ambient thread CPU priority.
pub const AMBIENT_THREAD_PRIORITY: u8 = 7;

/// Active thread CPU priority.
pub const ACTIVE_THREAD_PRIORITY: u8 = 9;

/// How long the kernel waits for graceful shutdown before forcing (ms).
pub const GRACEFUL_SHUTDOWN_TIMEOUT_MS: u64 = 5_000;

/// Kernel version string.
pub const KERNEL_VERSION: &str = "0.1.0";
