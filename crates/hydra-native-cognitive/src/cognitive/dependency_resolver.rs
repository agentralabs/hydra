//! Dynamic dependency resolution — adapts to discovered requirements during tasks.
//!
//! UCU Module #4 (Wave 5). When a tool/command fails because something is missing,
//! this module identifies what's needed and suggests or performs the install.
//! Sister-first: uses Reality sister for environment checks, Codebase for deps.

use std::sync::Arc;
use crate::sisters::SisterGateway;

/// Resolution result for a missing dependency.
#[derive(Debug, Clone)]
pub struct Resolution {
    pub resolved: bool,
    pub action_taken: ResolutionAction,
    pub description: String,
}

/// What action was taken (or suggested) to resolve the dependency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionAction {
    /// Already available — no action needed.
    AlreadyAvailable,
    /// Suggested install command.
    SuggestInstall(String),
    /// Dependency cannot be resolved automatically.
    CannotResolve,
    /// Need user input to proceed.
    NeedUserInput(String),
}

/// Analyze a failure error to detect missing dependencies.
pub fn detect_missing_dependency(error: &str) -> Option<MissingDependency> {
    let lower = error.to_lowercase();

    // Rust crate missing
    if lower.contains("can't find crate") || lower.contains("unresolved import") {
        let crate_name = extract_crate_name(error);
        return Some(MissingDependency {
            kind: DependencyKind::RustCrate,
            name: crate_name,
            error_excerpt: error[..error.len().min(120)].to_string(),
        });
    }

    // Python package missing
    if lower.contains("no module named") || lower.contains("modulenotfounderror") {
        let module = extract_after(error, "No module named ");
        return Some(MissingDependency {
            kind: DependencyKind::PythonPackage,
            name: module,
            error_excerpt: error[..error.len().min(120)].to_string(),
        });
    }

    // Node.js module missing
    if lower.contains("cannot find module") || lower.contains("module not found") {
        let module = extract_after(error, "Cannot find module ");
        return Some(MissingDependency {
            kind: DependencyKind::NpmPackage,
            name: module,
            error_excerpt: error[..error.len().min(120)].to_string(),
        });
    }

    // System command missing
    if lower.contains("command not found") || lower.contains("not recognized") {
        let cmd = extract_before(error, ": command not found");
        return Some(MissingDependency {
            kind: DependencyKind::SystemCommand,
            name: cmd,
            error_excerpt: error[..error.len().min(120)].to_string(),
        });
    }

    // Shared library missing
    if lower.contains("cannot open shared object") || lower.contains("library not loaded") {
        let lib = extract_after(error, "cannot open shared object file");
        return Some(MissingDependency {
            kind: DependencyKind::SystemLibrary,
            name: lib,
            error_excerpt: error[..error.len().min(120)].to_string(),
        });
    }

    None
}

/// Suggest a resolution for a missing dependency.
pub fn suggest_resolution(dep: &MissingDependency) -> Resolution {
    match dep.kind {
        DependencyKind::RustCrate => {
            if dep.name.is_empty() {
                Resolution {
                    resolved: false,
                    action_taken: ResolutionAction::NeedUserInput("Which crate is needed?".into()),
                    description: "Cannot determine crate name from error".into(),
                }
            } else {
                Resolution {
                    resolved: false,
                    action_taken: ResolutionAction::SuggestInstall(
                        format!("cargo add {}", dep.name)),
                    description: format!("Rust crate '{}' not found — add it to Cargo.toml", dep.name),
                }
            }
        }
        DependencyKind::PythonPackage => Resolution {
            resolved: false,
            action_taken: ResolutionAction::SuggestInstall(
                format!("pip install {}", dep.name)),
            description: format!("Python package '{}' not installed", dep.name),
        },
        DependencyKind::NpmPackage => Resolution {
            resolved: false,
            action_taken: ResolutionAction::SuggestInstall(
                format!("npm install {}", dep.name)),
            description: format!("Node package '{}' not installed", dep.name),
        },
        DependencyKind::SystemCommand => Resolution {
            resolved: false,
            action_taken: ResolutionAction::SuggestInstall(
                suggest_system_install(&dep.name)),
            description: format!("Command '{}' not found", dep.name),
        },
        DependencyKind::SystemLibrary => Resolution {
            resolved: false,
            action_taken: ResolutionAction::NeedUserInput(
                format!("System library missing: {}. Install via your package manager.", dep.name)),
            description: format!("Shared library '{}' not found", dep.name),
        },
    }
}

