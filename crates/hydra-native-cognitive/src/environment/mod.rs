//! Environment adaptation engine — probes the system to build a complete profile.
//!
//! Detects OS, languages, package managers, services, and resources.
//! Used by the cognitive loop for context-aware decisions.

pub mod os_probe;
pub mod lang_probe;
pub mod pkg_probe;
pub mod service_probe;
pub mod resource_probe;

pub use lang_probe::Language;
pub use os_probe::OsInfo;
pub use pkg_probe::{Ecosystem, PackageManager};
pub use resource_probe::ResourceInfo;
pub use service_probe::Service;

/// Complete picture of the current system.
#[derive(Debug, Clone)]
pub struct EnvironmentProfile {
    pub os: OsInfo,
    pub languages: Vec<Language>,
    pub package_managers: Vec<PackageManager>,
    pub services: Vec<Service>,
    pub resources: ResourceInfo,
    pub shell: String,
    pub user: String,
    pub hostname: String,
    pub probed_at: chrono::DateTime<chrono::Utc>,
}

impl EnvironmentProfile {
    /// Probe everything — called on startup and on-demand.
    pub fn probe_all() -> Self {
        Self {
            os: os_probe::probe_os(),
            languages: lang_probe::probe_languages(),
            package_managers: pkg_probe::probe_package_managers(),
            services: service_probe::probe_services(),
            resources: resource_probe::probe_resources(),
            shell: os_probe::probe_shell(),
            user: os_probe::probe_user(),
            hostname: os_probe::probe_hostname(),
            probed_at: chrono::Utc::now(),
        }
    }

    /// Quick summary for LLM context injection.
    pub fn summary(&self) -> String {
        let langs: Vec<String> = self.languages.iter().map(|l| l.display()).collect();
        let pkgs: Vec<String> = self.package_managers.iter().map(|p| p.name.clone()).collect();
        let svcs: Vec<String> = self.services.iter().map(|s| s.display()).collect();

        format!(
            "{} {} {} | {} | {} | {} | {}",
            self.os.name,
            self.os.version,
            self.os.arch,
            if langs.is_empty() { "no languages detected".to_string() } else { langs.join(", ") },
            if pkgs.is_empty() { "no package managers".to_string() } else { pkgs.join(", ") },
            if svcs.is_empty() { "no services".to_string() } else { svcs.join(", ") },
            self.resources.display(),
        )
    }

    /// Check if a specific tool/language is available. Returns version if found.
    pub fn has_tool(&self, name: &str) -> Option<String> {
        let lower = name.to_lowercase();

        // Check languages
        for lang in &self.languages {
            if lang.name.to_lowercase() == lower || lang.binary.to_lowercase() == lower {
                return Some(lang.version.clone());
            }
        }

        // Check package managers
        for pm in &self.package_managers {
            if pm.name.to_lowercase() == lower || pm.binary.to_lowercase() == lower {
                return Some(pm.version.clone());
            }
        }

        // Fallback: direct check
        lang_probe::check_tool(name)
    }

    /// Best system-level package manager for installing tools.
    pub fn best_installer(&self) -> Option<&PackageManager> {
        pkg_probe::best_system_installer(&self.package_managers)
    }

    /// Best package manager for a specific ecosystem.
    pub fn installer_for(&self, eco: Ecosystem) -> Option<&PackageManager> {
        pkg_probe::best_ecosystem_installer(&self.package_managers, eco)
    }

    /// Formatted multi-line display for `/env` command.
    pub fn display_full(&self) -> String {
        let age = chrono::Utc::now()
            .signed_duration_since(self.probed_at)
            .num_seconds();
        let age_str = if age < 60 {
            format!("{}s ago", age)
        } else {
            format!("{}m ago", age / 60)
        };

        let langs: Vec<String> = self.languages.iter().map(|l| l.display()).collect();
        let pkgs: Vec<String> = self.package_managers.iter().map(|p| p.display()).collect();
        let svcs: Vec<String> = self.services.iter().map(|s| s.display()).collect();

        format!(
            "Environment Profile (probed {}):\n\
             \x20 OS:        {} {} ({})\n\
             \x20 Kernel:    {}\n\
             \x20 Languages: {}\n\
             \x20 Packages:  {}\n\
             \x20 Services:  {}\n\
             \x20 Resources: {}\n\
             \x20 Shell:     {}\n\
             \x20 User:      {}@{}",
            age_str,
            self.os.name,
            self.os.version,
            self.os.arch,
            self.os.kernel,
            if langs.is_empty() { "none detected".to_string() } else { langs.join(", ") },
            if pkgs.is_empty() { "none detected".to_string() } else { pkgs.join(", ") },
            if svcs.is_empty() { "none detected".to_string() } else { svcs.join(", ") },
            self.resources.display(),
            self.shell,
            self.user,
            self.hostname,
        )
    }
}

/// Generate the `/env` slash command output.
pub fn env_slash_command_output(profile: &EnvironmentProfile) -> String {
    profile.display_full()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_probe_all_completes() {
        let start = std::time::Instant::now();
        let profile = EnvironmentProfile::probe_all();
        let elapsed = start.elapsed();
        assert!(elapsed.as_secs() < 10, "Probe should complete in <10s, took {:?}", elapsed);
        assert!(!profile.os.name.is_empty());
        assert!(!profile.user.is_empty());
    }

    #[test]
    fn test_summary_non_empty() {
        let profile = EnvironmentProfile::probe_all();
        let summary = profile.summary();
        assert!(!summary.is_empty());
        assert!(summary.contains(&profile.os.name));
    }

    #[test]
    fn test_has_tool_rust() {
        let profile = EnvironmentProfile::probe_all();
        assert!(profile.has_tool("Rust").is_some() || profile.has_tool("rustc").is_some());
    }

    #[test]
    fn test_has_tool_nonexistent() {
        let profile = EnvironmentProfile::probe_all();
        assert!(profile.has_tool("nonexistent_tool_xyz_999").is_none());
    }

    #[test]
    fn test_best_installer() {
        let profile = EnvironmentProfile::probe_all();
        // On macOS should find brew, may be None on minimal systems
        let _best = profile.best_installer(); // just verify no crash
    }

    #[test]
    fn test_display_full() {
        let profile = EnvironmentProfile::probe_all();
        let display = profile.display_full();
        assert!(display.contains("Environment Profile"));
        assert!(display.contains("OS:"));
        assert!(display.contains("Languages:"));
        assert!(display.contains("Resources:"));
    }

    #[test]
    fn test_env_slash_command() {
        let profile = EnvironmentProfile::probe_all();
        let output = env_slash_command_output(&profile);
        assert!(output.contains("Environment Profile"));
    }
}
