//! Tool installer — installs missing tools using the best available package manager.

use super::detector::{MissingTool, ToolEcosystem};
use crate::environment::{EnvironmentProfile, Ecosystem, PackageManager};

/// Result of a tool installation attempt.
#[derive(Debug, Clone)]
pub struct InstallResult {
    pub success: bool,
    pub tool_name: String,
    pub package_manager: String,
    pub command_used: String,
    pub output: String,
    pub error: Option<String>,
}

impl InstallResult {
    pub fn summary(&self) -> String {
        if self.success {
            format!("Installed {} via {} ({})", self.tool_name, self.package_manager, self.command_used)
        } else {
            format!(
                "Failed to install {} via {}: {}",
                self.tool_name,
                self.package_manager,
                self.error.as_deref().unwrap_or("unknown error")
            )
        }
    }
}

/// Build the install command for a missing tool.
pub fn build_install_command(
    tool: &MissingTool,
    env: &EnvironmentProfile,
) -> Result<(String, String), String> {
    let pm = select_package_manager(tool, env)?;
    let pkg_name = tool.package_name(&pm.name);
    let command = pm.install_cmd(&pkg_name);
    Ok((command, pm.name.clone()))
}

/// Select the best package manager for installing a tool.
fn select_package_manager<'a>(
    tool: &MissingTool,
    env: &'a EnvironmentProfile,
) -> Result<&'a PackageManager, String> {
    // Map tool ecosystem to environment ecosystem
    let target_eco = match tool.ecosystem {
        ToolEcosystem::Rust => Some(Ecosystem::Rust),
        ToolEcosystem::Python => Some(Ecosystem::Python),
        ToolEcosystem::JavaScript => Some(Ecosystem::JavaScript),
        ToolEcosystem::Go => Some(Ecosystem::Go),
        ToolEcosystem::Ruby => Some(Ecosystem::Ruby),
        ToolEcosystem::System | ToolEcosystem::Unknown => None,
    };

    // Try ecosystem-specific installer first
    if let Some(eco) = target_eco {
        if let Some(pm) = env.installer_for(eco) {
            return Ok(pm);
        }
    }

    // Fall back to system installer
    env.best_installer()
        .ok_or_else(|| "No package manager available".to_string())
}

/// Check if a command requires elevated privileges (sudo).
pub fn requires_sudo(command: &str) -> bool {
    command.starts_with("sudo ")
        || command.contains("apt install")
        || command.contains("apt-get install")
        || command.contains("dnf install")
        || command.contains("pacman -S")
}

/// Risk level for an install operation.
pub fn install_risk(tool: &MissingTool, command: &str) -> InstallRisk {
    if requires_sudo(command) {
        InstallRisk::High
    } else if tool.ecosystem == ToolEcosystem::System {
        InstallRisk::Medium
    } else {
        InstallRisk::Low
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InstallRisk {
    Low,    // cargo install, pip install --user, npm install -g
    Medium, // brew install (no sudo but system-level)
    High,   // sudo apt install, sudo dnf install
}

impl InstallRisk {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    pub fn needs_approval(&self) -> bool {
        matches!(self, Self::Medium | Self::High)
    }
}

/// Create a belief record for a successful installation.
pub fn install_as_belief(tool: &MissingTool, result: &InstallResult) -> (String, String, String) {
    let subject = format!("tool_install:{}", tool.name);
    let content = format!(
        "Tool '{}' installed via {} using: {}\nEcosystem: {:?}",
        tool.name, result.package_manager, result.command_used, tool.ecosystem,
    );
    (subject, content, "Fact".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::detector::{DetectionSource, ToolEcosystem};

    fn make_tool(name: &str, eco: ToolEcosystem) -> MissingTool {
        MissingTool {
            name: name.to_string(),
            source: DetectionSource::ErrorOutput,
            ecosystem: eco,
        }
    }

    #[test]
    fn test_build_install_command() {
        let env = EnvironmentProfile::probe_all();
        let tool = make_tool("jq", ToolEcosystem::System);
        let result = build_install_command(&tool, &env);
        // Should succeed if any PM is available
        if let Ok((cmd, pm)) = result {
            assert!(cmd.contains("jq"));
            assert!(!pm.is_empty());
        }
    }

    #[test]
    fn test_build_install_rust_tool() {
        let env = EnvironmentProfile::probe_all();
        let tool = make_tool("cargo-watch", ToolEcosystem::Rust);
        if let Ok((cmd, pm)) = build_install_command(&tool, &env) {
            assert!(cmd.contains("cargo-watch"));
            assert_eq!(pm, "cargo");
        }
    }

    #[test]
    fn test_requires_sudo() {
        assert!(requires_sudo("sudo apt install jq"));
        assert!(requires_sudo("sudo dnf install gcc"));
        assert!(!requires_sudo("brew install jq"));
        assert!(!requires_sudo("cargo install ripgrep"));
    }

    #[test]
    fn test_install_risk() {
        let tool = make_tool("jq", ToolEcosystem::System);
        assert_eq!(install_risk(&tool, "sudo apt install jq"), InstallRisk::High);
        assert_eq!(install_risk(&tool, "brew install jq"), InstallRisk::Medium);

        let rust_tool = make_tool("cargo-watch", ToolEcosystem::Rust);
        assert_eq!(install_risk(&rust_tool, "cargo install cargo-watch"), InstallRisk::Low);
    }

    #[test]
    fn test_install_result_summary() {
        let result = InstallResult {
            success: true,
            tool_name: "jq".into(),
            package_manager: "brew".into(),
            command_used: "brew install jq".into(),
            output: "installed".into(),
            error: None,
        };
        assert!(result.summary().contains("Installed jq via brew"));
    }

    #[test]
    fn test_install_result_failure() {
        let result = InstallResult {
            success: false,
            tool_name: "foo".into(),
            package_manager: "apt".into(),
            command_used: "sudo apt install foo".into(),
            output: String::new(),
            error: Some("package not found".into()),
        };
        assert!(result.summary().contains("Failed"));
    }

    #[test]
    fn test_install_as_belief() {
        let tool = make_tool("jq", ToolEcosystem::System);
        let result = InstallResult {
            success: true,
            tool_name: "jq".into(),
            package_manager: "brew".into(),
            command_used: "brew install jq".into(),
            output: String::new(),
            error: None,
        };
        let (subject, content, category) = install_as_belief(&tool, &result);
        assert!(subject.contains("jq"));
        assert!(content.contains("brew"));
        assert_eq!(category, "Fact");
    }

    #[test]
    fn test_risk_needs_approval() {
        assert!(!InstallRisk::Low.needs_approval());
        assert!(InstallRisk::Medium.needs_approval());
        assert!(InstallRisk::High.needs_approval());
    }
}
