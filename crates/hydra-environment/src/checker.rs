//! RequirementsChecker — can we run this skill in this environment?
//! Produces a CheckOutcome — full, degraded, or blocked.

use crate::{
    profile::EnvironmentProfile,
    requirements::SkillRequirements,
};
use serde::{Deserialize, Serialize};

/// The result of checking requirements against an environment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CheckOutcome {
    /// All requirements met — full execution possible.
    FullCapability,
    /// Some optional requirements missing — degraded but functional.
    DegradedCapability {
        /// Names of missing optional capabilities.
        missing: Vec<String>,
    },
    /// Required resources missing — cannot execute.
    Blocked {
        /// Why the skill is blocked.
        reason: String,
    },
}

impl CheckOutcome {
    /// Return true if the skill can execute (fully or degraded).
    pub fn can_execute(&self) -> bool {
        !matches!(self, Self::Blocked { .. })
    }

    /// Return a human-readable label for TUI display.
    pub fn label(&self) -> &'static str {
        match self {
            Self::FullCapability => "full",
            Self::DegradedCapability { .. } => "degraded",
            Self::Blocked { .. } => "blocked",
        }
    }
}

/// Check skill requirements against the current environment.
pub fn check_requirements(
    reqs: &SkillRequirements,
    profile: &EnvironmentProfile,
) -> CheckOutcome {
    let mut missing_required = Vec::new();
    let mut missing_optional = Vec::new();

    // RAM check
    if profile.capabilities.ram_mb < reqs.min_ram_mb {
        if profile.capabilities.ram_mb < reqs.low_resource_threshold_mb {
            return CheckOutcome::Blocked {
                reason: format!(
                    "Insufficient RAM: {}MB available, {}MB required",
                    profile.capabilities.ram_mb, reqs.min_ram_mb
                ),
            };
        }
        missing_optional.push(format!(
            "Low RAM: {}MB (prefer {}MB)",
            profile.capabilities.ram_mb, reqs.min_ram_mb
        ));
    }

    // GPU check
    if reqs.gpu_required && !profile.capabilities.has_gpu {
        return CheckOutcome::Blocked {
            reason: "GPU required but not available".into(),
        };
    }

    // Binary checks
    for binary in &reqs.binaries {
        let found = profile.has_binary(&binary.name);
        let fallback_found = binary
            .fallback
            .as_ref()
            .map(|f| profile.has_binary(f))
            .unwrap_or(false);

        if !found && !fallback_found {
            if binary.required {
                missing_required.push(format!("{} ({})", binary.name, binary.install_hint));
            } else {
                missing_optional.push(binary.name.clone());
            }
        }
    }

    // Required binaries missing -> blocked
    if !missing_required.is_empty() {
        return CheckOutcome::Blocked {
            reason: format!(
                "Required binaries not found: {}",
                missing_required.join(", ")
            ),
        };
    }

    // Optional things missing -> degraded
    if !missing_optional.is_empty() {
        return CheckOutcome::DegradedCapability {
            missing: missing_optional,
        };
    }

    CheckOutcome::FullCapability
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::{EnvironmentCapabilities, EnvironmentClass, EnvironmentProfile, OsType};
    use crate::requirements::RequiredBinary;

    fn profile_with_binary(binary: &str, ram_mb: u64) -> EnvironmentProfile {
        let mut p = EnvironmentProfile::new(
            EnvironmentClass::LocalMachine,
            EnvironmentCapabilities {
                ram_mb,
                disk_mb: 100_000,
                cpu_cores: 4,
                has_gpu: false,
                has_network: true,
                has_filesystem: true,
                os_type: OsType::detect(),
            },
        );
        if !binary.is_empty() {
            p.available_binaries.push(binary.to_string());
        }
        p
    }

    #[test]
    fn all_met_full_capability() {
        let mut reqs = SkillRequirements::new("test-skill");
        reqs.min_ram_mb = 512;
        reqs.binaries.push(RequiredBinary {
            name: "git".into(),
            required: true,
            install_hint: "install git".into(),
            fallback: None,
        });
        let p = profile_with_binary("git", 4096);
        assert_eq!(check_requirements(&reqs, &p), CheckOutcome::FullCapability);
    }

    #[test]
    fn missing_required_binary_blocked() {
        let mut reqs = SkillRequirements::new("test-skill");
        reqs.binaries.push(RequiredBinary {
            name: "ffmpeg".into(),
            required: true,
            install_hint: "brew install ffmpeg".into(),
            fallback: None,
        });
        let p = profile_with_binary("", 4096);
        let outcome = check_requirements(&reqs, &p);
        assert!(!outcome.can_execute());
        assert_eq!(outcome.label(), "blocked");
    }

    #[test]
    fn fallback_binary_satisfies_requirement() {
        let mut reqs = SkillRequirements::new("test-skill");
        reqs.binaries.push(RequiredBinary {
            name: "ffmpeg".into(),
            required: true,
            install_hint: "install ffmpeg".into(),
            fallback: Some("handbrake".into()),
        });
        let p = profile_with_binary("handbrake", 4096);
        assert_eq!(check_requirements(&reqs, &p), CheckOutcome::FullCapability);
    }

    #[test]
    fn optional_missing_degraded() {
        let mut reqs = SkillRequirements::new("test-skill");
        reqs.binaries.push(RequiredBinary {
            name: "ffprobe".into(),
            required: false,
            install_hint: "ships with ffmpeg".into(),
            fallback: None,
        });
        let p = profile_with_binary("", 4096);
        let outcome = check_requirements(&reqs, &p);
        assert!(outcome.can_execute());
        assert_eq!(outcome.label(), "degraded");
    }

    #[test]
    fn insufficient_ram_blocked() {
        let mut reqs = SkillRequirements::new("test-skill");
        reqs.min_ram_mb = 2048;
        reqs.low_resource_threshold_mb = 512;
        let p = profile_with_binary("", 256);
        let outcome = check_requirements(&reqs, &p);
        assert!(!outcome.can_execute());
    }
}
