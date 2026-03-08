use std::path::{Path, PathBuf};

/// Single-instance lock file
pub struct InstanceLock {
    lock_path: PathBuf,
    held: bool,
}

impl InstanceLock {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            lock_path: data_dir.join("hydra.lock"),
            held: false,
        }
    }

    /// Acquire the lock — returns false if another instance is running
    pub fn acquire(&mut self) -> Result<(), LockError> {
        if self.lock_path.exists() {
            // Check if the lock is stale (process dead)
            if let Ok(contents) = std::fs::read_to_string(&self.lock_path) {
                if let Ok(pid) = contents.trim().parse::<u32>() {
                    if is_process_alive(pid) {
                        return Err(LockError::AlreadyRunning(pid));
                    }
                }
            }
            // Stale lock — remove it
            let _ = std::fs::remove_file(&self.lock_path);
        }

        // Create lock with our PID
        let pid = std::process::id();
        if let Some(parent) = self.lock_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&self.lock_path, pid.to_string())
            .map_err(|e| LockError::IoError(e.to_string()))?;
        self.held = true;
        Ok(())
    }

    /// Release the lock
    pub fn release(&mut self) {
        if self.held {
            let _ = std::fs::remove_file(&self.lock_path);
            self.held = false;
        }
    }

    /// Check if lock is held
    pub fn is_held(&self) -> bool {
        self.held
    }

    /// Check if lock file exists (for stale detection)
    pub fn lock_exists(&self) -> bool {
        self.lock_path.exists()
    }
}

impl Drop for InstanceLock {
    fn drop(&mut self) {
        self.release();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockError {
    AlreadyRunning(u32),
    IoError(String),
}

impl std::fmt::Display for LockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyRunning(pid) => write!(
                f, "Another Hydra instance is running (PID {pid}). Only one instance can run at a time. Stop the other instance first."
            ),
            Self::IoError(msg) => write!(
                f, "Cannot create lock file. {msg}. Check directory permissions."
            ),
        }
    }
}

fn is_process_alive(_pid: u32) -> bool {
    // Simple check: try to read /proc/{pid} on Linux or use sysctl on macOS
    // For portability, just check if the lock file's PID matches a running process
    // In production, use the `sysinfo` crate. For now, assume stale.
    false
}
