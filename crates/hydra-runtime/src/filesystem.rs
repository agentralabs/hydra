use std::path::Path;

/// Required subdirectories under HYDRA_DATA_DIR
const SUBDIRS: &[&str] = &["receipts", "evidence", "cache", "logs", "voice"];

/// Initialize the Hydra filesystem structure.
/// Creates the data directory and all subdirectories.
/// Idempotent: safe to call multiple times.
pub fn init_filesystem(data_dir: &Path) -> Result<(), FilesystemError> {
    // Create root
    std::fs::create_dir_all(data_dir)
        .map_err(|e| FilesystemError::CreateDir(data_dir.display().to_string(), e.to_string()))?;

    // Create subdirectories
    for dir in SUBDIRS {
        let path = data_dir.join(dir);
        std::fs::create_dir_all(&path)
            .map_err(|e| FilesystemError::CreateDir(path.display().to_string(), e.to_string()))?;
    }

    // Set permissions on Unix
    #[cfg(unix)]
    set_unix_permissions(data_dir)?;

    Ok(())
}

#[cfg(unix)]
fn set_unix_permissions(data_dir: &Path) -> Result<(), FilesystemError> {
    use std::os::unix::fs::PermissionsExt;

    let perms = std::fs::Permissions::from_mode(0o700);
    std::fs::set_permissions(data_dir, perms.clone())
        .map_err(|e| FilesystemError::Permissions(data_dir.display().to_string(), e.to_string()))?;

    for dir in SUBDIRS {
        let path = data_dir.join(dir);
        std::fs::set_permissions(&path, perms.clone())
            .map_err(|e| FilesystemError::Permissions(path.display().to_string(), e.to_string()))?;
    }

    Ok(())
}

/// Verify that the filesystem is properly initialized
pub fn verify_filesystem(data_dir: &Path) -> bool {
    if !data_dir.is_dir() {
        return false;
    }
    SUBDIRS.iter().all(|d| data_dir.join(d).is_dir())
}

#[derive(Debug, Clone)]
pub enum FilesystemError {
    CreateDir(String, String),
    Permissions(String, String),
}

impl std::fmt::Display for FilesystemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CreateDir(path, err) => write!(
                f,
                "Cannot create directory '{path}'. {err}. Check permissions."
            ),
            Self::Permissions(path, err) => write!(f, "Cannot set permissions on '{path}'. {err}."),
        }
    }
}

impl std::error::Error for FilesystemError {}
