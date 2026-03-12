//! Sister codebase analyzer — identifies language, structure, tests, and limitations.

use std::fmt;
use std::path::{Path, PathBuf};

use super::verifier::TestResults;

/// Language of the sister project.
#[derive(Debug, Clone, PartialEq)]
pub enum SisterLanguage {
    Rust,
    TypeScript,
    Python,
    Go,
    Unknown(String),
}

impl fmt::Display for SisterLanguage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rust => write!(f, "Rust"),
            Self::TypeScript => write!(f, "TypeScript"),
            Self::Python => write!(f, "Python"),
            Self::Go => write!(f, "Go"),
            Self::Unknown(s) => write!(f, "{}", s),
        }
    }
}

/// Analysis of a sister project's structure.
#[derive(Debug, Clone)]
pub struct SisterAnalysis {
    pub language: SisterLanguage,
    pub project_name: String,
    pub source_files: Vec<PathBuf>,
    pub test_files: Vec<PathBuf>,
    pub has_ci: bool,
    pub test_command: String,
    pub build_command: String,
    pub doc_files: Vec<PathBuf>,
}

impl SisterAnalysis {
    pub fn summary(&self) -> String {
        format!(
            "{} project '{}': {} source files, {} test files",
            self.language, self.project_name,
            self.source_files.len(), self.test_files.len()
        )
    }
}

/// Analyze a sister project at the given path.
pub fn analyze_sister(path: &Path) -> Result<SisterAnalysis, String> {
    if !path.exists() {
        return Err(format!("Sister path does not exist: {}", path.display()));
    }

    let language = detect_language(path);
    let project_name = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let (test_command, build_command) = match &language {
        SisterLanguage::Rust => ("cargo test -j 1".into(), "cargo check -j 1".into()),
        SisterLanguage::TypeScript => ("npm test".into(), "npm run build".into()),
        SisterLanguage::Python => ("pytest".into(), "python -m py_compile".into()),
        SisterLanguage::Go => ("go test ./...".into(), "go build ./...".into()),
        SisterLanguage::Unknown(_) => ("make test".into(), "make".into()),
    };

    let source_files = collect_source_files(path, &language);
    let test_files = collect_test_files(path, &language);
    let doc_files = collect_doc_files(path);
    let has_ci = path.join(".github/workflows").exists()
        || path.join(".gitlab-ci.yml").exists();

    Ok(SisterAnalysis {
        language,
        project_name,
        source_files,
        test_files,
        has_ci,
        test_command,
        build_command,
        doc_files,
    })
}

/// Detect the primary language of a project.
fn detect_language(path: &Path) -> SisterLanguage {
    if path.join("Cargo.toml").exists() {
        SisterLanguage::Rust
    } else if path.join("package.json").exists() {
        SisterLanguage::TypeScript
    } else if path.join("pyproject.toml").exists() || path.join("setup.py").exists() {
        SisterLanguage::Python
    } else if path.join("go.mod").exists() {
        SisterLanguage::Go
    } else {
        SisterLanguage::Unknown("unknown".into())
    }
}

/// Collect source files for a language.
fn collect_source_files(path: &Path, lang: &SisterLanguage) -> Vec<PathBuf> {
    let extensions = match lang {
        SisterLanguage::Rust => vec!["rs"],
        SisterLanguage::TypeScript => vec!["ts", "tsx"],
        SisterLanguage::Python => vec!["py"],
        SisterLanguage::Go => vec!["go"],
        SisterLanguage::Unknown(_) => vec![],
    };
    collect_files_with_ext(path, &extensions, false)
}

/// Collect test files for a language.
fn collect_test_files(path: &Path, lang: &SisterLanguage) -> Vec<PathBuf> {
    let extensions = match lang {
        SisterLanguage::Rust => vec!["rs"],
        SisterLanguage::TypeScript => vec!["ts", "tsx"],
        SisterLanguage::Python => vec!["py"],
        SisterLanguage::Go => vec!["go"],
        SisterLanguage::Unknown(_) => vec![],
    };
    collect_files_with_ext(path, &extensions, true)
}

/// Collect documentation files.
fn collect_doc_files(path: &Path) -> Vec<PathBuf> {
    let mut docs = Vec::new();
    for name in &["README.md", "CHANGELOG.md", "CONTRIBUTING.md", "docs"] {
        let p = path.join(name);
        if p.exists() {
            docs.push(p);
        }
    }
    docs
}

fn collect_files_with_ext(path: &Path, extensions: &[&str], tests_only: bool) -> Vec<PathBuf> {
    let mut results = Vec::new();
    walk_dir_recursive(path, extensions, tests_only, &mut results, 0);
    results
}

fn walk_dir_recursive(
    dir: &Path, extensions: &[&str], tests_only: bool,
    results: &mut Vec<PathBuf>, depth: usize,
) {
    if depth > 6 { return; }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let p = entry.path();
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        // Skip build/vendor/hidden directories
        if p.is_dir() {
            if name == "target" || name == "node_modules" || name == "vendor"
                || name == ".git" {
                continue;
            }
            walk_dir_recursive(&p, extensions, tests_only, results, depth + 1);
            continue;
        }
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !extensions.contains(&ext) { continue; }
        let path_str = p.to_string_lossy();
        let is_test = name.contains("test") || name.contains("spec")
            || path_str.contains("/tests/") || name.ends_with("_test.go");
        if tests_only == is_test {
            results.push(p);
        }
    }
}

/// Identify a specific limitation based on the goal and analysis.
pub fn identify_limitation(
    analysis: &SisterAnalysis,
    goal: &str,
    baseline: &TestResults,
) -> String {
    let goal_lower = goal.to_lowercase();

    // If goal is "auto-detect", find something concrete
    if goal_lower.contains("auto") {
        if baseline.fail_count > 0 {
            return format!(
                "Fix {} failing tests in the test suite",
                baseline.fail_count
            );
        }
        if analysis.test_files.is_empty() {
            return "Add test coverage — no test files found".to_string();
        }
        if !analysis.has_ci {
            return "Add CI configuration for automated testing".to_string();
        }
        if analysis.doc_files.is_empty() {
            return "Add README documentation".to_string();
        }
        return String::new(); // No obvious limitation
    }

    // Use the goal directly as the limitation description
    goal.to_string()
}
