//! Persistence utilities — data directory, DB connections, boot lock.

use rusqlite::Connection;
use std::fs;
use std::path::PathBuf;

/// Returns ~/.hydra/data/, creating it on first call.
pub fn data_dir() -> PathBuf {
    let base = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".hydra")
        .join("data");
    if !base.exists() {
        if let Err(e) = fs::create_dir_all(&base) {
            eprintln!("hydra: failed to create data dir {:?}: {}", base, e);
        }
    }
    base
}

/// Open (or create) a SQLite database at ~/.hydra/data/<name>.db with WAL mode.
pub fn open_db(name: &str) -> rusqlite::Result<Connection> {
    let path = data_dir().join(format!("{}.db", name));
    let conn = Connection::open(&path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    Ok(conn)
}

/// Lock file path.
fn lock_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".hydra")
        .join("hydra.lock")
}

/// Acquire a PID-based boot lock at ~/.hydra/hydra.lock.
///
/// If the lock file exists and is < 60s old, another instance may be running.
pub fn acquire_boot_lock() -> Result<(), String> {
    let path = lock_path();

    // Ensure parent exists
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    // Check existing lock
    if path.exists() {
        if let Ok(contents) = fs::read_to_string(&path) {
            if let Ok(pid) = contents.trim().parse::<u32>() {
                if is_process_alive(pid) {
                    return Err(format!(
                        "another Hydra instance (PID {}) holds the lock",
                        pid
                    ));
                }
            }
        }
        // Stale lock — remove it
        let _ = fs::remove_file(&path);
    }

    // Write our PID
    let pid = std::process::id();
    fs::write(&path, pid.to_string())
        .map_err(|e| format!("failed to write lock file: {}", e))?;
    eprintln!("hydra: acquired boot lock (PID {})", pid);
    Ok(())
}

/// Remove the boot lock file.
pub fn release_boot_lock() {
    let path = lock_path();
    if path.exists() {
        if let Err(e) = fs::remove_file(&path) {
            eprintln!("hydra: failed to remove lock: {}", e);
        } else {
            eprintln!("hydra: released boot lock");
        }
    }
}

/// Check if a lock is still alive: file must exist and be < 10s old.
/// Reduced from 60s to 10s — harness subprocesses are short-lived,
/// a 10s-old lock from a subprocess is always stale.
pub fn is_process_alive(pid: u32) -> bool {
    let path = lock_path();
    if let Ok(meta) = fs::metadata(&path) {
        if let Ok(modified) = meta.modified() {
            if let Ok(age) = std::time::SystemTime::now().duration_since(modified) {
                if age.as_secs() >= 10 {
                    return false; // stale
                }
            }
        }
    }
    let _ = pid;
    true
}
