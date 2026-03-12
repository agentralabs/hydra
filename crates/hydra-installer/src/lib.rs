pub mod auth_setup;
pub mod completions;
pub mod error;
pub mod mcp;
pub mod profile;
pub mod steps;

pub use auth_setup::{auth_required, generate_token, setup_auth};
pub use completions::{completion_path, install_completions};
pub use error::InstallerError;
pub use mcp::{load_mcp_config, merge_mcp_config, save_mcp_config, McpConfig, McpServerEntry};
pub use profile::{default_profile, profile_description, InstallProfile, ProfileConfig};
pub use steps::{execute_step, steps_for_profile, InstallStep, StepResult};
