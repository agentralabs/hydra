//! Auto-update system — version check and update management.

use serde::{Deserialize, Serialize};

/// Current app version (from Cargo.toml).
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Update availability status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateStatus {
    Unknown,
    UpToDate,
    Available { version: String, changelog: String, download_url: String },
    Downloading { version: String, progress_percent: u8 },
    Ready { version: String },
    Skipped { version: String },
    Error(String),
}

/// The update manager state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateManager {
    pub status: UpdateStatus,
    pub last_check: Option<String>,
    pub skipped_versions: Vec<String>,
    pub check_url: String,
    pub auto_check: bool,
}

impl UpdateManager {
    /// Create a new update manager.
    pub fn new() -> Self {
        Self {
            status: UpdateStatus::Unknown,
            last_check: None,
            skipped_versions: Vec::new(),
            check_url: "https://api.github.com/repos/agentralabs/agentic-hydra/releases/latest".into(),
            auto_check: true,
        }
    }

    /// Check if an update is available (async — called from runtime).
    /// Returns the parsed release info or None.
    pub async fn check_for_update(&mut self) -> Option<ReleaseInfo> {
        self.last_check = Some(chrono::Utc::now().to_rfc3339());

        // HTTP fetch would go here — using reqwest or hyper
        // For now, simulate the check
        let release = fetch_latest_release(&self.check_url).await;

        match release {
            Some(info) => {
                if self.skipped_versions.contains(&info.version) {
                    self.status = UpdateStatus::Skipped { version: info.version.clone() };
                    return None;
                }
                if version_is_newer(&info.version, CURRENT_VERSION) {
                    self.status = UpdateStatus::Available {
                        version: info.version.clone(),
                        changelog: info.changelog.clone(),
                        download_url: info.download_url.clone(),
                    };
                    Some(info)
                } else {
                    self.status = UpdateStatus::UpToDate;
                    None
                }
            }
            None => {
                self.status = UpdateStatus::Error("Failed to check for updates".into());
                None
            }
        }
    }

    /// Skip this version.
    pub fn skip_version(&mut self, version: &str) {
        if !self.skipped_versions.contains(&version.to_string()) {
            self.skipped_versions.push(version.to_string());
        }
        self.status = UpdateStatus::Skipped { version: version.into() };
    }

    /// Dismiss the update banner.
    pub fn dismiss(&mut self) {
        self.status = UpdateStatus::UpToDate;
    }

    /// Check if update banner should be shown.
    pub fn should_show_banner(&self) -> bool {
        matches!(self.status, UpdateStatus::Available { .. } | UpdateStatus::Ready { .. })
    }

    /// Get the available version string, if any.
    pub fn available_version(&self) -> Option<&str> {
        match &self.status {
            UpdateStatus::Available { version, .. }
            | UpdateStatus::Downloading { version, .. }
            | UpdateStatus::Ready { version, .. } => Some(version),
            _ => None,
        }
    }
}

impl Default for UpdateManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Release information from the update server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseInfo {
    pub version: String,
    pub changelog: String,
    pub download_url: String,
    pub published_at: String,
}

/// Compare two semver strings. Returns true if `new_ver` > `current`.
pub fn version_is_newer(new_ver: &str, current: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> {
        v.trim_start_matches('v')
            .split('.')
            .filter_map(|s| s.parse().ok())
            .collect()
    };
    let new_parts = parse(new_ver);
    let cur_parts = parse(current);

    for i in 0..3 {
        let n = new_parts.get(i).copied().unwrap_or(0);
        let c = cur_parts.get(i).copied().unwrap_or(0);
        if n > c { return true; }
        if n < c { return false; }
    }
    false
}

/// Fetch the latest release from the update URL.
/// In production, this would use reqwest/hyper. For now returns None.
async fn fetch_latest_release(_url: &str) -> Option<ReleaseInfo> {
    // Real implementation would:
    // 1. HTTP GET to the releases API
    // 2. Parse JSON response
    // 3. Return ReleaseInfo
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_is_newer() {
        assert!(version_is_newer("1.1.0", "1.0.0"));
        assert!(version_is_newer("2.0.0", "1.9.9"));
        assert!(version_is_newer("1.0.1", "1.0.0"));
        assert!(!version_is_newer("1.0.0", "1.0.0"));
        assert!(!version_is_newer("0.9.0", "1.0.0"));
        assert!(version_is_newer("v1.1.0", "v1.0.0"));
    }

    #[test]
    fn test_update_manager_creation() {
        let mgr = UpdateManager::new();
        assert_eq!(mgr.status, UpdateStatus::Unknown);
        assert!(mgr.auto_check);
        assert!(mgr.skipped_versions.is_empty());
    }

    #[test]
    fn test_skip_version() {
        let mut mgr = UpdateManager::new();
        mgr.skip_version("1.2.0");
        assert!(mgr.skipped_versions.contains(&"1.2.0".to_string()));
        assert!(matches!(mgr.status, UpdateStatus::Skipped { .. }));
    }

    #[test]
    fn test_should_show_banner() {
        let mut mgr = UpdateManager::new();
        assert!(!mgr.should_show_banner());
        mgr.status = UpdateStatus::Available {
            version: "1.2.0".into(),
            changelog: "New features".into(),
            download_url: "https://example.com".into(),
        };
        assert!(mgr.should_show_banner());
    }

    #[test]
    fn test_dismiss() {
        let mut mgr = UpdateManager::new();
        mgr.status = UpdateStatus::Available {
            version: "1.2.0".into(),
            changelog: "".into(),
            download_url: "".into(),
        };
        mgr.dismiss();
        assert_eq!(mgr.status, UpdateStatus::UpToDate);
        assert!(!mgr.should_show_banner());
    }

    #[test]
    fn test_available_version() {
        let mut mgr = UpdateManager::new();
        assert_eq!(mgr.available_version(), None);
        mgr.status = UpdateStatus::Available {
            version: "2.0.0".into(),
            changelog: "".into(),
            download_url: "".into(),
        };
        assert_eq!(mgr.available_version(), Some("2.0.0"));
    }

    #[test]
    fn test_current_version_exists() {
        assert!(!CURRENT_VERSION.is_empty());
    }
}
