//! Fix verifier — confirms an obstacle is actually resolved after applying a fix.

use super::detector::ObstaclePattern;

/// Verification result.
#[derive(Debug, Clone, PartialEq)]
pub enum VerifyResult {
    /// The fix resolved the obstacle.
    Passed,
    /// The fix didn't resolve it — same or similar error.
    SameError,
    /// The fix introduced a new error.
    NewError(String),
    /// Verification couldn't be performed (e.g., no test to run).
    Inconclusive,
}

impl VerifyResult {
    pub fn is_passed(&self) -> bool {
        matches!(self, Self::Passed)
    }
}

/// Determine the verification command for an obstacle pattern.
pub fn verification_command(pattern: &ObstaclePattern, source_file: Option<&str>) -> Option<String> {
    match pattern {
        ObstaclePattern::CompilationError => {
            // Try to verify just the affected crate
            if let Some(path) = source_file {
                if let Some(crate_name) = extract_crate_name(path) {
                    return Some(format!("cargo check -p {} -j 1 2>&1", crate_name));
                }
            }
            Some("cargo check -j 1 2>&1".to_string())
        }
        ObstaclePattern::TestFailure => {
            if let Some(path) = source_file {
                if let Some(crate_name) = extract_crate_name(path) {
                    return Some(format!("cargo test -p {} -j 1 2>&1", crate_name));
                }
            }
            Some("cargo test -j 1 2>&1".to_string())
        }
        ObstaclePattern::MissingDependency => {
            Some("cargo check -j 1 2>&1".to_string())
        }
        ObstaclePattern::FileNotFound => {
            if let Some(path) = source_file {
                Some(format!("test -f {} && echo 'EXISTS' || echo 'NOT FOUND'", path))
            } else {
                None
            }
        }
        ObstaclePattern::InvalidConfig => {
            // Try cargo check to verify config
            Some("cargo check -j 1 2>&1".to_string())
        }
        // Network, Timeout, Permission — can't verify with a simple command
        _ => None,
    }
}

/// Check if verification output indicates success.
pub fn check_verification_output(
    pattern: &ObstaclePattern,
    output: &str,
    original_error: &str,
) -> VerifyResult {
    let lower = output.to_lowercase();

    match pattern {
        ObstaclePattern::CompilationError | ObstaclePattern::MissingDependency => {
            if lower.contains("error[e") || lower.contains("could not compile") {
                // Check if it's the same error or a new one
                if has_overlap(output, original_error) {
                    VerifyResult::SameError
                } else {
                    VerifyResult::NewError(first_error_line(output))
                }
            } else {
                VerifyResult::Passed
            }
        }
        ObstaclePattern::TestFailure => {
            if lower.contains("test result: failed") || lower.contains("panicked") {
                if has_overlap(output, original_error) {
                    VerifyResult::SameError
                } else {
                    VerifyResult::NewError(first_error_line(output))
                }
            } else {
                VerifyResult::Passed
            }
        }
        ObstaclePattern::FileNotFound => {
            if lower.contains("exists") {
                VerifyResult::Passed
            } else {
                VerifyResult::SameError
            }
        }
        ObstaclePattern::InvalidConfig => {
            if lower.contains("error") {
                VerifyResult::SameError
            } else {
                VerifyResult::Passed
            }
        }
        _ => VerifyResult::Inconclusive,
    }
}

/// Extract crate name from a file path like "crates/hydra-kernel/src/lib.rs".
fn extract_crate_name(path: &str) -> Option<String> {
    let parts: Vec<&str> = path.split('/').collect();
    for (i, part) in parts.iter().enumerate() {
        if *part == "crates" {
            if let Some(name) = parts.get(i + 1) {
                return Some(name.to_string());
            }
        }
    }
    None
}

