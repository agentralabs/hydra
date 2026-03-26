//! Generic self-sufficiency — Hydra detects and installs ANY missing dependency.
//!
//! When any shell command fails with "command not found" or similar,
//! Hydra figures out what package provides it and installs it.
//! No hardcoded lists — uses the package manager's search to resolve.
//! Like a human: see the error → figure out the fix → apply it → retry.

use std::collections::HashMap;
use std::process::Command;
use std::sync::Mutex;
use std::time::Instant;

static INSTALL_CACHE: Mutex<Option<HashMap<String, Instant>>> = Mutex::new(None);

/// Check if a command exists in PATH.
pub fn cmd_exists(cmd: &str) -> bool {
    Command::new("which").arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status().map(|s| s.success()).unwrap_or(false)
}

/// Detect the platform's package manager.
fn detect_package_manager() -> Option<PackageManager> {
    if cfg!(target_os = "macos") {
        if cmd_exists("brew") { return Some(PackageManager::Brew); }
        if cmd_exists("port") { return Some(PackageManager::MacPorts); }
    }
    if cfg!(target_os = "linux") {
        if cmd_exists("apt-get") { return Some(PackageManager::Apt); }
        if cmd_exists("dnf") { return Some(PackageManager::Dnf); }
        if cmd_exists("pacman") { return Some(PackageManager::Pacman); }
        if cmd_exists("zypper") { return Some(PackageManager::Zypper); }
    }
    None
}

#[derive(Debug, Clone)]
enum PackageManager { Brew, MacPorts, Apt, Dnf, Pacman, Zypper }

impl PackageManager {
    /// Search for a package that provides a given command.
    fn search(&self, cmd_name: &str) -> Option<String> {
        let output = match self {
            Self::Brew => Command::new("brew").args(["search", cmd_name])
                .output().ok()?,
            Self::Apt => Command::new("apt-cache").args(["search", cmd_name])
                .output().ok()?,
            Self::Dnf => Command::new("dnf").args(["search", cmd_name])
                .output().ok()?,
            Self::Pacman => Command::new("pacman").args(["-Ss", cmd_name])
                .output().ok()?,
            Self::MacPorts => Command::new("port").args(["search", cmd_name])
                .output().ok()?,
            Self::Zypper => Command::new("zypper").args(["search", cmd_name])
                .output().ok()?,
        };
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Find the best match — usually the first line that contains the exact command name
        for line in stdout.lines() {
            let pkg = line.split_whitespace().next().unwrap_or("");
            // Exact match or contains the command name
            if pkg == cmd_name || pkg.contains(cmd_name) {
                // Clean up brew format (removes version info)
                let clean = pkg.split('/').last().unwrap_or(pkg)
                    .split('@').next().unwrap_or(pkg);
                return Some(clean.to_string());
            }
        }
        // Fallback: the command name itself is often the package name
        Some(cmd_name.to_string())
    }

    /// Install a package by name.
    fn install(&self, package: &str) -> bool {
        eprintln!("hydra-deps: installing '{package}' via {:?}...", self);
        let status = match self {
            Self::Brew => Command::new("brew").args(["install", package])
                .stdout(std::process::Stdio::null()).status(),
            Self::Apt => Command::new("sudo").args(["apt-get", "install", "-y", package])
                .stdout(std::process::Stdio::null()).status(),
            Self::Dnf => Command::new("sudo").args(["dnf", "install", "-y", package])
                .stdout(std::process::Stdio::null()).status(),
            Self::Pacman => Command::new("sudo").args(["pacman", "-S", "--noconfirm", package])
                .stdout(std::process::Stdio::null()).status(),
            Self::MacPorts => Command::new("sudo").args(["port", "install", package])
                .stdout(std::process::Stdio::null()).status(),
            Self::Zypper => Command::new("sudo").args(["zypper", "install", "-y", package])
                .stdout(std::process::Stdio::null()).status(),
        };
        match status {
            Ok(s) if s.success() => {
                eprintln!("hydra-deps: '{package}' installed successfully");
                true
            }
            _ => {
                eprintln!("hydra-deps: failed to install '{package}'");
                false
            }
        }
    }
}

/// Ensure a command is available. If missing, find the package and install it.
/// Returns true if the command is now available (was present or successfully installed).
/// Caches install attempts to avoid retrying within the same session.
pub fn ensure_command(cmd: &str) -> bool {
    if cmd_exists(cmd) { return true; }

    // Check cache — don't retry failed installs in same session
    {
        let mut cache = INSTALL_CACHE.lock().unwrap();
        let map = cache.get_or_insert_with(HashMap::new);
        if let Some(when) = map.get(cmd) {
            if when.elapsed().as_secs() < 3600 { return cmd_exists(cmd); }
        }
    }

    eprintln!("hydra-deps: '{cmd}' not found — resolving...");

    let pm = match detect_package_manager() {
        Some(pm) => pm,
        None => {
            eprintln!("hydra-deps: no package manager found — cannot install '{cmd}'");
            return false;
        }
    };

    // Search for the package
    let package = match pm.search(cmd) {
        Some(pkg) => pkg,
        None => {
            eprintln!("hydra-deps: could not find package for '{cmd}'");
            return false;
        }
    };

    // Install it
    let success = pm.install(&package);

    // Cache the attempt
    {
        let mut cache = INSTALL_CACHE.lock().unwrap();
        let map = cache.get_or_insert_with(HashMap::new);
        map.insert(cmd.to_string(), Instant::now());
    }

    success && cmd_exists(cmd)
}

