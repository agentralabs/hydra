//! Surgical edit tool — reads files, validates edits, computes diffs,
//! and applies changes with full undo support.
//!
//! Why isn't a sister doing this? This is purely in-memory edit logic
//! with local file I/O. Sisters handle discovery and understanding;
//! this handles the mechanical edit-apply-verify cycle.

use std::path::{Path, PathBuf};

use super::diff_engine;

/// The type of edit to perform on a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditType {
    /// Replace exact match of old_text with new_text.
    Replace,
    /// Insert new_text after old_text.
    InsertAfter,
    /// Delete exact match of old_text.
    Delete,
    /// Append new_text to end of file.
    Append,
    /// Prepend new_text to beginning of file.
    Prepend,
}

/// A single edit operation to be performed on a file.
#[derive(Debug, Clone)]
pub struct EditOperation {
    pub file_path: PathBuf,
    pub edit_type: EditType,
    pub old_text: Option<String>,
    pub new_text: String,
    pub description: String,
}

/// The result of executing an edit (before writing to disk).
#[derive(Debug, Clone)]
pub struct EditResult {
    pub file_path: PathBuf,
    pub diff: String,
    pub lines_added: usize,
    pub lines_removed: usize,
    pub old_content: String,
    pub new_content: String,
    pub applied: bool,
}

/// Errors that can occur during edit operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditError {
    /// File not found at the given path.
    NotFound(String),
    /// old_text matches multiple locations — ambiguous edit.
    Ambiguous(String, usize),
    /// I/O or other operational error.
    IoError(String),
}

impl std::fmt::Display for EditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EditError::NotFound(p) => write!(f, "File not found: {}", p),
            EditError::Ambiguous(t, n) => {
                write!(f, "Ambiguous: '{}' found {} times", t, n)
            }
            EditError::IoError(e) => write!(f, "IO error: {}", e),
        }
    }
}

/// Execute an edit operation: read file, validate, compute new content and diff.
///
/// Returns the result WITHOUT writing to disk. Call `apply_edit` to persist.
pub fn execute_edit(op: EditOperation) -> Result<EditResult, EditError> {
    let old_content = std::fs::read_to_string(&op.file_path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            EditError::NotFound(op.file_path.display().to_string())
        } else {
            EditError::IoError(e.to_string())
        }
    })?;

    let new_content = match op.edit_type {
        EditType::Replace => {
            let old_text = op.old_text.as_deref().ok_or_else(|| {
                EditError::IoError("Replace requires old_text".into())
            })?;
            validate_unique(&old_content, old_text)?;
            old_content.replacen(old_text, &op.new_text, 1)
        }
        EditType::InsertAfter => {
            let old_text = op.old_text.as_deref().ok_or_else(|| {
                EditError::IoError("InsertAfter requires old_text".into())
            })?;
            validate_unique(&old_content, old_text)?;
            let pos = old_content.find(old_text).unwrap();
            let insert_at = pos + old_text.len();
            format!(
                "{}{}{}",
                &old_content[..insert_at],
                &op.new_text,
                &old_content[insert_at..]
            )
        }
        EditType::Delete => {
            let old_text = op.old_text.as_deref().ok_or_else(|| {
                EditError::IoError("Delete requires old_text".into())
            })?;
            validate_unique(&old_content, old_text)?;
            old_content.replacen(old_text, "", 1)
        }
        EditType::Append => {
            format!("{}{}", old_content, op.new_text)
        }
        EditType::Prepend => {
            format!("{}{}", op.new_text, old_content)
        }
    };

    let diff_lines = diff_engine::compute_diff(&old_content, &new_content);
    let (lines_added, lines_removed) = diff_engine::diff_summary(&diff_lines);
    let diff = diff_engine::format_diff_display(&diff_lines);

    Ok(EditResult {
        file_path: op.file_path,
        diff,
        lines_added,
        lines_removed,
        old_content,
        new_content,
        applied: false,
    })
}

/// Write the edit result to disk, persisting the new content.
pub fn apply_edit(result: &EditResult) -> Result<(), String> {
    std::fs::write(&result.file_path, &result.new_content)
        .map_err(|e| format!("Failed to write {}: {}", result.file_path.display(), e))?;
    eprintln!(
        "[hydra:edit] Applied edit to {} (+{} -{})",
        result.file_path.display(),
        result.lines_added,
        result.lines_removed
    );
    Ok(())
}

/// Assess risk level of editing a file based on its path patterns.
///
/// Returns "low", "medium", "high", or "critical".
pub fn risk_level(file_path: &Path) -> &'static str {
    let path_str = file_path.to_string_lossy().to_lowercase();
    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Critical: secrets, credentials, environment files
    if file_name == ".env"
        || file_name.ends_with(".pem")
        || file_name.ends_with(".key")
        || file_name.contains("credentials")
        || file_name.contains("secret")
        || file_name == ".npmrc"
        || file_name == ".pypirc"
    {
        return "critical";
    }

    // High: deployment, CI/CD, infrastructure
    if path_str.contains("deploy")
        || path_str.contains(".github/workflows")
        || path_str.contains("dockerfile")
        || path_str.contains("docker-compose")
        || path_str.contains("terraform")
        || path_str.contains("k8s")
        || path_str.contains("kubernetes")
        || file_name == "cargo.toml"
        || file_name == "package.json"
        || file_name == "makefile"
    {
        return "high";
    }

    // Low: tests, docs, examples
    if path_str.contains("test")
        || path_str.contains("spec")
        || path_str.contains("example")
        || path_str.contains("docs/")
        || file_name.ends_with(".md")
        || file_name.ends_with(".txt")
    {
        return "low";
    }

    // Medium: everything else (src, lib, etc.)
    "medium"
}

