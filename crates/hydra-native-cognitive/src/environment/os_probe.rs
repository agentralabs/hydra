//! OS probe — detects OS, architecture, kernel, hostname, user, shell.

use std::process::Command;

/// Operating system information.
#[derive(Debug, Clone)]
pub struct OsInfo {
    pub name: String,       // "macOS", "Linux", "Windows"
    pub version: String,    // "14.3", "22.04", "11"
    pub arch: String,       // "arm64", "x86_64"
    pub kernel: String,     // "Darwin 23.3.0", "Linux 6.1.0"
}

/// Detect OS information.
pub fn probe_os() -> OsInfo {
    let (name, version) = detect_os_name_version();
    let arch = run_cmd("uname", &["-m"]).unwrap_or_else(|| std::env::consts::ARCH.to_string());
    let kernel = run_cmd("uname", &["-sr"]).unwrap_or_else(|| "unknown".to_string());

    OsInfo { name, version, arch, kernel }
}

/// Detect hostname.
pub fn probe_hostname() -> String {
    run_cmd("hostname", &[])
        .or_else(|| std::env::var("HOSTNAME").ok())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Detect current user.
pub fn probe_user() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| run_cmd("whoami", &[]).unwrap_or_else(|| "unknown".to_string()))
}

/// Detect current shell.
pub fn probe_shell() -> String {
    std::env::var("SHELL")
        .map(|s| {
            s.rsplit('/').next().unwrap_or(&s).to_string()
        })
        .unwrap_or_else(|_| "unknown".to_string())
}

fn detect_os_name_version() -> (String, String) {
    match std::env::consts::OS {
        "macos" => {
            let ver = run_cmd("sw_vers", &["-productVersion"]).unwrap_or_default();
            ("macOS".to_string(), ver)
        }
        "linux" => {
            // Try /etc/os-release first
            if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
                let name = extract_field(&content, "PRETTY_NAME=")
                    .or_else(|| extract_field(&content, "NAME="))
                    .unwrap_or_else(|| "Linux".to_string());
                let ver = extract_field(&content, "VERSION_ID=").unwrap_or_default();
                return (name, ver);
            }
            ("Linux".to_string(), run_cmd("uname", &["-r"]).unwrap_or_default())
        }
        "windows" => {
            ("Windows".to_string(), run_cmd("ver", &[]).unwrap_or_default())
        }
        other => (other.to_string(), String::new()),
    }
}

fn extract_field(content: &str, key: &str) -> Option<String> {
    content.lines()
        .find(|l| l.starts_with(key))
        .map(|l| l[key.len()..].trim().trim_matches('"').to_string())
}

/// Run a command and capture stdout (trimmed). Returns None on failure.
pub(crate) fn run_cmd(cmd: &str, args: &[&str]) -> Option<String> {
    Command::new(cmd)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_probe_os() {
        let info = probe_os();
        assert!(!info.name.is_empty());
        assert!(!info.arch.is_empty());
        assert!(!info.kernel.is_empty());
    }

    #[test]
    fn test_probe_hostname() {
        let h = probe_hostname();
        assert!(!h.is_empty());
    }

    #[test]
    fn test_probe_user() {
        let u = probe_user();
        assert!(!u.is_empty());
    }

    #[test]
    fn test_probe_shell() {
        let s = probe_shell();
        assert!(!s.is_empty());
    }

    #[test]
    fn test_run_cmd_success() {
        assert!(run_cmd("echo", &["hello"]).is_some());
    }

    #[test]
    fn test_run_cmd_failure() {
        assert!(run_cmd("nonexistent_command_xyz", &[]).is_none());
    }
}
