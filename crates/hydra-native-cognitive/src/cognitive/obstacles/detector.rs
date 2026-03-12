//! Obstacle detector — classifies errors into obstacle patterns.

/// Known obstacle patterns that the resolver can handle.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ObstaclePattern {
    CompilationError,
    TestFailure,
    MissingDependency,
    FileNotFound,
    NetworkError,
    Timeout,
    InvalidConfig,
    PermissionDenied,
    UnknownFramework,
    Unknown,
}

impl ObstaclePattern {
    /// Human-readable label for display.
    pub fn label(&self) -> &'static str {
        match self {
            Self::CompilationError => "Compilation Error",
            Self::TestFailure => "Test Failure",
            Self::MissingDependency => "Missing Dependency",
            Self::FileNotFound => "File Not Found",
            Self::NetworkError => "Network Error",
            Self::Timeout => "Timeout",
            Self::InvalidConfig => "Invalid Configuration",
            Self::PermissionDenied => "Permission Denied",
            Self::UnknownFramework => "Unknown Framework",
            Self::Unknown => "Unknown Obstacle",
        }
    }

    /// Whether this pattern is safe to auto-resolve without user approval.
    pub fn is_auto_resolvable(&self) -> bool {
        matches!(
            self,
            Self::CompilationError
                | Self::MissingDependency
                | Self::FileNotFound
                | Self::Timeout
                | Self::InvalidConfig
                | Self::UnknownFramework
        )
    }
}

/// A detected obstacle with context.
#[derive(Debug, Clone)]
pub struct Obstacle {
    pub pattern: ObstaclePattern,
    pub error_message: String,
    pub source_file: Option<String>,
    pub task_context: String,
}

impl Obstacle {
    /// Create an obstacle from a raw error message and task context.
    pub fn from_error(error: &str, task_context: &str) -> Self {
        let pattern = classify_error(error);
        let source_file = extract_source_file(error);
        Self {
            pattern,
            error_message: error.to_string(),
            source_file,
            task_context: task_context.to_string(),
        }
    }

    /// Unique key for belief storage lookup.
    pub fn belief_key(&self) -> String {
        format!("obstacle:{}:{}", self.pattern.label(), self.fingerprint())
    }

    /// Short fingerprint for deduplication (first significant error line).
    fn fingerprint(&self) -> String {
        let first_line = self.error_message.lines().next().unwrap_or("");
        let clean: String = first_line
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == ' ')
            .take(60)
            .collect();
        clean.trim().to_lowercase().replace(' ', "_")
    }
}

/// Classify an error message into an ObstaclePattern.
pub fn classify_error(error: &str) -> ObstaclePattern {
    let lower = error.to_lowercase();

    // Order matters — more specific patterns first
    if is_compilation_error(&lower) {
        ObstaclePattern::CompilationError
    } else if is_test_failure(&lower) {
        ObstaclePattern::TestFailure
    } else if is_missing_dependency(&lower) {
        ObstaclePattern::MissingDependency
    } else if is_file_not_found(&lower) {
        ObstaclePattern::FileNotFound
    } else if is_permission_denied(&lower) {
        ObstaclePattern::PermissionDenied
    } else if is_timeout(&lower) {
        ObstaclePattern::Timeout
    } else if is_network_error(&lower) {
        ObstaclePattern::NetworkError
    } else if is_invalid_config(&lower) {
        ObstaclePattern::InvalidConfig
    } else if is_unknown_framework(&lower) {
        ObstaclePattern::UnknownFramework
    } else {
        ObstaclePattern::Unknown
    }
}

fn is_compilation_error(s: &str) -> bool {
    s.contains("error[e") // rustc error codes like error[E0433]
        || s.contains("cannot find") && (s.contains("module") || s.contains("type") || s.contains("value"))
        || s.contains("unresolved import")
        || s.contains("mismatched types")
        || s.contains("expected ") && s.contains("found ")
        || s.contains("could not compile")
}

fn is_test_failure(s: &str) -> bool {
    s.contains("test result: failed")
        || s.contains("assertion") && s.contains("failed")
        || s.contains("thread '") && s.contains("panicked")
}

fn is_missing_dependency(s: &str) -> bool {
    s.contains("no matching package")
        || s.contains("could not find") && s.contains("crate")
        || s.contains("unresolved import") && s.contains("::")
        || s.contains("package not found")
        || s.contains("module not found")
}

fn is_file_not_found(s: &str) -> bool {
    s.contains("no such file")
        || s.contains("file not found")
        || s.contains("notfound")
        || s.contains("doesn't exist")
        || s.contains("does not exist") && s.contains("file")
}

fn is_permission_denied(s: &str) -> bool {
    s.contains("permission denied") || s.contains("eacces") || s.contains("eperm")
}

