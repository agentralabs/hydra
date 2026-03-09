//! Lock management with stale recovery.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

const STALE_THRESHOLD: Duration = Duration::from_secs(5 * 60);

/// Error from lock operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockError {
    AlreadyHeld(u32),
    IoError(String),
    StaleRecoveryFailed(String),
}

impl std::fmt::Display for LockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyHeld(pid) => {
                write!(f, "Hydra is already running on this project (PID {pid}). Use a different terminal for a different project.")
            }
            Self::IoError(msg) => write!(f, "Lock I/O error: {msg}"),
            Self::StaleRecoveryFailed(msg) => write!(f, "Stale lock recovery failed: {msg}"),
        }
    }
}

impl std::error::Error for LockError {}

/// Manages a file-based lock with PID + timestamp and stale detection.
pub struct LockManager {
    lock_path: PathBuf,
}

impl LockManager {
    pub fn new(lock_path: PathBuf) -> Self {
        Self { lock_path }
    }

    /// Acquire the lock. Writes PID + timestamp. Returns a LockGuard that releases on drop.
    pub fn acquire(&self) -> Result<LockGuard, LockError> {
        if self.lock_path.exists() {
            if self.is_stale() {
                self.recover_stale()?;
            } else {
                // Read existing PID
                if let Ok(contents) = std::fs::read_to_string(&self.lock_path) {
                    if let Some(pid_str) = contents.lines().next() {
                        if let Ok(pid) = pid_str.trim().parse::<u32>() {
                            return Err(LockError::AlreadyHeld(pid));
                        }
                    }
                }
                return Err(LockError::IoError("lock file exists but unreadable".into()));
            }
        }

        // Create parent directory
        if let Some(parent) = self.lock_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| LockError::IoError(e.to_string()))?;
        }

        let pid = std::process::id();
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let content = format!("{pid}\n{timestamp}");
        std::fs::write(&self.lock_path, &content)
            .map_err(|e| LockError::IoError(e.to_string()))?;

        Ok(LockGuard {
            lock_path: self.lock_path.clone(),
        })
    }

    /// Check if the lock is stale: PID is not alive, or lock file age > 5 minutes.
    pub fn is_stale(&self) -> bool {
        let contents = match std::fs::read_to_string(&self.lock_path) {
            Ok(c) => c,
            Err(_) => return false,
        };

        let mut lines = contents.lines();

        // Check PID liveness
        if let Some(pid_str) = lines.next() {
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                if !is_process_alive(pid) {
                    return true;
                }
            }
        }

        // Check timestamp age
        if let Some(ts_str) = lines.next() {
            if let Ok(ts) = ts_str.trim().parse::<u64>() {
                let now = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                if now.saturating_sub(ts) > STALE_THRESHOLD.as_secs() {
                    return true;
                }
            }
        }

        false
    }

    /// Remove a stale lock file.
    pub fn recover_stale(&self) -> Result<(), LockError> {
        std::fs::remove_file(&self.lock_path)
            .map_err(|e| LockError::StaleRecoveryFailed(e.to_string()))
    }
}

/// RAII guard that removes the lock file on drop.
pub struct LockGuard {
    lock_path: PathBuf,
}

impl LockGuard {
    /// Path to the held lock file.
    pub fn path(&self) -> &PathBuf {
        &self.lock_path
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.lock_path);
    }
}

/// Check if a process is alive using kill -0 via Command.
fn is_process_alive(pid: u32) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
