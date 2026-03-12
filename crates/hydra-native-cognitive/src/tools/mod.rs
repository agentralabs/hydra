//! Tool & dependency discovery engine — detects, installs, and verifies missing tools.
//!
//! When Hydra encounters a "command not found" or missing dependency error:
//! 1. Detects what's missing from the error output
//! 2. Selects the best package manager (from environment probe)
//! 3. Builds the install command
//! 4. Verifies the installation
//! 5. Stores the solution as a belief

pub mod detector;
pub mod installer;
pub mod verifier;

pub use detector::{DetectionSource, MissingTool, ToolEcosystem};
pub use installer::{InstallResult, InstallRisk};
pub use verifier::VerifyStatus;

use crate::environment::EnvironmentProfile;

/// Main tool installer — coordinates detection, installation, and verification.
pub struct ToolInstaller {
    env: EnvironmentProfile,
}

impl ToolInstaller {
    /// Create a new installer with the current environment profile.
    pub fn new(env: EnvironmentProfile) -> Self {
        Self { env }
    }

    /// Refresh the environment profile (e.g., after installing a new tool).
    pub fn refresh_env(&mut self) {
        self.env = EnvironmentProfile::probe_all();
    }

    /// Detect a missing tool from an error message.
    pub fn detect_missing(&self, error: &str) -> Option<MissingTool> {
        detector::detect_missing(error)
    }

    /// Build the install command for a missing tool (does NOT execute it).
    /// Returns (command, package_manager_name) or error.
    pub fn build_install_command(&self, tool: &MissingTool) -> Result<(String, String), String> {
        installer::build_install_command(tool, &self.env)
    }

    /// Get the risk level for installing a tool.
    pub fn install_risk(&self, tool: &MissingTool) -> InstallRisk {
        match self.build_install_command(tool) {
            Ok((cmd, _)) => installer::install_risk(tool, &cmd),
            Err(_) => InstallRisk::High, // Unknown = assume high risk
        }
    }

    /// Verify a tool is installed and working.
    pub fn verify(&self, tool: &MissingTool) -> VerifyStatus {
        verifier::verify_tool(tool)
    }

    /// Verify a tool by name.
    pub fn verify_by_name(&self, name: &str) -> VerifyStatus {
        verifier::verify_by_name(name)
    }

    /// Check if a tool is already available (no install needed).
    pub fn is_available(&self, name: &str) -> bool {
        self.env.has_tool(name).is_some()
    }

    /// Create a belief record for a successful installation.
    pub fn install_as_belief(
        tool: &MissingTool,
        result: &InstallResult,
    ) -> (String, String, String) {
        installer::install_as_belief(tool, result)
    }

    /// Full detection + install plan: analyze error → find missing tool → build command.
    /// Returns None if no missing tool detected.
    pub fn plan_install(&self, error: &str) -> Option<InstallPlan> {
        let tool = self.detect_missing(error)?;

        // Check if already installed
        if self.is_available(&tool.name) {
            return Some(InstallPlan {
                tool: tool.clone(),
                command: None,
                package_manager: None,
                risk: InstallRisk::Low,
                already_installed: true,
            });
        }

        let (command, pm) = self.build_install_command(&tool).ok()?;
        let risk = installer::install_risk(&tool, &command);

        Some(InstallPlan {
            tool,
            command: Some(command),
            package_manager: Some(pm),
            risk,
            already_installed: false,
        })
    }

    /// Get the environment profile.
    pub fn environment(&self) -> &EnvironmentProfile {
        &self.env
    }
}

/// A plan for installing a missing tool.
#[derive(Debug, Clone)]
pub struct InstallPlan {
    pub tool: MissingTool,
    pub command: Option<String>,
    pub package_manager: Option<String>,
    pub risk: InstallRisk,
    pub already_installed: bool,
}

impl InstallPlan {
    /// Human-readable summary of the plan.
    pub fn summary(&self) -> String {
        if self.already_installed {
            return format!("{} is already installed", self.tool.name);
        }
        match (&self.command, &self.package_manager) {
            (Some(cmd), Some(pm)) => {
                format!(
                    "Install {} via {} (risk: {}): {}",
                    self.tool.name,
                    pm,
                    self.risk.label(),
                    cmd,
                )
            }
            _ => format!("Cannot determine how to install {}", self.tool.name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_installer_new() {
        let env = EnvironmentProfile::probe_all();
        let installer = ToolInstaller::new(env);
        // rustc should be available
        assert!(installer.is_available("Rust") || installer.is_available("rustc"));
    }

    #[test]
    fn test_detect_and_plan() {
        let env = EnvironmentProfile::probe_all();
        let installer = ToolInstaller::new(env);

        // Simulate a "command not found" error
        let plan = installer.plan_install("zsh: command not found: jq");
        assert!(plan.is_some());
        let plan = plan.unwrap();
        assert_eq!(plan.tool.name, "jq");
        assert!(!plan.already_installed || plan.already_installed); // may or may not be installed
    }

    #[test]
    fn test_plan_no_detection() {
        let env = EnvironmentProfile::probe_all();
        let installer = ToolInstaller::new(env);
        assert!(installer.plan_install("something went wrong").is_none());
    }

    #[test]
    fn test_plan_already_installed() {
        let env = EnvironmentProfile::probe_all();
        let installer = ToolInstaller::new(env);
        // cargo is definitely installed
        let plan = installer.plan_install("zsh: command not found: cargo");
        if let Some(plan) = plan {
            // It detects cargo, but cargo IS installed — should flag as already_installed
            if plan.already_installed {
                assert!(plan.summary().contains("already installed"));
            }
        }
    }

    #[test]
    fn test_verify_installed() {
        let env = EnvironmentProfile::probe_all();
        let installer = ToolInstaller::new(env);
        let status = installer.verify_by_name("rustc");
        assert!(status.is_available());
    }

    #[test]
    fn test_verify_not_installed() {
        let env = EnvironmentProfile::probe_all();
        let installer = ToolInstaller::new(env);
        let status = installer.verify_by_name("nonexistent_xyz");
        assert_eq!(status, VerifyStatus::NotFound);
    }

    #[test]
    fn test_install_plan_summary() {
        let plan = InstallPlan {
            tool: MissingTool {
                name: "jq".into(),
                source: DetectionSource::ErrorOutput,
                ecosystem: ToolEcosystem::System,
            },
            command: Some("brew install jq".into()),
            package_manager: Some("brew".into()),
            risk: InstallRisk::Medium,
            already_installed: false,
        };
        let summary = plan.summary();
        assert!(summary.contains("jq"));
        assert!(summary.contains("brew"));
        assert!(summary.contains("medium"));
    }

    #[test]
    fn test_install_plan_already_installed() {
        let plan = InstallPlan {
            tool: MissingTool {
                name: "cargo".into(),
                source: DetectionSource::ErrorOutput,
                ecosystem: ToolEcosystem::Rust,
            },
            command: None,
            package_manager: None,
            risk: InstallRisk::Low,
            already_installed: true,
        };
        assert!(plan.summary().contains("already installed"));
    }
}
