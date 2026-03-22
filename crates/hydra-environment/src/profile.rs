//! EnvironmentProfile — the detected environment for one execution context.
//! Built by EnvironmentDetector. Used by hydra-executor.
//! Stored in hydra-plastic as a plasticity profile (growth invariant).

use serde::{Deserialize, Serialize};

/// The class of execution environment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EnvironmentClass {
    /// Local machine — full capabilities.
    LocalMachine,
    /// Remote server via SSH.
    RemoteSSH { host: String },
    /// Container (Docker, Podman).
    Container { runtime: String },
    /// Cloud compute instance.
    CloudInstance { provider: String, region: String },
    /// Embedded or resource-constrained device.
    Embedded { platform: String },
    /// Distributed — multiple nodes.
    Distributed { node_count: usize },
    /// Browser sandbox (WASM).
    BrowserSandbox,
    /// Unknown — probe and classify.
    Unknown,
}

impl EnvironmentClass {
    /// Return a human-readable label for this environment class.
    pub fn label(&self) -> String {
        match self {
            Self::LocalMachine => "local".into(),
            Self::RemoteSSH { host } => format!("ssh:{host}"),
            Self::Container { runtime } => format!("container:{runtime}"),
            Self::CloudInstance { provider, .. } => format!("cloud:{provider}"),
            Self::Embedded { platform } => format!("embedded:{platform}"),
            Self::Distributed { .. } => "distributed".into(),
            Self::BrowserSandbox => "browser".into(),
            Self::Unknown => "unknown".into(),
        }
    }

    /// Return true if this environment class is resource-constrained.
    pub fn is_resource_constrained(&self) -> bool {
        matches!(self, Self::Embedded { .. } | Self::BrowserSandbox)
    }
}

/// Detected capabilities of the current environment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentCapabilities {
    /// Available RAM in megabytes.
    pub ram_mb: u64,
    /// Available disk space in megabytes.
    pub disk_mb: u64,
    /// Number of CPU cores.
    pub cpu_cores: u32,
    /// Whether a GPU is available.
    pub has_gpu: bool,
    /// Whether network access is available.
    pub has_network: bool,
    /// Whether filesystem access is available.
    pub has_filesystem: bool,
    /// The detected operating system.
    pub os_type: OsType,
}

/// The operating system type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OsType {
    /// Linux-based OS.
    Linux,
    /// macOS.
    MacOS,
    /// Windows.
    Windows,
    /// Could not determine the OS.
    Unknown,
}

impl OsType {
    /// Detect the current operating system at compile time.
    pub fn detect() -> Self {
        if cfg!(target_os = "linux") {
            Self::Linux
        } else if cfg!(target_os = "macos") {
            Self::MacOS
        } else if cfg!(target_os = "windows") {
            Self::Windows
        } else {
            Self::Unknown
        }
    }

    /// Return a human-readable label for this OS type.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Linux => "linux",
            Self::MacOS => "macos",
            Self::Windows => "windows",
            Self::Unknown => "unknown",
        }
    }
}

/// The full detected environment profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentProfile {
    /// Unique identifier for this profile.
    pub id: String,
    /// The class of environment detected.
    pub class: EnvironmentClass,
    /// The capabilities of this environment.
    pub capabilities: EnvironmentCapabilities,
    /// Binaries available on the system PATH.
    pub available_binaries: Vec<String>,
    /// When the profile was created.
    pub profiled_at: chrono::DateTime<chrono::Utc>,
}

impl EnvironmentProfile {
    /// Create a new profile with the given class and capabilities.
    pub fn new(class: EnvironmentClass, capabilities: EnvironmentCapabilities) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            class,
            capabilities,
            available_binaries: Vec::new(),
            profiled_at: chrono::Utc::now(),
        }
    }

    /// Return true if the environment can support full execution.
    pub fn is_capable_of_full_execution(&self) -> bool {
        self.capabilities.ram_mb >= crate::constants::MIN_RAM_MB_FULL
            && self.capabilities.has_filesystem
    }

    /// Return true if the environment can support degraded execution.
    pub fn is_capable_of_degraded_execution(&self) -> bool {
        self.capabilities.ram_mb >= crate::constants::MIN_RAM_MB_DEGRADED
    }

    /// Return true if a binary is in the available binaries list.
    pub fn has_binary(&self, binary: &str) -> bool {
        self.available_binaries.iter().any(|b| b == binary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capable_profile() -> EnvironmentProfile {
        let mut p = EnvironmentProfile::new(
            EnvironmentClass::LocalMachine,
            EnvironmentCapabilities {
                ram_mb: 8192,
                disk_mb: 100_000,
                cpu_cores: 8,
                has_gpu: false,
                has_network: true,
                has_filesystem: true,
                os_type: OsType::detect(),
            },
        );
        p.available_binaries.push("ffmpeg".into());
        p
    }

    #[test]
    fn capable_profile_full_execution() {
        let p = capable_profile();
        assert!(p.is_capable_of_full_execution());
    }

    #[test]
    fn low_ram_only_degraded() {
        let p = EnvironmentProfile::new(
            EnvironmentClass::Embedded {
                platform: "rpi".into(),
            },
            EnvironmentCapabilities {
                ram_mb: 200,
                disk_mb: 4_000,
                cpu_cores: 4,
                has_gpu: false,
                has_network: true,
                has_filesystem: true,
                os_type: OsType::Linux,
            },
        );
        assert!(!p.is_capable_of_full_execution());
        assert!(p.is_capable_of_degraded_execution());
    }

    #[test]
    fn binary_detection() {
        let p = capable_profile();
        assert!(p.has_binary("ffmpeg"));
        assert!(!p.has_binary("handbrake"));
    }

    #[test]
    fn class_labels() {
        assert_eq!(EnvironmentClass::LocalMachine.label(), "local");
        assert_eq!(
            EnvironmentClass::Container {
                runtime: "docker".into()
            }
            .label(),
            "container:docker"
        );
    }
}