/// Detect and install a missing command from an error message.
/// Parses error output for "command not found", "No such file", etc.
/// Returns the command name if detected and installed.
pub fn resolve_from_error(error_msg: &str) -> Option<String> {
    let lower = error_msg.to_lowercase();

    // Pattern: "command not found: <cmd>"
    if lower.contains("command not found") {
        let cmd = error_msg.split("command not found:").last()
            .or_else(|| error_msg.split("command not found").last())
            .map(|s| s.trim().trim_matches('\'').trim_matches('"'))
            .filter(|s| !s.is_empty() && s.len() < 50)?;
        if ensure_command(cmd) { return Some(cmd.to_string()); }
    }

    // Pattern: "No such file or directory" with a path containing a binary name
    if lower.contains("no such file or directory") {
        // Extract the last path component
        let parts: Vec<&str> = error_msg.split('/').collect();
        if let Some(binary) = parts.last() {
            let clean = binary.trim().trim_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '_');
            if !clean.is_empty() && clean.len() < 50 {
                if ensure_command(clean) { return Some(clean.to_string()); }
            }
        }
    }

    // Pattern: "error: program '<cmd>' not found"
    if lower.contains("not found") || lower.contains("not installed") {
        // Try to extract the program name between quotes
        let between_quotes: Vec<&str> = error_msg.split('\'').collect();
        if between_quotes.len() >= 3 {
            let cmd = between_quotes[1].trim();
            if !cmd.is_empty() && cmd.len() < 50 && !cmd.contains(' ') {
                if ensure_command(cmd) { return Some(cmd.to_string()); }
            }
        }
    }

    None
}

/// Check macOS permissions (screen recording + accessibility).
pub fn check_permissions() -> (bool, bool) {
    let screen_ok = crate::screen::ScreenCapture::has_permission();
    let a11y_ok = if cfg!(target_os = "macos") {
        Command::new("osascript")
            .args(["-e", "tell application \"System Events\" to get name of first process whose frontmost is true"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status().map(|s| s.success()).unwrap_or(false)
    } else { true };
    (screen_ok, a11y_ok)
}

/// Preflight: check permissions, prompt user to grant if missing.
/// Opens System Settings automatically on macOS. Returns true if all OK.
pub fn preflight() -> bool {
    let (screen_ok, a11y_ok) = check_permissions();
    if screen_ok && a11y_ok {
        eprintln!("hydra-deps: preflight OK — permissions granted, deps install on-demand");
        return true;
    }
    if !a11y_ok {
        eprintln!("hydra-deps: ⚠ Accessibility permission required for desktop automation");
        eprintln!("hydra-deps: Opening System Settings — please grant access to your terminal app");
        // Open Accessibility preferences directly
        let _ = Command::new("open").arg(
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
        ).spawn();
        // Wait for user to grant permission (poll every 2s, timeout 60s)
        for i in 0..30 {
            std::thread::sleep(std::time::Duration::from_secs(2));
            let (_, ok) = check_permissions();
            if ok {
                eprintln!("hydra-deps: ✓ Accessibility permission granted!");
                break;
            }
            if i % 5 == 4 {
                eprintln!("hydra-deps: still waiting for Accessibility permission... ({}/60s)", (i+1)*2);
            }
        }
    }
    if !screen_ok {
        eprintln!("hydra-deps: ⚠ Screen Recording permission required for screenshots");
        eprintln!("hydra-deps: Opening System Settings — please grant access to your terminal app");
        let _ = Command::new("open").arg(
            "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture"
        ).spawn();
        for i in 0..30 {
            std::thread::sleep(std::time::Duration::from_secs(2));
            let (ok, _) = check_permissions();
            if ok {
                eprintln!("hydra-deps: ✓ Screen Recording permission granted!");
                break;
            }
            if i % 5 == 4 {
                eprintln!("hydra-deps: still waiting for Screen Recording permission... ({}/60s)", (i+1)*2);
            }
        }
    }
    let (s, a) = check_permissions();
    if !s || !a {
        eprintln!("hydra-deps: ⚠ Some permissions still missing. Desktop automation may not work.");
        eprintln!("hydra-deps: Grant in: System Settings > Privacy & Security > Accessibility + Screen Recording");
    }
    s && a
}
