//! FileOperations — safe file system operations with constitutional checks.
//! Law 6 (Principal Supremacy): delete requires explicit approval.
//! System files are blocked entirely.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Result of a file operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileResult {
    pub operation: String,
    pub path: String,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub bytes_affected: u64,
}

impl FileResult {
    fn ok(op: &str, path: &str, output: impl Into<String>, bytes: u64) -> Self {
        Self {
            operation: op.into(),
            path: path.into(),
            success: true,
            output: output.into(),
            error: None,
            bytes_affected: bytes,
        }
    }

    fn err(op: &str, path: &str, error: impl Into<String>) -> Self {
        Self {
            operation: op.into(),
            path: path.into(),
            success: false,
            output: String::new(),
            error: Some(error.into()),
            bytes_affected: 0,
        }
    }
}

/// Safe file system operations.
pub struct FileOperations;

impl FileOperations {
    /// Read file contents.
    pub fn read(path: &str) -> FileResult {
        if Self::is_blocked(path) {
            return FileResult::err("read", path, "Path is blocked (system file)");
        }
        match std::fs::read_to_string(path) {
            Ok(content) => {
                let bytes = content.len() as u64;
                FileResult::ok("read", path, content, bytes)
            }
            Err(e) => FileResult::err("read", path, e.to_string()),
        }
    }

