//! Auto-dependency installer — Hydra installs its own system dependencies.
//!
//! On first run, checks for required tools (tesseract, cliclick, etc.)
//! and installs them via Homebrew (macOS) or apt (Linux).
//! Never asks the user to install anything manually.

use std::process::Command;

/// A system dependency with install command.
struct Dep {
    name: &'static str,
    check_cmd: &'static str,
    brew_pkg: &'static str,
    apt_pkg: &'static str,
    critical: bool,
}

const DEPS: &[Dep] = &[
    Dep { name: "tesseract", check_cmd: "tesseract", brew_pkg: "tesseract", apt_pkg: "tesseract-ocr", critical: false },
    Dep { name: "cliclick", check_cmd: "cliclick", brew_pkg: "cliclick", apt_pkg: "", critical: false },
    Dep { name: "ImageMagick", check_cmd: "convert", brew_pkg: "imagemagick", apt_pkg: "imagemagick", critical: false },
];

/// Check if a command exists in PATH.
fn cmd_exists(cmd: &str) -> bool {
    Command::new("which").arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status().map(|s| s.success()).unwrap_or(false)
}

/// Check if Homebrew is available (macOS).
fn has_brew() -> bool { cmd_exists("brew") }

/// Check if apt-get is available (Linux).
fn has_apt() -> bool { cmd_exists("apt-get") }

/// Install a package via the appropriate package manager.
fn install_pkg(dep: &Dep) -> bool {
    if cfg!(target_os = "macos") && has_brew() && !dep.brew_pkg.is_empty() {
        eprintln!("hydra-deps: installing {} via brew...", dep.name);
        let status = Command::new("brew")
            .args(["install", dep.brew_pkg])
            .stdout(std::process::Stdio::null())
            .status();
        match status {
            Ok(s) if s.success() => {
                eprintln!("hydra-deps: {} installed successfully", dep.name);
                true
            }
            _ => {
                eprintln!("hydra-deps: failed to install {}", dep.name);
                false
            }
        }
    } else if cfg!(target_os = "linux") && has_apt() && !dep.apt_pkg.is_empty() {
        eprintln!("hydra-deps: installing {} via apt-get...", dep.name);
        let status = Command::new("sudo")
            .args(["apt-get", "install", "-y", dep.apt_pkg])
            .stdout(std::process::Stdio::null())
            .status();
        match status {
            Ok(s) if s.success() => {
                eprintln!("hydra-deps: {} installed successfully", dep.name);
                true
            }
            _ => {
                eprintln!("hydra-deps: failed to install {}", dep.name);
                false
            }
        }
    } else {
        eprintln!("hydra-deps: no package manager found for {}", dep.name);
        false
    }
}

/// Check and install all dependencies. Called once at startup.
/// Returns (installed_count, missing_count).
pub fn ensure_deps() -> (usize, usize) {
    let mut installed = 0;
    let mut missing = 0;
    for dep in DEPS {
        if cmd_exists(dep.check_cmd) {
            continue; // already installed
        }
        eprintln!("hydra-deps: {} not found, attempting install...", dep.name);
        if install_pkg(dep) {
            installed += 1;
        } else {
            missing += 1;
            if dep.critical {
                eprintln!("hydra-deps: CRITICAL dependency {} missing — some features will not work", dep.name);
            }
        }
    }
    if installed > 0 {
        eprintln!("hydra-deps: installed {installed} dependencies");
    }
    (installed, missing)
}

/// Check macOS permissions (screen recording + accessibility).
/// Returns (screen_ok, a11y_ok).
pub fn check_permissions() -> (bool, bool) {
    let screen_ok = crate::screen::ScreenCapture::has_permission();
    // Accessibility check: try to query System Events
    let a11y_ok = if cfg!(target_os = "macos") {
        Command::new("osascript")
            .args(["-e", "tell application \"System Events\" to get name of first process whose frontmost is true"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status().map(|s| s.success()).unwrap_or(false)
    } else { true };

    if !screen_ok {
        eprintln!("hydra-deps: Screen Recording permission NOT granted");
        eprintln!("hydra-deps: Grant in: System Settings → Privacy & Security → Screen Recording");
    }
    if !a11y_ok {
        eprintln!("hydra-deps: Accessibility permission NOT granted");
        eprintln!("hydra-deps: Grant in: System Settings → Privacy & Security → Accessibility");
    }
    (screen_ok, a11y_ok)
}

/// Full preflight: check deps + permissions. Call at startup.
pub fn preflight() -> bool {
    let (installed, missing) = ensure_deps();
    let (screen_ok, a11y_ok) = check_permissions();
    let ready = screen_ok && a11y_ok;
    if ready {
        eprintln!("hydra-deps: preflight OK (deps: {} installed, {} missing, permissions: all granted)",
            installed, missing);
    } else {
        eprintln!("hydra-deps: preflight INCOMPLETE — some permissions missing");
    }
    ready
}
