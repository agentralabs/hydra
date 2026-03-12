//! Project isolation via canonical path hashing.

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Isolates per-project data using a hash of the canonical project path.
#[derive(Debug, Clone)]
pub struct ProjectIsolation {
    canonical_path: PathBuf,
    hash: String,
}

impl ProjectIsolation {
    /// Create a new ProjectIsolation from a project root path.
    /// Canonicalizes the path to resolve symlinks and relative components.
    pub fn new(project_root: &Path) -> Self {
        let canonical_path = project_root
            .canonicalize()
            .unwrap_or_else(|_| project_root.to_path_buf());

        let mut hasher = Sha256::new();
        hasher.update(canonical_path.to_string_lossy().as_bytes());
        let result = hasher.finalize();
        let hash = hex_encode(&result)[..12].to_string();

        Self {
            canonical_path,
            hash,
        }
    }

    /// SHA256 of canonical path, first 12 hex chars.
    pub fn project_hash(&self) -> String {
        self.hash.clone()
    }

    /// Per-project data directory: ~/.hydra/projects/{hash}/
    pub fn data_dir(&self) -> PathBuf {
        let home = home_dir();
        home.join(".hydra").join("projects").join(&self.hash)
    }

    /// Lock file path within the project data directory.
    pub fn lock_path(&self) -> PathBuf {
        self.data_dir().join("hydra.lock")
    }

    /// Check if another path refers to the same project (same canonical path).
    pub fn is_same_project(&self, other: &Path) -> bool {
        let other_canonical = other
            .canonicalize()
            .unwrap_or_else(|_| other.to_path_buf());
        self.canonical_path == other_canonical
    }
}

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_encode_empty() {
        assert_eq!(hex_encode(&[]), "");
    }

    #[test]
    fn test_hex_encode_bytes() {
        assert_eq!(hex_encode(&[0x00, 0xff, 0xab]), "00ffab");
    }

    #[test]
    fn test_project_isolation_hash_length() {
        let iso = ProjectIsolation::new(Path::new("/tmp"));
        let hash = iso.project_hash();
        assert_eq!(hash.len(), 12);
    }

    #[test]
    fn test_project_isolation_hash_deterministic() {
        let iso1 = ProjectIsolation::new(Path::new("/tmp"));
        let iso2 = ProjectIsolation::new(Path::new("/tmp"));
        assert_eq!(iso1.project_hash(), iso2.project_hash());
    }

    #[test]
    fn test_project_isolation_different_paths_different_hashes() {
        let iso1 = ProjectIsolation::new(Path::new("/tmp/project_a_unique_12345"));
        let iso2 = ProjectIsolation::new(Path::new("/tmp/project_b_unique_67890"));
        assert_ne!(iso1.project_hash(), iso2.project_hash());
    }

    #[test]
    fn test_data_dir_contains_hash() {
        let iso = ProjectIsolation::new(Path::new("/tmp"));
        let data_dir = iso.data_dir();
        let hash = iso.project_hash();
        assert!(data_dir.to_string_lossy().contains(&hash));
    }

    #[test]
    fn test_data_dir_under_hydra() {
        let iso = ProjectIsolation::new(Path::new("/tmp"));
        let data_dir = iso.data_dir();
        assert!(data_dir.to_string_lossy().contains(".hydra/projects/"));
    }

    #[test]
    fn test_lock_path_in_data_dir() {
        let iso = ProjectIsolation::new(Path::new("/tmp"));
        let lock = iso.lock_path();
        let data = iso.data_dir();
        assert_eq!(lock, data.join("hydra.lock"));
    }

    #[test]
    fn test_is_same_project_same_path() {
        let iso = ProjectIsolation::new(Path::new("/tmp"));
        assert!(iso.is_same_project(Path::new("/tmp")));
    }

    #[test]
    fn test_is_same_project_different_path() {
        let iso = ProjectIsolation::new(Path::new("/tmp"));
        assert!(!iso.is_same_project(Path::new("/var")));
    }

    #[test]
    fn test_clone() {
        let iso = ProjectIsolation::new(Path::new("/tmp"));
        let cloned = iso.clone();
        assert_eq!(iso.project_hash(), cloned.project_hash());
    }

    #[test]
    fn test_debug() {
        let iso = ProjectIsolation::new(Path::new("/tmp"));
        let debug = format!("{:?}", iso);
        assert!(debug.contains("ProjectIsolation"));
    }
}
