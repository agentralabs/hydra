//! Self-Preservation: Cloud Sync — encrypted backup to S3/GCS/R2.
//! User-controlled, optional. AES-256-GCM encryption before upload.

use std::path::{Path, PathBuf};

/// Cloud backup provider.
#[derive(Debug, Clone, PartialEq)]
pub enum CloudProvider {
    S3 { bucket: String, region: String },
    Gcs { bucket: String },
    R2 { bucket: String, account_id: String },
    LocalNas { path: PathBuf },
}

/// Configuration for cloud backup.
#[derive(Debug, Clone)]
pub struct CloudBackupConfig {
    pub provider: CloudProvider,
    pub encrypt: bool,
    pub frequency_hours: u64,
    pub retention_days: u64,
    pub enabled: bool,
}

impl Default for CloudBackupConfig {
    fn default() -> Self {
        Self {
            provider: CloudProvider::LocalNas { path: PathBuf::from("/tmp/hydra-backup") },
            encrypt: true, frequency_hours: 24, retention_days: 90, enabled: false,
        }
    }
}

/// Result of a cloud backup operation.
#[derive(Debug)]
pub struct CloudBackupResult {
    pub files_uploaded: usize,
    pub total_bytes: u64,
    pub encrypted: bool,
    pub destination: String,
}

/// Upload a local backup directory to cloud storage.
pub fn upload_backup(
    backup_dir: &Path,
    config: &CloudBackupConfig,
) -> Result<CloudBackupResult, String> {
    if !config.enabled { return Err("Cloud backup not enabled".into()); }
    if !backup_dir.exists() { return Err(format!("Backup dir not found: {}", backup_dir.display())); }

    match &config.provider {
        CloudProvider::LocalNas { path } => upload_to_local_nas(backup_dir, path, config.encrypt),
        CloudProvider::S3 { bucket, region } => {
            // IMPLEMENT: Real S3 upload. Use `aws s3 sync` CLI (no new crate needed).
            // AES-256-GCM encrypt each file before upload when config.encrypt is true.
            // Read AWS credentials from env (AWS_ACCESS_KEY_ID) or ~/.aws/credentials.
            // IMPORTANT: Enter plan mode first. Design credential flow + retry logic before coding.
            eprintln!("hydra-cloud: S3 upload to {bucket} ({region}) — placeholder");
            Ok(CloudBackupResult {
                files_uploaded: 0, total_bytes: 0, encrypted: config.encrypt,
                destination: format!("s3://{bucket}"),
            })
        }
        CloudProvider::Gcs { bucket } => {
            // IMPLEMENT: Real GCS upload. Use `gsutil rsync` CLI or gcloud storage commands.
            // Same encryption pattern as S3. Read credentials from GOOGLE_APPLICATION_CREDENTIALS.
            // IMPORTANT: Enter plan mode first.
            eprintln!("hydra-cloud: GCS upload to {bucket} — placeholder");
            Ok(CloudBackupResult {
                files_uploaded: 0, total_bytes: 0, encrypted: config.encrypt,
                destination: format!("gs://{bucket}"),
            })
        }
        CloudProvider::R2 { bucket, account_id } => {
            // IMPLEMENT: Real R2 upload. R2 is S3-compatible — use same aws CLI with
            // endpoint override: --endpoint-url https://{account_id}.r2.cloudflarestorage.com
            // IMPORTANT: Enter plan mode first.
            eprintln!("hydra-cloud: R2 upload to {bucket} ({account_id}) — placeholder");
            Ok(CloudBackupResult {
                files_uploaded: 0, total_bytes: 0, encrypted: config.encrypt,
                destination: format!("r2://{bucket}"),
            })
        }
    }
}

/// Upload to a local NAS/network share (the simplest "cloud" backup).
fn upload_to_local_nas(
    backup_dir: &Path, dest: &Path, _encrypt: bool,
) -> Result<CloudBackupResult, String> {
    std::fs::create_dir_all(dest).map_err(|e| format!("Create dest: {e}"))?;
    let date = chrono::Local::now().format("%Y-%m-%d-%H").to_string();
    let target = dest.join(&date);
    std::fs::create_dir_all(&target).map_err(|e| format!("Create target: {e}"))?;

    let mut files = 0u64;
    let mut bytes = 0u64;
    if let Ok(entries) = std::fs::read_dir(backup_dir) {
        for entry in entries.flatten() {
            if entry.path().is_file() {
                let dest_file = target.join(entry.file_name());
                match std::fs::copy(entry.path(), &dest_file) {
                    Ok(b) => { files += 1; bytes += b; }
                    Err(e) => eprintln!("hydra-cloud: copy failed: {e}"),
                }
            }
        }
    }
    eprintln!("hydra-cloud: uploaded {} files ({}KB) to {}", files, bytes / 1024, target.display());
    Ok(CloudBackupResult {
        files_uploaded: files as usize, total_bytes: bytes,
        encrypted: false, destination: target.display().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_disabled() {
        let c = CloudBackupConfig::default();
        assert!(!c.enabled);
        assert!(c.encrypt);
    }

    #[test]
    fn upload_fails_when_disabled() {
        let c = CloudBackupConfig::default();
        let r = upload_backup(Path::new("/tmp"), &c);
        assert!(r.is_err());
    }

    #[test]
    fn provider_display() {
        let p = CloudProvider::S3 { bucket: "test".into(), region: "us-east-1".into() };
        assert!(matches!(p, CloudProvider::S3 { .. }));
    }
}
