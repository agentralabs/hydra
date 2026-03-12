use std::path::{Path, PathBuf};

use super::user::UserProfile;

/// Errors that can occur during profile I/O
#[derive(Debug)]
pub enum ProfileStorageError {
    /// Failed to create directories
    DirCreate(std::io::Error),
    /// Failed to read file
    Read(std::io::Error),
    /// Failed to write file
    Write(std::io::Error),
    /// Failed to parse JSON
    Parse(serde_json::Error),
    /// Failed to serialize JSON
    Serialize(serde_json::Error),
}

impl std::fmt::Display for ProfileStorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DirCreate(e) => write!(f, "failed to create profile directory: {}", e),
            Self::Read(e) => write!(f, "failed to read profile: {}", e),
            Self::Write(e) => write!(f, "failed to write profile: {}", e),
            Self::Parse(e) => write!(f, "failed to parse profile JSON: {}", e),
            Self::Serialize(e) => write!(f, "failed to serialize profile: {}", e),
        }
    }
}

impl std::error::Error for ProfileStorageError {}

/// Handles loading and saving `UserProfile` to disk
pub struct ProfileStorage;

impl ProfileStorage {
    /// Default profile path: ~/.hydra/profile.json
    pub fn default_path() -> PathBuf {
        std::env::var("HOME")
            .map(|h| PathBuf::from(h).join(".hydra").join("profile.json"))
            .unwrap_or_else(|_| PathBuf::from("/tmp/.hydra/profile.json"))
    }

    /// Load from the default path, returning a fresh profile if not found
    pub fn load_default() -> Result<UserProfile, ProfileStorageError> {
        let path = Self::default_path();
        Self::load(&path)
    }

    /// Load from a specific path, returning a fresh profile if not found
    pub fn load(path: &Path) -> Result<UserProfile, ProfileStorageError> {
        match std::fs::read_to_string(path) {
            Ok(contents) => {
                let profile: UserProfile =
                    serde_json::from_str(&contents).map_err(ProfileStorageError::Parse)?;
                Ok(profile)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(UserProfile::default()),
            Err(e) => Err(ProfileStorageError::Read(e)),
        }
    }

    /// Save to the default path
    pub fn save_default(profile: &UserProfile) -> Result<(), ProfileStorageError> {
        let path = Self::default_path();
        Self::save(&path, profile)
    }

    /// Save to a specific path, creating parent directories if needed
    pub fn save(path: &Path, profile: &UserProfile) -> Result<(), ProfileStorageError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(ProfileStorageError::DirCreate)?;
        }
        let json =
            serde_json::to_string_pretty(profile).map_err(ProfileStorageError::Serialize)?;
        std::fs::write(path, json).map_err(ProfileStorageError::Write)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_path_not_empty() {
        let path = ProfileStorage::default_path();
        assert!(path.to_string_lossy().contains("profile.json"));
    }

    #[test]
    fn test_default_path_under_hydra() {
        let path = ProfileStorage::default_path();
        assert!(path.to_string_lossy().contains(".hydra"));
    }

    #[test]
    fn test_load_nonexistent_returns_default() {
        let path = Path::new("/tmp/hydra_storage_test_nonexistent_xyz.json");
        let profile = ProfileStorage::load(path).unwrap();
        assert!(profile.name.is_none());
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let dir = std::env::temp_dir().join(format!("hydra_storage_test_{}", std::process::id()));
        let path = dir.join("profile.json");
        let mut profile = UserProfile::default();
        profile.name = Some("StorageTest".into());
        ProfileStorage::save(&path, &profile).unwrap();

        let loaded = ProfileStorage::load(&path).unwrap();
        assert_eq!(loaded.name, Some("StorageTest".into()));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_save_creates_parent_dirs() {
        let dir = std::env::temp_dir().join(format!("hydra_storage_nested_{}/sub/dir", std::process::id()));
        let path = dir.join("profile.json");
        let profile = UserProfile::default();
        ProfileStorage::save(&path, &profile).unwrap();
        assert!(path.exists());
        let _ = std::fs::remove_dir_all(std::env::temp_dir().join(format!("hydra_storage_nested_{}", std::process::id())));
    }

    #[test]
    fn test_error_display_dir_create() {
        let err = ProfileStorageError::DirCreate(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "no access"));
        let msg = format!("{}", err);
        assert!(msg.contains("profile directory"));
    }

    #[test]
    fn test_error_display_read() {
        let err = ProfileStorageError::Read(std::io::Error::new(std::io::ErrorKind::NotFound, "gone"));
        let msg = format!("{}", err);
        assert!(msg.contains("read profile"));
    }

    #[test]
    fn test_error_display_write() {
        let err = ProfileStorageError::Write(std::io::Error::new(std::io::ErrorKind::Other, "disk full"));
        let msg = format!("{}", err);
        assert!(msg.contains("write profile"));
    }

    #[test]
    fn test_load_invalid_json() {
        let dir = std::env::temp_dir().join(format!("hydra_storage_invalid_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("bad.json");
        std::fs::write(&path, "not json at all").unwrap();
        let result = ProfileStorage::load(&path);
        assert!(result.is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
