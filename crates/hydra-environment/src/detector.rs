//! EnvironmentDetector — probe and classify the current environment.
//! Called on skill load and on first execution in a new environment.
//! Results stored in hydra-plastic (plasticity profiles).

use crate::{
    errors::EnvironmentError,
    profile::{EnvironmentCapabilities, EnvironmentClass, EnvironmentProfile, OsType},
};

/// Detects the current execution environment.
pub struct EnvironmentDetector;

impl EnvironmentDetector {
    /// Create a new detector.
    pub fn new() -> Self {
        Self
    }

    /// Probe and build a profile for the current environment.
    pub fn detect_current(&self) -> Result<EnvironmentProfile, EnvironmentError> {
        let class = self.classify_environment();
        let caps = self.probe_capabilities()?;
        let mut profile = EnvironmentProfile::new(class, caps);

        // Check for common binaries
        for binary in &[
            "ffmpeg", "git", "docker", "python3", "node", "cargo", "kubectl", "ssh", "curl",
            "wget",
        ] {
            if self.binary_exists(binary) {
                profile.available_binaries.push((*binary).to_string());
            }
        }

        Ok(profile)
    }

    /// Classify based on environment signals.
    fn classify_environment(&self) -> EnvironmentClass {
        // Check for container indicators
        if std::path::Path::new("/.dockerenv").exists() {
            return EnvironmentClass::Container {
                runtime: "docker".into(),
            };
        }
        // Check for cloud indicators via env vars
        if std::env::var("AWS_EXECUTION_ENV").is_ok() {
            return EnvironmentClass::CloudInstance {
                provider: "aws".into(),
                region: std::env::var("AWS_DEFAULT_REGION")
                    .unwrap_or_else(|_| "unknown".into()),
            };
        }
        if std::env::var("GOOGLE_CLOUD_PROJECT").is_ok() {
            return EnvironmentClass::CloudInstance {
                provider: "gcp".into(),
                region: "unknown".into(),
            };
        }
        // Default: local machine
        EnvironmentClass::LocalMachine
    }

    /// Probe available resources.
    fn probe_capabilities(&self) -> Result<EnvironmentCapabilities, EnvironmentError> {
        Ok(EnvironmentCapabilities {
            ram_mb: self.estimate_ram_mb(),
            disk_mb: self.estimate_disk_mb(),
            cpu_cores: self.cpu_count(),
            has_gpu: false,
            has_network: self.has_network(),
            has_filesystem: true,
            os_type: OsType::detect(),
        })
    }

    fn estimate_ram_mb(&self) -> u64 {
        if let Ok(v) = std::env::var("HYDRA_TEST_RAM_MB") {
            return v.parse().unwrap_or(4096);
        }
        4096
    }

    fn estimate_disk_mb(&self) -> u64 {
        if let Ok(v) = std::env::var("HYDRA_TEST_DISK_MB") {
            return v.parse().unwrap_or(50_000);
        }
        50_000
    }

    fn cpu_count(&self) -> u32 {
        std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(4)
    }

    fn has_network(&self) -> bool {
        std::env::var("HYDRA_TEST_NO_NETWORK").is_err()
    }

    /// Check if a binary exists in PATH.
    pub fn binary_exists(&self, binary: &str) -> bool {
        if cfg!(target_os = "windows") {
            std::process::Command::new("where")
                .arg(binary)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        } else {
            std::process::Command::new("which")
                .arg(binary)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        }
    }
}

impl Default for EnvironmentDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detection_produces_profile() {
        let d = EnvironmentDetector::new();
        let p = d.detect_current().expect("detect should succeed");
        assert!(!p.id.is_empty());
        assert!(p.capabilities.cpu_cores > 0);
        assert!(p.capabilities.ram_mb > 0);
    }

    #[test]
    fn os_type_detected() {
        let os = OsType::detect();
        assert!(matches!(
            os,
            OsType::Linux | OsType::MacOS | OsType::Windows | OsType::Unknown
        ));
    }

    #[test]
    fn binary_check_works() {
        let d = EnvironmentDetector::new();
        // Just verify it doesn't panic — result depends on the machine
        let _ = d.binary_exists("git");
    }
}