fn is_timeout(s: &str) -> bool {
    s.contains("timed out")
        || s.contains("timeout")
        || s.contains("deadline exceeded")
        || s.contains("elapsed")
}

fn is_network_error(s: &str) -> bool {
    s.contains("connection refused")
        || s.contains("network") && s.contains("error")
        || s.contains("http error")
        || s.contains("builder error")
        || s.contains("dns")
        || s.contains("econnrefused")
}

fn is_invalid_config(s: &str) -> bool {
    s.contains("invalid config")
        || s.contains("configuration error")
        || s.contains("toml") && s.contains("error")
        || s.contains("missing field")
        || s.contains("invalid value")
}

fn is_unknown_framework(s: &str) -> bool {
    s.contains("unknown build system")
        || s.contains("don't know how to build")
        || s.contains("no makefile found") && s.contains("no build")
        || s.contains("unrecognized project type")
        || s.contains("unknown framework")
}

/// Extract a source file path from an error message (if present).
fn extract_source_file(error: &str) -> Option<String> {
    // Rust compiler style: "  --> src/main.rs:42:5"
    for line in error.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("-->") {
            let path = rest.trim().split(':').next()?;
            return Some(path.trim().to_string());
        }
    }
    // Generic path detection: look for .rs, .toml, .ts paths
    for word in error.split_whitespace() {
        let clean = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '/' && c != '.' && c != '_' && c != '-');
        if (clean.ends_with(".rs") || clean.ends_with(".toml") || clean.ends_with(".ts"))
            && clean.contains('/')
        {
            return Some(clean.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_compilation_error() {
        assert_eq!(classify_error("error[E0433]: unresolved import"), ObstaclePattern::CompilationError);
        assert_eq!(classify_error("cannot find type `Foo` in module"), ObstaclePattern::CompilationError);
        assert_eq!(classify_error("could not compile `my-crate`"), ObstaclePattern::CompilationError);
    }

    #[test]
    fn test_classify_test_failure() {
        assert_eq!(classify_error("test result: FAILED. 1 passed; 2 failed"), ObstaclePattern::TestFailure);
        assert_eq!(classify_error("thread 'main' panicked at assertion failed"), ObstaclePattern::TestFailure);
    }

    #[test]
    fn test_classify_missing_dep() {
        assert_eq!(classify_error("no matching package named `serde`"), ObstaclePattern::MissingDependency);
    }

    #[test]
    fn test_classify_file_not_found() {
        assert_eq!(classify_error("No such file or directory: foo.rs"), ObstaclePattern::FileNotFound);
    }

    #[test]
    fn test_classify_network() {
        assert_eq!(classify_error("HTTP error: builder error"), ObstaclePattern::NetworkError);
        assert_eq!(classify_error("connection refused on port 8080"), ObstaclePattern::NetworkError);
    }

    #[test]
    fn test_classify_timeout() {
        assert_eq!(classify_error("request timed out after 30s"), ObstaclePattern::Timeout);
    }

    #[test]
    fn test_classify_invalid_config() {
        assert_eq!(classify_error("TOML parse error at line 5"), ObstaclePattern::InvalidConfig);
    }

    #[test]
    fn test_classify_unknown() {
        assert_eq!(classify_error("something weird happened"), ObstaclePattern::Unknown);
    }

    #[test]
    fn test_obstacle_from_error() {
        let obs = Obstacle::from_error(
            "error[E0433]: cannot find value `x`\n  --> src/main.rs:10:5",
            "building hydra",
        );
        assert_eq!(obs.pattern, ObstaclePattern::CompilationError);
        assert_eq!(obs.source_file.as_deref(), Some("src/main.rs"));
    }

    #[test]
    fn test_belief_key() {
        let obs = Obstacle::from_error("error[E0433]: unresolved import", "task");
        let key = obs.belief_key();
        assert!(key.starts_with("obstacle:Compilation Error:"));
    }

    #[test]
    fn test_auto_resolvable() {
        assert!(ObstaclePattern::CompilationError.is_auto_resolvable());
        assert!(ObstaclePattern::FileNotFound.is_auto_resolvable());
        assert!(!ObstaclePattern::PermissionDenied.is_auto_resolvable());
        assert!(!ObstaclePattern::Unknown.is_auto_resolvable());
    }

    #[test]
    fn test_extract_source_file_rust() {
        assert_eq!(
            extract_source_file("  --> crates/hydra-kernel/src/lib.rs:42:5"),
            Some("crates/hydra-kernel/src/lib.rs".to_string())
        );
    }

    #[test]
    fn test_extract_source_file_generic() {
        assert_eq!(
            extract_source_file("error in crates/foo/src/bar.rs something"),
            Some("crates/foo/src/bar.rs".to_string())
        );
    }
}
