//! Tool verifier — confirms a tool is available after installation.

use crate::environment::os_probe::run_cmd;
use super::detector::MissingTool;

/// Verification result for a tool.
#[derive(Debug, Clone, PartialEq)]
pub enum VerifyStatus {
    /// Tool is available and working.
    Available { version: String },
    /// Tool binary exists but returned an error.
    Broken { error: String },
    /// Tool binary not found.
    NotFound,
}

impl VerifyStatus {
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Available { .. })
    }
}

/// Verify a tool is installed and working.
pub fn verify_tool(tool: &MissingTool) -> VerifyStatus {
    // First try `which` / `command -v` to check if binary exists
    let exists = run_cmd("which", &[&tool.name]).is_some();

    if !exists {
        return VerifyStatus::NotFound;
    }

    // Try to get version to confirm it works
    let version_check = try_version(&tool.name);
    match version_check {
        Some(version) => VerifyStatus::Available { version },
        None => {
            // Binary exists but --version failed. Still might be usable.
            VerifyStatus::Available {
                version: "unknown".to_string(),
            }
        }
    }
}

/// Verify a tool by name (convenience function).
pub fn verify_by_name(name: &str) -> VerifyStatus {
    let exists = run_cmd("which", &[name]).is_some();
    if !exists {
        return VerifyStatus::NotFound;
    }
    match try_version(name) {
        Some(version) => VerifyStatus::Available { version },
        None => VerifyStatus::Available {
            version: "unknown".to_string(),
        },
    }
}

/// Try to get a tool's version.
fn try_version(name: &str) -> Option<String> {
    // Try common version flags
    let flags = ["--version", "-version", "version", "-V"];
    for flag in &flags {
        if let Some(output) = run_cmd(name, &[flag]) {
            // Extract version number from output
            let version = extract_version_number(&output);
            if !version.is_empty() {
                return Some(version);
            }
        }
    }
    None
}

/// Extract a version number from output text.
fn extract_version_number(output: &str) -> String {
    let first_line = output.lines().next().unwrap_or("");
    // Find first token that looks like a version
    for word in first_line.split_whitespace() {
        let clean = word.trim_start_matches('v').trim_end_matches(|c: char| c == ',' || c == ')' || c == ';');
        if clean.chars().next().map_or(false, |c| c.is_ascii_digit())
            && clean.contains('.')
        {
            return clean.to_string();
        }
    }
    // If no version-like token, return first line trimmed
    first_line.trim().to_string()
}

/// Format a verification result for display.
pub fn format_verify_result(name: &str, status: &VerifyStatus) -> String {
    match status {
        VerifyStatus::Available { version } => {
            format!("{} is available (version: {})", name, version)
        }
        VerifyStatus::Broken { error } => {
            format!("{} exists but has errors: {}", name, error)
        }
        VerifyStatus::NotFound => {
            format!("{} is not installed", name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::detector::{DetectionSource, ToolEcosystem};

    #[test]
    fn test_verify_installed_tool() {
        // rustc should be installed (we're building with it)
        let status = verify_by_name("rustc");
        assert!(status.is_available(), "rustc should be available");
        if let VerifyStatus::Available { version } = &status {
            assert!(version.contains('.'), "Version should contain dots: {}", version);
        }
    }

    #[test]
    fn test_verify_missing_tool() {
        let status = verify_by_name("nonexistent_tool_xyz_999");
        assert_eq!(status, VerifyStatus::NotFound);
    }

    #[test]
    fn test_verify_tool_struct() {
        let tool = MissingTool {
            name: "rustc".into(),
            source: DetectionSource::ErrorOutput,
            ecosystem: ToolEcosystem::Rust,
        };
        let status = verify_tool(&tool);
        assert!(status.is_available());
    }

    #[test]
    fn test_format_verify_available() {
        let msg = format_verify_result(
            "jq",
            &VerifyStatus::Available { version: "1.7".into() },
        );
        assert!(msg.contains("jq is available"));
        assert!(msg.contains("1.7"));
    }

    #[test]
    fn test_format_verify_not_found() {
        let msg = format_verify_result("jq", &VerifyStatus::NotFound);
        assert!(msg.contains("not installed"));
    }

    #[test]
    fn test_extract_version_number() {
        assert_eq!(extract_version_number("rustc 1.77.0 (aedd 2024)"), "1.77.0");
        assert_eq!(extract_version_number("v20.11.0"), "20.11.0");
        assert_eq!(extract_version_number("Python 3.12.2"), "3.12.2");
    }
}