/// Try to resolve a dependency using sisters (sister-first).
pub async fn resolve_with_sisters(
    dep: &MissingDependency,
    gateway: &Option<Arc<SisterGateway>>,
) -> Resolution {
    // Try Reality sister for environment info
    if let Some(ref _gw) = gateway {
        // Gateway can check if tool/command is available via Reality sister
        // For now, fall back to static resolution
        eprintln!("[hydra:deps] Checking dependency '{}' ({:?})", dep.name, dep.kind);
    }
    suggest_resolution(dep)
}

/// A detected missing dependency.
#[derive(Debug, Clone)]
pub struct MissingDependency {
    pub kind: DependencyKind,
    pub name: String,
    pub error_excerpt: String,
}

/// Category of dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyKind {
    RustCrate,
    PythonPackage,
    NpmPackage,
    SystemCommand,
    SystemLibrary,
}

// ── Extraction helpers ──

fn extract_crate_name(error: &str) -> String {
    // "can't find crate for `serde`" → "serde"
    if let Some(start) = error.find('`') {
        if let Some(end) = error[start + 1..].find('`') {
            return error[start + 1..start + 1 + end].to_string();
        }
    }
    // "unresolved import `tokio`" → "tokio"
    if let Some(pos) = error.find("import `") {
        let after = &error[pos + 8..];
        if let Some(end) = after.find('`') {
            return after[..end].split("::").next().unwrap_or("").to_string();
        }
    }
    String::new()
}

fn extract_after(error: &str, marker: &str) -> String {
    if let Some(pos) = error.find(marker) {
        let after = error[pos + marker.len()..].trim();
        let end = after.find(|c: char| c.is_whitespace() || c == '\'' || c == '"')
            .unwrap_or(after.len());
        return after[..end].trim_matches(|c| c == '\'' || c == '"').to_string();
    }
    String::new()
}

fn extract_before(error: &str, marker: &str) -> String {
    if let Some(pos) = error.find(marker) {
        let before = error[..pos].trim();
        let start = before.rfind(|c: char| c.is_whitespace() || c == ':' || c == '\n')
            .map(|i| i + 1).unwrap_or(0);
        return before[start..].to_string();
    }
    String::new()
}

fn suggest_system_install(cmd: &str) -> String {
    match cmd {
        "git" => "brew install git  # or: apt install git".into(),
        "node" | "npm" => "brew install node  # or: apt install nodejs npm".into(),
        "python" | "python3" => "brew install python  # or: apt install python3".into(),
        "docker" => "brew install --cask docker  # or: apt install docker.io".into(),
        "curl" => "brew install curl  # or: apt install curl".into(),
        "jq" => "brew install jq  # or: apt install jq".into(),
        _ => format!("brew install {}  # or: apt install {}", cmd, cmd),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_rust_crate() {
        let dep = detect_missing_dependency("error: can't find crate for `serde`");
        assert!(dep.is_some());
        let dep = dep.unwrap();
        assert_eq!(dep.kind, DependencyKind::RustCrate);
        assert_eq!(dep.name, "serde");
    }

    #[test]
    fn test_detect_python() {
        let dep = detect_missing_dependency("ModuleNotFoundError: No module named 'requests'");
        assert!(dep.is_some());
        assert_eq!(dep.unwrap().kind, DependencyKind::PythonPackage);
    }

    #[test]
    fn test_detect_command() {
        let dep = detect_missing_dependency("jq: command not found");
        assert!(dep.is_some());
        let dep = dep.unwrap();
        assert_eq!(dep.kind, DependencyKind::SystemCommand);
        assert_eq!(dep.name, "jq");
    }

    #[test]
    fn test_suggest_rust() {
        let dep = MissingDependency {
            kind: DependencyKind::RustCrate, name: "tokio".into(), error_excerpt: "".into(),
        };
        let res = suggest_resolution(&dep);
        assert_eq!(res.action_taken, ResolutionAction::SuggestInstall("cargo add tokio".into()));
    }

    #[test]
    fn test_no_dependency_normal_error() {
        assert!(detect_missing_dependency("type mismatch: expected i32").is_none());
    }

    #[test]
    fn test_system_install_known() {
        let cmd = suggest_system_install("jq");
        assert!(cmd.contains("jq"));
    }
}