/// Validate that `needle` appears exactly once in `haystack`.
fn validate_unique(haystack: &str, needle: &str) -> Result<(), EditError> {
    if needle.is_empty() {
        return Err(EditError::IoError("old_text cannot be empty".into()));
    }
    let count = haystack.matches(needle).count();
    if count == 0 {
        return Err(EditError::NotFound(format!(
            "old_text not found in file: '{}'",
            truncate_for_display(needle, 80)
        )));
    }
    if count > 1 {
        return Err(EditError::Ambiguous(
            truncate_for_display(needle, 80).to_string(),
            count,
        ));
    }
    Ok(())
}

fn truncate_for_display(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        &s[..max]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_file(content: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    #[test]
    fn test_replace_edit() {
        let f = temp_file("hello world\ngoodbye world\n");
        let op = EditOperation {
            file_path: f.path().to_path_buf(),
            edit_type: EditType::Replace,
            old_text: Some("hello world".into()),
            new_text: "hi earth".into(),
            description: "test replace".into(),
        };
        let result = execute_edit(op).unwrap();
        assert!(result.new_content.contains("hi earth"));
        assert!(!result.new_content.contains("hello world"));
        assert!(!result.applied);
    }

    #[test]
    fn test_insert_after_edit() {
        let f = temp_file("line1\nline3\n");
        let op = EditOperation {
            file_path: f.path().to_path_buf(),
            edit_type: EditType::InsertAfter,
            old_text: Some("line1\n".into()),
            new_text: "line2\n".into(),
            description: "test insert".into(),
        };
        let result = execute_edit(op).unwrap();
        assert_eq!(result.new_content, "line1\nline2\nline3\n");
    }

    #[test]
    fn test_delete_edit() {
        let f = temp_file("keep\nremove\nkeep\n");
        let op = EditOperation {
            file_path: f.path().to_path_buf(),
            edit_type: EditType::Delete,
            old_text: Some("remove\n".into()),
            new_text: String::new(),
            description: "test delete".into(),
        };
        let result = execute_edit(op).unwrap();
        assert_eq!(result.new_content, "keep\nkeep\n");
    }

    #[test]
    fn test_append_edit() {
        let f = temp_file("existing\n");
        let op = EditOperation {
            file_path: f.path().to_path_buf(),
            edit_type: EditType::Append,
            old_text: None,
            new_text: "new line\n".into(),
            description: "test append".into(),
        };
        let result = execute_edit(op).unwrap();
        assert_eq!(result.new_content, "existing\nnew line\n");
    }

    #[test]
    fn test_prepend_edit() {
        let f = temp_file("existing\n");
        let op = EditOperation {
            file_path: f.path().to_path_buf(),
            edit_type: EditType::Prepend,
            old_text: None,
            new_text: "header\n".into(),
            description: "test prepend".into(),
        };
        let result = execute_edit(op).unwrap();
        assert_eq!(result.new_content, "header\nexisting\n");
    }

    #[test]
    fn test_apply_edit_writes_to_disk() {
        let f = temp_file("original");
        let op = EditOperation {
            file_path: f.path().to_path_buf(),
            edit_type: EditType::Replace,
            old_text: Some("original".into()),
            new_text: "modified".into(),
            description: "test apply".into(),
        };
        let result = execute_edit(op).unwrap();
        apply_edit(&result).unwrap();
        let on_disk = std::fs::read_to_string(f.path()).unwrap();
        assert_eq!(on_disk, "modified");
    }

    #[test]
    fn test_not_found_error() {
        let op = EditOperation {
            file_path: PathBuf::from("/nonexistent/file.rs"),
            edit_type: EditType::Replace,
            old_text: Some("x".into()),
            new_text: "y".into(),
            description: "test".into(),
        };
        let err = execute_edit(op).unwrap_err();
        assert!(matches!(err, EditError::NotFound(_)));
    }

    #[test]
    fn test_ambiguous_error() {
        let f = temp_file("foo bar foo baz foo");
        let op = EditOperation {
            file_path: f.path().to_path_buf(),
            edit_type: EditType::Replace,
            old_text: Some("foo".into()),
            new_text: "qux".into(),
            description: "test".into(),
        };
        let err = execute_edit(op).unwrap_err();
        match err {
            EditError::Ambiguous(_, count) => assert_eq!(count, 3),
            _ => panic!("Expected Ambiguous error"),
        }
    }

    #[test]
    fn test_risk_levels() {
        assert_eq!(risk_level(Path::new("/project/.env")), "critical");
        assert_eq!(risk_level(Path::new("/keys/server.pem")), "critical");
        assert_eq!(risk_level(Path::new("/deploy/script.sh")), "high");
        assert_eq!(risk_level(Path::new("/.github/workflows/ci.yml")), "high");
        assert_eq!(risk_level(Path::new("/app/Cargo.toml")), "high");
        assert_eq!(risk_level(Path::new("/src/tests/mod.rs")), "low");
        assert_eq!(risk_level(Path::new("/docs/guide.md")), "low");
        assert_eq!(risk_level(Path::new("/src/lib.rs")), "medium");
    }

    #[test]
    fn test_diff_in_result() {
        let f = temp_file("aaa\nbbb\nccc\n");
        let op = EditOperation {
            file_path: f.path().to_path_buf(),
            edit_type: EditType::Replace,
            old_text: Some("bbb".into()),
            new_text: "xxx".into(),
            description: "test diff".into(),
        };
        let result = execute_edit(op).unwrap();
        assert_eq!(result.lines_added, 1);
        assert_eq!(result.lines_removed, 1);
        assert!(result.diff.contains("bbb"));
        assert!(result.diff.contains("xxx"));
    }
}
