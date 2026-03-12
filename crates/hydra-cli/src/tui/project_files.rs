/// File listing, reading, language detection, and git commands.
///
/// Extracted from `project.rs` — these utilities operate on files and git
/// rather than project-type detection.

use std::path::Path;

/// Get a file tree listing (limited depth).
pub fn list_files(dir: &Path, max_depth: usize) -> Vec<String> {
    let mut entries = Vec::new();
    list_files_recursive(dir, dir, 0, max_depth, &mut entries);
    entries
}

fn list_files_recursive(
    root: &Path,
    dir: &Path,
    depth: usize,
    max_depth: usize,
    entries: &mut Vec<String>,
) {
    if depth > max_depth {
        return;
    }

    let mut items: Vec<_> = match std::fs::read_dir(dir) {
        Ok(rd) => rd.filter_map(|e| e.ok()).collect(),
        Err(_) => return,
    };
    items.sort_by_key(|e| e.file_name());

    for entry in items {
        let name = entry.file_name().to_string_lossy().to_string();
        // Skip hidden dirs and common build artifacts
        if name.starts_with('.') || name == "target" || name == "node_modules" || name == "__pycache__" {
            continue;
        }

        let rel = entry
            .path()
            .strip_prefix(root)
            .unwrap_or(&entry.path())
            .display()
            .to_string();

        let indent = "  ".repeat(depth);
        let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);

        if is_dir {
            entries.push(format!("{}📁 {}/", indent, name));
            list_files_recursive(root, &entry.path(), depth + 1, max_depth, entries);
        } else {
            entries.push(format!("{}   {}", indent, rel));
        }
    }
}

/// Read a file's content with line numbers.
pub fn read_file_with_lines(path: &Path) -> Result<(String, String), String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;
    let language = detect_language(path);
    Ok((content, language))
}

/// Detect language from file extension.
pub fn detect_language(path: &Path) -> String {
    match path.extension().and_then(|e| e.to_str()) {
        Some("rs") => "Rust",
        Some("js" | "jsx" | "mjs") => "JavaScript",
        Some("ts" | "tsx") => "TypeScript",
        Some("py") => "Python",
        Some("go") => "Go",
        Some("toml") => "TOML",
        Some("json") => "JSON",
        Some("yaml" | "yml") => "YAML",
        Some("md") => "Markdown",
        Some("sh" | "bash" | "zsh") => "Bash",
        Some("html") => "HTML",
        Some("css") => "CSS",
        Some("sql") => "SQL",
        Some("c") => "C",
        Some("cpp" | "cc" | "cxx") => "C++",
        Some("h" | "hpp") => "C",
        Some("java") => "Java",
        Some("rb") => "Ruby",
        Some("swift") => "Swift",
        Some("kt") => "Kotlin",
        _ => "Plain Text",
    }
    .to_string()
}

/// Run `git diff` and return the output.
pub fn git_diff(dir: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["diff"])
        .current_dir(dir)
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        None
    }
}

/// Run `git status --short` and return the output.
pub fn git_status(dir: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["status", "--short", "--branch"])
        .current_dir(dir)
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        None
    }
}

/// Run `git log --oneline -n` and return the output.
pub fn git_log(dir: &Path, count: usize) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["log", "--oneline", "-n", &count.to_string()])
        .current_dir(dir)
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::message::FileChangeKind;

    #[test]
    fn detect_language_rs() {
        assert_eq!(detect_language(Path::new("foo.rs")), "Rust");
    }

    #[test]
    fn detect_language_unknown() {
        assert_eq!(detect_language(Path::new("foo.xyz")), "Plain Text");
    }

    #[test]
    fn file_change_markers() {
        assert_eq!(FileChangeKind::Modified.marker(), "M");
        assert_eq!(FileChangeKind::Added.marker(), "A");
        assert_eq!(FileChangeKind::Deleted.marker(), "D");
    }
}
