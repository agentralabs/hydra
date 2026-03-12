//! Package manager probe — detects available package managers.

use super::os_probe::run_cmd;

/// A detected package manager.
#[derive(Debug, Clone)]
pub struct PackageManager {
    pub name: String,
    pub binary: String,
    pub version: String,
    pub ecosystem: Ecosystem,
}

/// What ecosystem a package manager serves.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ecosystem {
    Rust,
    Python,
    JavaScript,
    System,
    Go,
    Ruby,
    General,
}

impl PackageManager {
    pub fn display(&self) -> String {
        if self.version.is_empty() {
            self.name.clone()
        } else {
            format!("{} {}", self.name, self.version)
        }
    }

    /// Get the install command for this package manager.
    pub fn install_cmd(&self, package: &str) -> String {
        match self.binary.as_str() {
            "cargo" => format!("cargo install {}", package),
            "pip3" | "pip" => format!("pip3 install {}", package),
            "npm" => format!("npm install -g {}", package),
            "brew" => format!("brew install {}", package),
            "apt" => format!("sudo apt install -y {}", package),
            "dnf" => format!("sudo dnf install -y {}", package),
            "pacman" => format!("sudo pacman -S --noconfirm {}", package),
            "go" => format!("go install {}@latest", package),
            "gem" => format!("gem install {}", package),
            _ => format!("{} install {}", self.binary, package),
        }
    }
}

/// Probe for all known package managers.
pub fn probe_package_managers() -> Vec<PackageManager> {
    let probes: Vec<(&str, &str, &[&str], Ecosystem)> = vec![
        ("cargo", "cargo", &["--version"], Ecosystem::Rust),
        ("rustup", "rustup", &["--version"], Ecosystem::Rust),
        ("pip", "pip3", &["--version"], Ecosystem::Python),
        ("npm", "npm", &["--version"], Ecosystem::JavaScript),
        ("yarn", "yarn", &["--version"], Ecosystem::JavaScript),
        ("pnpm", "pnpm", &["--version"], Ecosystem::JavaScript),
        ("bun", "bun", &["--version"], Ecosystem::JavaScript),
        ("brew", "brew", &["--version"], Ecosystem::System),
        ("apt", "apt", &["--version"], Ecosystem::System),
        ("dnf", "dnf", &["--version"], Ecosystem::System),
        ("pacman", "pacman", &["--version"], Ecosystem::System),
        ("go", "go", &["version"], Ecosystem::Go),
        ("gem", "gem", &["--version"], Ecosystem::Ruby),
    ];

    let mut managers = Vec::new();
    for (name, binary, args, ecosystem) in probes {
        if let Some(output) = run_cmd(binary, args) {
            let version = extract_version(&output);
            managers.push(PackageManager {
                name: name.to_string(),
                binary: binary.to_string(),
                version,
                ecosystem,
            });
        }
    }
    managers
}

/// Pick the best system-level package manager for the current OS.
pub fn best_system_installer(managers: &[PackageManager]) -> Option<&PackageManager> {
    // Prefer: brew (macOS) > apt (Debian/Ubuntu) > dnf (Fedora) > pacman (Arch)
    let preference = ["brew", "apt", "dnf", "pacman"];
    for pref in &preference {
        if let Some(pm) = managers.iter().find(|m| m.binary == *pref) {
            return Some(pm);
        }
    }
    managers.iter().find(|m| m.ecosystem == Ecosystem::System)
}

/// Pick the best installer for a specific ecosystem.
pub fn best_ecosystem_installer(managers: &[PackageManager], eco: Ecosystem) -> Option<&PackageManager> {
    managers.iter().find(|m| m.ecosystem == eco)
}

fn extract_version(output: &str) -> String {
    let first = output.lines().next().unwrap_or("");
    first
        .split_whitespace()
        .find(|w| {
            w.chars().next().map_or(false, |c| c.is_ascii_digit())
                || w.starts_with('v')
        })
        .map(|w| w.trim_start_matches('v').trim_end_matches(')').to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_probe_finds_cargo() {
        let managers = probe_package_managers();
        assert!(
            managers.iter().any(|m| m.name == "cargo"),
            "Should detect cargo (required for this project)"
        );
    }

    #[test]
    fn test_install_cmd() {
        let pm = PackageManager {
            name: "cargo".into(),
            binary: "cargo".into(),
            version: "1.77.0".into(),
            ecosystem: Ecosystem::Rust,
        };
        assert_eq!(pm.install_cmd("ripgrep"), "cargo install ripgrep");
    }

    #[test]
    fn test_best_system_installer() {
        let managers = probe_package_managers();
        // On macOS we should get brew, on Linux apt/dnf/pacman
        let best = best_system_installer(&managers);
        // May be None on minimal systems, that's OK
        if let Some(pm) = best {
            assert!(pm.ecosystem == Ecosystem::System);
        }
    }

    #[test]
    fn test_best_ecosystem_installer_rust() {
        let managers = probe_package_managers();
        let rust_pm = best_ecosystem_installer(&managers, Ecosystem::Rust);
        assert!(rust_pm.is_some(), "Should have a Rust package manager");
    }

    #[test]
    fn test_display() {
        let pm = PackageManager {
            name: "npm".into(),
            binary: "npm".into(),
            version: "10.2.0".into(),
            ecosystem: Ecosystem::JavaScript,
        };
        assert_eq!(pm.display(), "npm 10.2.0");
    }
}
