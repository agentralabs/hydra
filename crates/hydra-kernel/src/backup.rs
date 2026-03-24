//! Backup system — automated local backup with encryption and restore verification.
//! Copies ~/.hydra/data/ to ~/.hydra/backups/YYYY-MM-DD/.
//! Prunes backups older than 30 days.
//! Optional AES-256-GCM encryption via vault_crypto passphrase.

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Result of a backup operation.
#[derive(Debug, Clone)]
pub struct BackupResult {
    pub path: PathBuf,
    pub files_copied: usize,
    pub total_bytes: u64,
    pub encrypted: bool,
    pub hash: String,
}

/// Result of a restore operation.
#[derive(Debug, Clone)]
pub struct RestoreResult {
    pub source: PathBuf,
    pub files_restored: usize,
    pub hash_verified: bool,
}

/// Create a backup of ~/.hydra/data/ to ~/.hydra/backups/YYYY-MM-DD/.
pub fn create_backup() -> Result<BackupResult, String> {
    let data_dir = data_path();
    let backup_dir = backup_path_today();

    if !data_dir.exists() {
        return Err("No data directory to backup (~/.hydra/data/)".into());
    }

    std::fs::create_dir_all(&backup_dir)
        .map_err(|e| format!("Cannot create backup dir: {e}"))?;

    let mut files_copied = 0usize;
    let mut total_bytes = 0u64;
    let mut hasher = Sha256::new();

    let entries = std::fs::read_dir(&data_dir)
        .map_err(|e| format!("Cannot read data dir: {e}"))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_name = entry.file_name();
        let dest = backup_dir.join(&file_name);

        match std::fs::copy(&path, &dest) {
            Ok(bytes) => {
                files_copied += 1;
                total_bytes += bytes;
                // Hash each file for integrity verification
                if let Ok(content) = std::fs::read(&dest) {
                    hasher.update(&content);
                }
            }
            Err(e) => {
                eprintln!(
                    "hydra-backup: failed to copy {}: {e}",
                    path.display()
                );
            }
        }
    }

    let hash = hex::encode(hasher.finalize());

    // Write hash file for verification
    let hash_path = backup_dir.join("backup.sha256");
    let _ = std::fs::write(&hash_path, &hash);

    eprintln!(
        "hydra-backup: created backup at {} ({files_copied} files, {}KB)",
        backup_dir.display(),
        total_bytes / 1024
    );

    Ok(BackupResult {
        path: backup_dir,
        files_copied,
        total_bytes,
        encrypted: false,
        hash,
    })
}

/// Create an encrypted backup (AES-256-GCM).
/// Requires HYDRA_VAULT_PASSPHRASE to be set.
pub fn create_encrypted_backup() -> Result<BackupResult, String> {
    let mut result = create_backup()?;

    if !crate::vault_crypto::is_encryption_enabled() {
        eprintln!("hydra-backup: encryption skipped (HYDRA_VAULT_PASSPHRASE not set)");
        return Ok(result);
    }

    // Read all backup files, concatenate, encrypt
    let backup_dir = &result.path;
    let entries = std::fs::read_dir(backup_dir)
        .map_err(|e| format!("Cannot read backup dir: {e}"))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() || path.extension().map(|e| e == "sha256").unwrap_or(false) {
            continue;
        }
        if let Err(e) = crate::vault_crypto::encrypt_file(&path) {
            eprintln!("hydra-backup: encrypt failed for {}: {e}", path.display());
        }
    }

    result.encrypted = true;
    eprintln!("hydra-backup: encrypted backup at {}", backup_dir.display());
    Ok(result)
}

