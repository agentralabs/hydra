//! Git clone — clones repositories to temp directories.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Result of a clone operation.
#[derive(Debug, Clone)]
pub struct CloneResult {
    pub project_dir: PathBuf,
    pub repo_name: String,
    pub branch: String,
    pub success: bool,
    pub error: Option<String>,
}

/// Extract the repo name from a URL.
/// "https://github.com/user/repo" → "repo"
/// "https://github.com/user/repo.git" → "repo"
pub fn repo_name_from_url(url: &str) -> String {
    let trimmed = url.trim_end_matches('/').trim_end_matches(".git");
    trimmed.rsplit('/').next().unwrap_or("project").to_string()
}

/// Clone a git repository into a temp directory.
/// Returns the path to the cloned project.
pub fn clone_repo(url: &str, base_dir: &Path) -> CloneResult {
    let name = repo_name_from_url(url);
    let target = base_dir.join(&name);

    // If already cloned, just return it
    if target.join(".git").exists() {
        let branch = detect_branch(&target);
        return CloneResult {
            project_dir: target,
            repo_name: name,
            branch,
            success: true,
            error: None,
        };
    }

    let output = Command::new("git")
        .args(["clone", "--depth", "1", url, target.to_str().unwrap_or(".")])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let branch = detect_branch(&target);
            CloneResult {
                project_dir: target,
                repo_name: name,
                branch,
                success: true,
                error: None,
            }
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            CloneResult {
                project_dir: target,
                repo_name: name,
                branch: String::new(),
                success: false,
                error: Some(stderr),
            }
        }
        Err(e) => CloneResult {
            project_dir: target,
            repo_name: name,
            branch: String::new(),
            success: false,
            error: Some(format!("Failed to run git: {}", e)),
        },
    }
}

/// Detect the default branch of a cloned repo.
fn detect_branch(dir: &Path) -> String {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(dir)
        .output();

    match output {
        Ok(out) if out.status.success() => {
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        }
        _ => "main".to_string(),
    }
}

/// Create a temp directory for project execution.
pub fn create_work_dir() -> Result<PathBuf, String> {
    let dir = std::env::temp_dir().join("hydra-projects");
    std::fs::create_dir_all(&dir).map_err(|e| format!("Cannot create work dir: {}", e))?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repo_name_from_url() {
        assert_eq!(repo_name_from_url("https://github.com/user/repo"), "repo");
        assert_eq!(repo_name_from_url("https://github.com/user/repo.git"), "repo");
        assert_eq!(repo_name_from_url("https://github.com/user/repo/"), "repo");
        assert_eq!(repo_name_from_url("git@github.com:user/my-project.git"), "my-project");
    }

    #[test]
    fn test_create_work_dir() {
        let dir = create_work_dir().unwrap();
        assert!(dir.exists());
    }

    #[test]
    fn test_clone_nonexistent_repo() {
        let work_dir = create_work_dir().unwrap();
        let result = clone_repo("https://github.com/nonexistent-user-xyz/nonexistent-repo-abc", &work_dir);
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_repo_name_edge_cases() {
        assert_eq!(repo_name_from_url("repo"), "repo");
        assert_eq!(repo_name_from_url("https://gitlab.com/group/sub/project.git"), "project");
    }
}
