use serde::{Deserialize, Serialize};

/// Installation profile determining which components to install.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstallProfile {
    /// Full GUI + TUI + server stack (macOS/Linux desktop).
    Desktop,
    /// TUI + CLI only, no GUI or server components.
    Terminal,
    /// Headless server: CLI + MCP servers + auth gating.
    Server,
}

/// Per-profile configuration describing what gets installed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConfig {
    pub profile: InstallProfile,
    pub install_gui: bool,
    pub install_tui: bool,
    pub install_cli: bool,
    pub install_mcp_servers: bool,
    pub install_voice: bool,
    pub require_auth: bool,
}

impl ProfileConfig {
    pub fn for_profile(profile: &InstallProfile) -> Self {
        match profile {
            InstallProfile::Desktop => Self {
                profile: InstallProfile::Desktop,
                install_gui: true,
                install_tui: true,
                install_cli: true,
                install_mcp_servers: true,
                install_voice: true,
                require_auth: false,
            },
            InstallProfile::Terminal => Self {
                profile: InstallProfile::Terminal,
                install_gui: false,
                install_tui: true,
                install_cli: true,
                install_mcp_servers: true,
                install_voice: false,
                require_auth: false,
            },
            InstallProfile::Server => Self {
                profile: InstallProfile::Server,
                install_gui: false,
                install_tui: false,
                install_cli: true,
                install_mcp_servers: true,
                install_voice: false,
                require_auth: true,
            },
        }
    }
}

/// Auto-detect the best installation profile from the environment.
///
/// Heuristic:
/// - If `DISPLAY` or `WAYLAND_DISPLAY` is set, or running on macOS → Desktop
/// - If `SSH_CONNECTION` is set or no TTY → Server
/// - Otherwise → Terminal
pub fn default_profile() -> InstallProfile {
    // Server indicators take precedence
    if std::env::var("SSH_CONNECTION").is_ok() {
        return InstallProfile::Server;
    }

    // Desktop indicators
    if std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok() {
        return InstallProfile::Desktop;
    }

    // macOS always has a desktop environment
    if cfg!(target_os = "macos") {
        return InstallProfile::Desktop;
    }

    InstallProfile::Terminal
}

/// Human-readable description of each profile.
pub fn profile_description(profile: &InstallProfile) -> &str {
    match profile {
        InstallProfile::Desktop => "Full desktop installation with GUI, TUI, CLI, voice, and MCP servers",
        InstallProfile::Terminal => "Terminal-only installation with TUI, CLI, and MCP servers",
        InstallProfile::Server => "Headless server installation with CLI, MCP servers, and auth gating",
    }
}