/// Check if two error outputs share significant error lines.
fn has_overlap(output: &str, original: &str) -> bool {
    let orig_errors: Vec<&str> = original
        .lines()
        .filter(|l| l.contains("error") || l.contains("panicked"))
        .take(3)
        .collect();

    for orig_line in &orig_errors {
        // Extract the core error message (skip location info)
        let core = orig_line
            .split("error")
            .nth(1)
            .unwrap_or(orig_line)
            .trim_start_matches(|c: char| c == '[' || c == ']' || c.is_alphanumeric())
            .trim();
        if core.len() > 10 && output.contains(core) {
            return true;
        }
    }
    false
}

/// Get the first error line from output.
fn first_error_line(output: &str) -> String {
    output
        .lines()
        .find(|l| l.to_lowercase().contains("error"))
        .unwrap_or("unknown error")
        .to_string()
}

/// Format a verification report for display.
pub fn verification_report(
    pattern: &ObstaclePattern,
    result: &VerifyResult,
    strategy_desc: &str,
) -> String {
    match result {
        VerifyResult::Passed => {
            format!("Fix verified: {} resolved {} successfully.", strategy_desc, pattern.label())
        }
        VerifyResult::SameError => {
            format!("Fix did not resolve {}: same error persists.", pattern.label())
        }
        VerifyResult::NewError(e) => {
            format!("Fix for {} introduced a new error: {}", pattern.label(), e)
        }
        VerifyResult::Inconclusive => {
            format!("Could not automatically verify fix for {}. Manual check recommended.", pattern.label())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_crate_name() {
        assert_eq!(
            extract_crate_name("crates/hydra-kernel/src/lib.rs"),
            Some("hydra-kernel".to_string())
        );
        assert_eq!(extract_crate_name("src/main.rs"), None);
    }

    #[test]
    fn test_verification_command_compilation() {
        let cmd = verification_command(
            &ObstaclePattern::CompilationError,
            Some("crates/hydra-kernel/src/lib.rs"),
        );
        assert!(cmd.unwrap().contains("cargo check -p hydra-kernel"));
    }

    #[test]
    fn test_verification_command_file_not_found() {
        let cmd = verification_command(
            &ObstaclePattern::FileNotFound,
            Some("src/missing.rs"),
        );
        assert!(cmd.unwrap().contains("test -f"));
    }

    #[test]
    fn test_verification_command_network_none() {
        let cmd = verification_command(&ObstaclePattern::NetworkError, None);
        assert!(cmd.is_none());
    }

    #[test]
    fn test_check_compilation_passed() {
        let result = check_verification_output(
            &ObstaclePattern::CompilationError,
            "Finished dev profile [unoptimized] target(s) in 1.5s",
            "error[E0433]: unresolved import",
        );
        assert_eq!(result, VerifyResult::Passed);
    }

    #[test]
    fn test_check_compilation_same_error() {
        let original = "error[E0433]: unresolved import `foo::bar`";
        let output = "error[E0433]: unresolved import `foo::bar`\n  --> src/lib.rs:5";
        let result = check_verification_output(
            &ObstaclePattern::CompilationError,
            output,
            original,
        );
        assert_eq!(result, VerifyResult::SameError);
    }

    #[test]
    fn test_check_file_exists() {
        let result = check_verification_output(
            &ObstaclePattern::FileNotFound,
            "EXISTS",
            "file not found",
        );
        assert_eq!(result, VerifyResult::Passed);
    }

    #[test]
    fn test_verification_report() {
        let report = verification_report(
            &ObstaclePattern::CompilationError,
            &VerifyResult::Passed,
            "add missing import",
        );
        assert!(report.contains("resolved"));
        assert!(report.contains("Compilation Error"));
    }

    #[test]
    fn test_verify_result_is_passed() {
        assert!(VerifyResult::Passed.is_passed());
        assert!(!VerifyResult::SameError.is_passed());
        assert!(!VerifyResult::NewError("x".into()).is_passed());
        assert!(!VerifyResult::Inconclusive.is_passed());
    }
}
