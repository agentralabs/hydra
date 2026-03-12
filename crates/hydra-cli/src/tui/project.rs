/// Project detection and file awareness.
///
/// Auto-detects project type by scanning the working directory for
/// marker files (Cargo.toml, package.json, go.mod, pyproject.toml, etc.)
/// and provides project metadata for the TUI sidebar and developer commands.

use std::path::{Path, PathBuf};

/// Detected project kind.
#[derive(Clone, Debug, PartialEq)]
pub enum ProjectKind {
    Rust,
    Node,
    Python,
    Go,
    Unknown,
}

impl ProjectKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Rust => "Rust",
            Self::Node => "Node.js",
            Self::Python => "Python",
            Self::Go => "Go",
            Self::Unknown => "Unknown",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Rust => "🦀",
            Self::Node => "⬢",
            Self::Python => "🐍",
            Self::Go => "⚙",
            Self::Unknown => "📁",
        }
    }

    /// The test command for this project type.
    pub fn test_cmd(&self) -> (&'static str, &'static [&'static str]) {
        match self {
            Self::Rust => ("cargo", &["test"]),
            Self::Node => ("npm", &["test"]),
            Self::Python => ("python", &["-m", "pytest"]),
            Self::Go => ("go", &["test", "./..."]),
            Self::Unknown => ("echo", &["No test command configured"]),
        }
    }

    /// The build command for this project type.
    pub fn build_cmd(&self) -> (&'static str, &'static [&'static str]) {
        match self {
            Self::Rust => ("cargo", &["build"]),
            Self::Node => ("npm", &["run", "build"]),
            Self::Python => ("python", &["-m", "build"]),
            Self::Go => ("go", &["build", "./..."]),
            Self::Unknown => ("echo", &["No build command configured"]),
        }
    }

    /// The run command for this project type.
    pub fn run_cmd(&self) -> (&'static str, &'static [&'static str]) {
        match self {
            Self::Rust => ("cargo", &["run"]),
            Self::Node => ("npm", &["start"]),
            Self::Python => ("python", &["main.py"]),
            Self::Go => ("go", &["run", "."]),
            Self::Unknown => ("echo", &["No run command configured"]),
        }
    }

    /// The lint command for this project type.
    pub fn lint_cmd(&self) -> (&'static str, &'static [&'static str]) {
        match self {
            Self::Rust => ("cargo", &["clippy"]),
            Self::Node => ("npx", &["eslint", "."]),
            Self::Python => ("python", &["-m", "ruff", "check", "."]),
            Self::Go => ("golangci-lint", &["run"]),
            Self::Unknown => ("echo", &["No lint command configured"]),
        }
    }

    /// The format command for this project type.
    pub fn fmt_cmd(&self) -> (&'static str, &'static [&'static str]) {
        match self {
            Self::Rust => ("cargo", &["fmt"]),
            Self::Node => ("npx", &["prettier", "--write", "."]),
            Self::Python => ("python", &["-m", "ruff", "format", "."]),
            Self::Go => ("gofmt", &["-w", "."]),
            Self::Unknown => ("echo", &["No format command configured"]),
        }
    }

    /// The bench command for this project type.
    pub fn bench_cmd(&self) -> (&'static str, &'static [&'static str]) {
        match self {
            Self::Rust => ("cargo", &["bench"]),
            Self::Node => ("npm", &["run", "bench"]),
            Self::Python => ("python", &["-m", "pytest", "--benchmark-only"]),
            Self::Go => ("go", &["test", "-bench=.", "./..."]),
            Self::Unknown => ("echo", &["No bench command configured"]),
        }
    }

    /// The doc command for this project type.
    pub fn doc_cmd(&self) -> (&'static str, &'static [&'static str]) {
        match self {
            Self::Rust => ("cargo", &["doc", "--open"]),
            Self::Node => ("npx", &["typedoc"]),
            Self::Python => ("python", &["-m", "pdoc", "--html", "."]),
            Self::Go => ("godoc", &["-http=:6060"]),
            Self::Unknown => ("echo", &["No doc command configured"]),
        }
    }

    /// The deps command for this project type.
    pub fn deps_cmd(&self) -> (&'static str, &'static [&'static str]) {
        match self {
            Self::Rust => ("cargo", &["tree", "--depth", "1"]),
            Self::Node => ("npm", &["ls", "--depth=0"]),
            Self::Python => ("pip", &["list"]),
            Self::Go => ("go", &["list", "-m", "all"]),
            Self::Unknown => ("echo", &["No deps command configured"]),
        }
    }
}

/// Detected project information.
#[derive(Clone, Debug)]
pub struct ProjectInfo {
    pub kind: ProjectKind,
    pub root: PathBuf,
    pub name: String,
    pub crate_count: Option<usize>,
    pub git_branch: Option<String>,
    pub git_ahead: Option<usize>,
    pub git_behind: Option<usize>,
}

/// Detect the project type from the given directory.
pub fn detect_project(dir: &Path) -> Option<ProjectInfo> {
    if !dir.is_dir() {
        return None;
    }

    let kind = if dir.join("Cargo.toml").exists() {
        ProjectKind::Rust
    } else if dir.join("package.json").exists() {
        ProjectKind::Node
    } else if dir.join("go.mod").exists() {
        ProjectKind::Go
    } else if dir.join("pyproject.toml").exists()
        || dir.join("setup.py").exists()
        || dir.join("requirements.txt").exists()
    {
        ProjectKind::Python
    } else {
        return None;
    };

    let name = detect_project_name(dir, &kind);
    let crate_count = if kind == ProjectKind::Rust {
        count_rust_crates(dir)
    } else {
        None
    };
    let (git_branch, git_ahead, git_behind) = detect_git_info(dir);

    Some(ProjectInfo {
        kind,
        root: dir.to_path_buf(),
        name,
        crate_count,
        git_branch,
        git_ahead,
        git_behind,
    })
}

fn detect_project_name(dir: &Path, kind: &ProjectKind) -> String {
    match kind {
        ProjectKind::Rust => {
            if let Ok(content) = std::fs::read_to_string(dir.join("Cargo.toml")) {
                // Try workspace name first, then package name
                for line in content.lines() {
                    let line = line.trim();
                    if line.starts_with("name") {
                        if let Some(val) = line.split('=').nth(1) {
                            let name = val.trim().trim_matches('"');
                            if !name.is_empty() {
                                return name.to_string();
                            }
                        }
                    }
                }
            }
        }
        ProjectKind::Node => {
            if let Ok(content) = std::fs::read_to_string(dir.join("package.json")) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(name) = json.get("name").and_then(|v| v.as_str()) {
                        return name.to_string();
                    }
                }
            }
        }
        ProjectKind::Go => {
            if let Ok(content) = std::fs::read_to_string(dir.join("go.mod")) {
                if let Some(line) = content.lines().next() {
                    if let Some(module) = line.strip_prefix("module ") {
                        let name = module.rsplit('/').next().unwrap_or(module);
                        return name.to_string();
                    }
                }
            }
        }
        _ => {}
    }

    // Fall back to directory name
    dir.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project")
        .to_string()
}

fn count_rust_crates(dir: &Path) -> Option<usize> {
    let content = std::fs::read_to_string(dir.join("Cargo.toml")).ok()?;
    let count = content
        .lines()
        .filter(|l| {
            let trimmed = l.trim();
            trimmed.starts_with('"') && trimmed.contains("crates/")
        })
        .count();
    if count > 0 {
        Some(count)
    } else {
        None
    }
}

fn detect_git_info(dir: &Path) -> (Option<String>, Option<usize>, Option<usize>) {
    let branch = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(dir)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        });

    let (ahead, behind) = if branch.is_some() {
        let output = std::process::Command::new("git")
            .args(["rev-list", "--left-right", "--count", "HEAD...@{upstream}"])
            .current_dir(dir)
            .output()
            .ok();
        if let Some(o) = output {
            if o.status.success() {
                let s = String::from_utf8_lossy(&o.stdout);
                let parts: Vec<&str> = s.trim().split('\t').collect();
                if parts.len() == 2 {
                    let a = parts[0].parse::<usize>().ok();
                    let b = parts[1].parse::<usize>().ok();
                    (a, b)
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            }
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    (branch, ahead, behind)
}

// File listing, reading, language detection, and git commands
// are in the sibling module `project_files`.
pub use super::project_files::*;