    /// Write content to a file.
    pub fn write(path: &str, content: &str) -> FileResult {
        if Self::is_blocked(path) {
            return FileResult::err("write", path, "Path is blocked (system file)");
        }
        // Ensure parent directory exists
        if let Some(parent) = Path::new(path).parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return FileResult::err("write", path, format!("Cannot create parent: {e}"));
            }
        }
        let bytes = content.len() as u64;
        match std::fs::write(path, content) {
            Ok(()) => {
                eprintln!("hydra-executor: wrote {} bytes to {path}", bytes);
                FileResult::ok("write", path, format!("{bytes} bytes written"), bytes)
            }
            Err(e) => FileResult::err("write", path, e.to_string()),
        }
    }

    /// Copy a file or directory.
    pub fn copy(src: &str, dst: &str) -> FileResult {
        if Self::is_blocked(src) || Self::is_blocked(dst) {
            return FileResult::err("copy", src, "Path is blocked");
        }
        let src_path = Path::new(src);
        if src_path.is_dir() {
            return Self::copy_dir_recursive(src, dst);
        }
        match std::fs::copy(src, dst) {
            Ok(bytes) => {
                eprintln!("hydra-executor: copied {src} → {dst} ({bytes} bytes)");
                FileResult::ok("copy", src, format!("Copied to {dst}"), bytes)
            }
            Err(e) => FileResult::err("copy", src, e.to_string()),
        }
    }

    /// Move/rename a file.
    pub fn rename(src: &str, dst: &str) -> FileResult {
        if Self::is_blocked(src) || Self::is_blocked(dst) {
            return FileResult::err("move", src, "Path is blocked");
        }
        match std::fs::rename(src, dst) {
            Ok(()) => {
                eprintln!("hydra-executor: moved {src} → {dst}");
                FileResult::ok("move", src, format!("Moved to {dst}"), 0)
            }
            Err(e) => FileResult::err("move", src, e.to_string()),
        }
    }

    /// Delete a file (constitutional check: requires explicit approval).
    pub fn delete(path: &str, approved: bool) -> FileResult {
        if Self::is_blocked(path) {
            return FileResult::err("delete", path, "Path is blocked (system file)");
        }
        if !approved {
            return FileResult::err(
                "delete",
                path,
                "Delete requires explicit principal approval (Law 6)",
            );
        }
        let p = Path::new(path);
        let result = if p.is_dir() {
            std::fs::remove_dir_all(path)
        } else {
            std::fs::remove_file(path)
        };
        match result {
            Ok(()) => {
                eprintln!("hydra-executor: deleted {path}");
                FileResult::ok("delete", path, "Deleted", 0)
            }
            Err(e) => FileResult::err("delete", path, e.to_string()),
        }
    }

    /// List directory contents.
    pub fn list(dir: &str) -> FileResult {
        match std::fs::read_dir(dir) {
            Ok(entries) => {
                let mut items = Vec::new();
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                    items.push(if is_dir {
                        format!("{name}/")
                    } else {
                        name
                    });
                }
                items.sort();
                let output = items.join("\n");
                let count = items.len() as u64;
                FileResult::ok("list", dir, output, count)
            }
            Err(e) => FileResult::err("list", dir, e.to_string()),
        }
    }

    /// Search for files matching a pattern in a directory.
    pub fn search(dir: &str, pattern: &str) -> FileResult {
        let lower_pattern = pattern.to_lowercase();
        let mut matches = Vec::new();
        Self::search_recursive(Path::new(dir), &lower_pattern, &mut matches, 0);
        let output = matches.join("\n");
        let count = matches.len() as u64;
        FileResult::ok("search", dir, output, count)
    }

    /// Download a file from a URL.
    pub fn download(url: &str, path: &str) -> FileResult {
        if Self::is_blocked(path) {
            return FileResult::err("download", path, "Path is blocked");
        }
        // Use curl for downloads (available on all platforms)
        let output = std::process::Command::new("curl")
            .args(["-sL", "-o", path, url])
            .output();
        match output {
            Ok(o) if o.status.success() => {
                let bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
                eprintln!("hydra-executor: downloaded {url} → {path} ({bytes} bytes)");
                FileResult::ok("download", path, format!("Downloaded from {url}"), bytes)
            }
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                FileResult::err("download", path, format!("curl failed: {stderr}"))
            }
            Err(e) => FileResult::err("download", path, e.to_string()),
        }
    }

    fn is_blocked(path: &str) -> bool {
        let blocked_prefixes = ["/System", "/usr/bin", "/sbin", "/etc/passwd", "/etc/shadow"];
        blocked_prefixes.iter().any(|p| path.starts_with(p))
    }

    fn copy_dir_recursive(src: &str, dst: &str) -> FileResult {
        let output = std::process::Command::new("cp")
            .args(["-r", src, dst])
            .output();
        match output {
            Ok(o) if o.status.success() => {
                FileResult::ok("copy", src, format!("Copied directory to {dst}"), 0)
            }
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                FileResult::err("copy", src, format!("cp -r failed: {stderr}"))
            }
            Err(e) => FileResult::err("copy", src, e.to_string()),
        }
    }

    fn search_recursive(dir: &Path, pattern: &str, matches: &mut Vec<String>, depth: usize) {
        if depth > 10 {
            return;
        }
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                let path = entry.path();
                if name.contains(pattern) {
                    matches.push(path.to_string_lossy().to_string());
                }
                if path.is_dir() && matches.len() < 500 {
                    Self::search_recursive(&path, pattern, matches, depth + 1);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocked_paths_rejected() {
        assert!(FileOperations::is_blocked("/System/Library/test"));
        assert!(FileOperations::is_blocked("/etc/passwd"));
        assert!(!FileOperations::is_blocked("/tmp/test.txt"));
    }

    #[test]
    fn delete_requires_approval() {
        let result = FileOperations::delete("/tmp/nonexistent-hydra-test", false);
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Law 6"));
    }

    #[test]
    fn read_nonexistent_file() {
        let result = FileOperations::read("/tmp/hydra-nonexistent-xyz-99999.txt");
        assert!(!result.success);
    }

    #[test]
    fn write_and_read_roundtrip() {
        let path = format!("/tmp/hydra-test-{}.txt", uuid::Uuid::new_v4());
        let write_result = FileOperations::write(&path, "hello hydra");
        assert!(write_result.success);

        let read_result = FileOperations::read(&path);
        assert!(read_result.success);
        assert_eq!(read_result.output, "hello hydra");

        // Cleanup
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn list_temp_directory() {
        let result = FileOperations::list("/tmp");
        assert!(result.success);
        assert!(result.bytes_affected > 0); // /tmp always has entries
    }
}