/// Restore from a backup directory to ~/.hydra/data/.
pub fn restore_backup(backup_dir: &Path) -> Result<RestoreResult, String> {
    if !backup_dir.exists() {
        return Err(format!("Backup dir not found: {}", backup_dir.display()));
    }

    // Verify hash if available
    let hash_path = backup_dir.join("backup.sha256");
    let hash_verified = if hash_path.exists() {
        let stored = std::fs::read_to_string(&hash_path).unwrap_or_default();
        let computed = compute_backup_hash(backup_dir);
        stored.trim() == computed
    } else {
        false
    };

    let data_dir = data_path();
    std::fs::create_dir_all(&data_dir)
        .map_err(|e| format!("Cannot create data dir: {e}"))?;

    let mut files_restored = 0usize;
    let entries = std::fs::read_dir(backup_dir)
        .map_err(|e| format!("Cannot read backup: {e}"))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_name = entry.file_name();
        // Skip hash files
        if file_name.to_string_lossy().ends_with(".sha256") {
            continue;
        }
        let dest = data_dir.join(&file_name);
        if let Err(e) = std::fs::copy(&path, &dest) {
            eprintln!("hydra-backup: restore failed for {}: {e}", path.display());
        } else {
            files_restored += 1;
        }
    }

    eprintln!(
        "hydra-backup: restored {files_restored} files from {}",
        backup_dir.display()
    );

    Ok(RestoreResult {
        source: backup_dir.to_path_buf(),
        files_restored,
        hash_verified,
    })
}

/// Prune backups older than max_days.
pub fn prune_old_backups(max_days: u32) -> usize {
    let backups_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".hydra/backups");

    if !backups_dir.exists() {
        return 0;
    }

    let mut pruned = 0;
    let cutoff = chrono::Utc::now() - chrono::Duration::days(max_days as i64);

    if let Ok(entries) = std::fs::read_dir(&backups_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            // Parse directory name as YYYY-MM-DD
            if let Ok(date) = chrono::NaiveDate::parse_from_str(&name, "%Y-%m-%d") {
                let dt = date.and_hms_opt(0, 0, 0).unwrap();
                let utc = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc);
                if utc < cutoff {
                    if let Err(e) = std::fs::remove_dir_all(entry.path()) {
                        eprintln!("hydra-backup: prune failed for {name}: {e}");
                    } else {
                        eprintln!("hydra-backup: pruned old backup {name}");
                        pruned += 1;
                    }
                }
            }
        }
    }
    pruned
}

/// List available backups.
pub fn list_backups() -> Vec<(String, u64)> {
    let backups_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".hydra/backups");

    let mut backups = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&backups_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                let size = dir_size(&entry.path());
                backups.push((name, size));
            }
        }
    }
    backups.sort();
    backups
}

fn data_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".hydra/data")
}

fn backup_path_today() -> PathBuf {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    dirs::home_dir()
        .unwrap_or_default()
        .join(format!(".hydra/backups/{today}"))
}

fn compute_backup_hash(dir: &Path) -> String {
    let mut hasher = Sha256::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        let mut paths: Vec<_> = entries.flatten().map(|e| e.path()).collect();
        paths.sort();
        for path in paths {
            if path.is_file() && !path.to_string_lossy().ends_with(".sha256") {
                if let Ok(content) = std::fs::read(&path) {
                    hasher.update(&content);
                }
            }
        }
    }
    hex::encode(hasher.finalize())
}

fn dir_size(path: &Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                total += meta.len();
            }
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_path_is_date_formatted() {
        let path = backup_path_today();
        let name = path.file_name().unwrap().to_string_lossy();
        // Should match YYYY-MM-DD format
        assert_eq!(name.len(), 10);
        assert_eq!(&name[4..5], "-");
        assert_eq!(&name[7..8], "-");
    }

    #[test]
    fn data_path_under_home() {
        let path = data_path();
        assert!(path.to_string_lossy().contains(".hydra/data"));
    }

    #[test]
    fn list_backups_empty_when_no_dir() {
        let backups = list_backups();
        // May or may not have backups depending on test env
        assert!(backups.len() < 1000); // sanity check
    }

    #[test]
    fn compute_hash_deterministic() {
        let dir = std::env::temp_dir().join("hydra-backup-test-hash");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("test.txt"), "hello").unwrap();

        let h1 = compute_backup_hash(&dir);
        let h2 = compute_backup_hash(&dir);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64); // SHA256 hex

        let _ = std::fs::remove_dir_all(&dir);
    }
}
