use serde::{Deserialize, Serialize};

use crate::error::InstallerError;
use crate::profile::{InstallProfile, ProfileConfig};

/// Individual steps the installer can execute.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstallStep {
    /// Verify required external dependencies are present.
    CheckDeps,
    /// Create data/config directories.
    CreateDirs,
    /// Write Hydra configuration files.
    WriteConfig,
    /// Merge Hydra MCP servers into the user's MCP config.
    MergeMcp,
    /// Generate auth tokens (server profile).
    SetupAuth,
    /// Install shell completions.
    WriteCompletions,
    /// Print the post-install welcome banner.
    PrintBanner,
}

/// Result of executing a single install step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub step: InstallStep,
    pub success: bool,
    pub message: String,
}

impl StepResult {
    pub fn ok(step: InstallStep, message: impl Into<String>) -> Self {
        Self {
            step,
            success: true,
            message: message.into(),
        }
    }

    pub fn fail(step: InstallStep, message: impl Into<String>) -> Self {
        Self {
            step,
            success: false,
            message: message.into(),
        }
    }
}

/// Return the ordered list of install steps for a given profile.
pub fn steps_for_profile(profile: &InstallProfile) -> Vec<InstallStep> {
    let config = ProfileConfig::for_profile(profile);

    let mut steps = vec![InstallStep::CheckDeps, InstallStep::CreateDirs];

    steps.push(InstallStep::WriteConfig);

    if config.install_mcp_servers {
        steps.push(InstallStep::MergeMcp);
    }

    if config.require_auth {
        steps.push(InstallStep::SetupAuth);
    }

    steps.push(InstallStep::WriteCompletions);
    steps.push(InstallStep::PrintBanner);

    steps
}

/// Execute a single installation step.
///
/// This is a skeleton implementation — each branch will be fleshed out
/// as we wire up real filesystem and process operations.
pub fn execute_step(step: &InstallStep) -> Result<StepResult, InstallerError> {
    match step {
        InstallStep::CheckDeps => Ok(StepResult::ok(
            InstallStep::CheckDeps,
            "All dependencies satisfied",
        )),
        InstallStep::CreateDirs => Ok(StepResult::ok(
            InstallStep::CreateDirs,
            "Directories created",
        )),
        InstallStep::WriteConfig => Ok(StepResult::ok(
            InstallStep::WriteConfig,
            "Configuration written",
        )),
        InstallStep::MergeMcp => Ok(StepResult::ok(
            InstallStep::MergeMcp,
            "MCP configuration merged",
        )),
        InstallStep::SetupAuth => Ok(StepResult::ok(
            InstallStep::SetupAuth,
            "Auth token generated",
        )),
        InstallStep::WriteCompletions => Ok(StepResult::ok(
            InstallStep::WriteCompletions,
            "Shell completions installed",
        )),
        InstallStep::PrintBanner => Ok(StepResult::ok(
            InstallStep::PrintBanner,
            "Welcome to Hydra!",
        )),
    }
}
