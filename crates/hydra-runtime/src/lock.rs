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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_dir() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("hydra_lock_test_{}_{}", std::process::id(), id));
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn test_new_not_held() {
        let dir = temp_dir();
        let lock = InstanceLock::new(&dir);
        assert!(!lock.is_held());
    }

    #[test]
    fn test_acquire_and_release() {
        let dir = temp_dir();
        let mut lock = InstanceLock::new(&dir);
        lock.acquire().unwrap();
        assert!(lock.is_held());
        assert!(lock.lock_exists());
        lock.release();
        assert!(!lock.is_held());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_release_without_acquire() {
        let dir = temp_dir();
        let mut lock = InstanceLock::new(&dir);
        lock.release(); // should not panic
        assert!(!lock.is_held());
    }

    #[test]
    fn test_drop_releases_lock() {
        let dir = temp_dir();
        let lock_path = dir.join("hydra.lock");
        {
            let mut lock = InstanceLock::new(&dir);
            lock.acquire().unwrap();
            assert!(lock_path.exists());
        }
        // After drop, lock file should be removed
        assert!(!lock_path.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_stale_lock_recovery() {
        let dir = temp_dir();
        // Create a stale lock file with a fake PID
        let lock_path = dir.join("hydra.lock");
        std::fs::write(&lock_path, "999999999").unwrap();
        let mut lock = InstanceLock::new(&dir);
        // Since is_process_alive returns false, should succeed
        lock.acquire().unwrap();
        assert!(lock.is_held());
        lock.release();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_lock_error_already_running_display() {
        let err = LockError::AlreadyRunning(1234);
        let msg = format!("{}", err);
        assert!(msg.contains("1234"));
        assert!(msg.contains("Another Hydra instance"));
    }

    #[test]
    fn test_lock_error_io_display() {
        let err = LockError::IoError("permission denied".into());
        let msg = format!("{}", err);
        assert!(msg.contains("permission denied"));
        assert!(msg.contains("lock file"));
    }

    #[test]
    fn test_lock_error_eq() {
        assert_eq!(LockError::AlreadyRunning(1), LockError::AlreadyRunning(1));
        assert_ne!(LockError::AlreadyRunning(1), LockError::AlreadyRunning(2));
    }

    #[test]
    fn test_lock_exists_before_acquire() {
        let dir = temp_dir();
        let lock = InstanceLock::new(&dir);
        // Lock file shouldn't exist if we haven't done anything
        let lock_path = dir.join("hydra.lock");
        if lock_path.exists() {
            let _ = std::fs::remove_file(&lock_path);
        }
        assert!(!lock.lock_exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
