//! SkillRequirements — parsed from environment.toml in a skill package.
//! This is what hydra-skills passes here when a skill loads.

use serde::{Deserialize, Serialize};

/// One required binary from environment.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequiredBinary {
    /// Binary name.
    pub name: String,
    /// Whether this binary is strictly required.
    pub required: bool,
    /// Hint for how to install the binary.
    pub install_hint: String,
    /// Optional fallback binary that can be used instead.
    pub fallback: Option<String>,
}

/// The skill's runtime requirements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRequirements {
    /// The name of the skill these requirements apply to.
    pub skill_name: String,
    /// Minimum RAM in megabytes.
    pub min_ram_mb: u64,
    /// Minimum disk space in megabytes.
    pub min_disk_mb: u64,
    /// Whether a GPU is preferred but not required.
    pub gpu_preferred: bool,
    /// Whether a GPU is strictly required.
    pub gpu_required: bool,
    /// Required and optional binaries.
    pub binaries: Vec<RequiredBinary>,
    /// Actions that can run in low-resource mode.
    pub low_resource_actions: Vec<String>,
    /// RAM threshold below which the skill enters degraded mode.
    pub low_resource_threshold_mb: u64,
}

impl SkillRequirements {
    /// Create new requirements for a skill with sensible defaults.
    pub fn new(skill_name: impl Into<String>) -> Self {
        Self {
            skill_name: skill_name.into(),
            min_ram_mb: crate::constants::MIN_RAM_MB_FULL,
            min_disk_mb: crate::constants::MIN_DISK_MB,
            gpu_preferred: false,
            gpu_required: false,
            binaries: Vec::new(),
            low_resource_actions: Vec::new(),
            low_resource_threshold_mb: crate::constants::MIN_RAM_MB_DEGRADED,
        }
    }

    /// Return only the required binaries (not optional).
    pub fn required_binaries(&self) -> Vec<&RequiredBinary> {
        self.binaries.iter().filter(|b| b.required).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_binaries_filtered() {
        let mut r = SkillRequirements::new("video-editor");
        r.binaries.push(RequiredBinary {
            name: "ffmpeg".into(),
            required: true,
            install_hint: "brew install ffmpeg".into(),
            fallback: Some("handbrake".into()),
        });
        r.binaries.push(RequiredBinary {
            name: "ffprobe".into(),
            required: false,
            install_hint: "ships with ffmpeg".into(),
            fallback: None,
        });
        assert_eq!(r.required_binaries().len(), 1);
        assert_eq!(r.required_binaries()[0].name, "ffmpeg");
    }
}
