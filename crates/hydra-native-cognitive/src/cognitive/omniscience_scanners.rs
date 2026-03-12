//! Omniscience scanner helpers — extracted from omniscience.rs for compilation performance.
//!
//! Language detection, source file counting, stub scanning (Rust + TS), health scoring.

use std::path::PathBuf;

use super::omniscience::OmniscienceGap;

/// Detect the primary language of a repo from its files.
pub(crate) fn detect_repo_language(path: &PathBuf) -> String {
    if path.join("Cargo.toml").exists() {
        "rust".into()
    } else if path.join("tsconfig.json").exists() {
        "typescript".into()
    } else if path.join("package.json").exists() {
        "javascript".into()
    } else if path.join("pyproject.toml").exists() || path.join("setup.py").exists() {
        "python".into()
    } else {
        "unknown".into()
    }
}

/// Count source files in a repo based on language.
pub(crate) fn count_source_files_in(root: &PathBuf, language: &str) -> usize {
    let extensions: &[&str] = match language {
        "rust" => &["rs"],
        "typescript" => &["ts", "tsx"],
        "javascript" => &["js", "jsx", "mjs"],
        "python" => &["py"],
        _ => &["rs", "ts", "js", "py"],
    };

    fn count_files(dir: &std::path::Path, exts: &[&str]) -> usize {
        let mut count = 0;
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if name != "target" && name != "node_modules" && name != ".git"
                        && name != "dist" && name != "build" && name != "__pycache__"
                        && !name.starts_with('.')
                    {
                        count += count_files(&path, exts);
                    }
                } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if exts.contains(&ext) {
                        count += 1;
                    }
                }
            }
        }
        count
    }

    count_files(root, extensions)
}

/// Scan a Rust repo for todo!()/unimplemented!() stubs.
///
/// Excludes false positives:
/// - `todo!()` inside string literals (template generators, push_str, format!)
/// - `todo!()` inside assert!() macros (test assertions)
/// - Files in tests/ and benches/ directories (test fixtures)
/// - Lines that are comments
pub(crate) fn scan_rust_stubs(root: &PathBuf, repo_name: &str, gaps: &mut Vec<OmniscienceGap>) {
    fn is_false_positive(line: &str, rel_path: &str) -> bool {
        let trimmed = line.trim();

        // Skip test fixtures and benchmarks
        if rel_path.starts_with("tests/") || rel_path.starts_with("benches/") {
            return true;
        }

        // Skip comments
        if trimmed.starts_with("//") || trimmed.starts_with("///") || trimmed.starts_with("//!") {
            return true;
        }

        // Skip string literals: todo!() inside quotes means it's template output
        // Matches: "...todo!()...", push_str("...todo!()..."), format!("...todo!()...")
        if trimmed.contains("\"") {
            // Check if todo!() appears inside a quoted string
            let chars: Vec<char> = trimmed.chars().collect();
            let todo_pattern = "todo!()";
            let unimpl_pattern = "unimplemented!()";

            // Find all positions of todo!()/unimplemented!() and check if they're inside strings
            for pattern in &[todo_pattern, unimpl_pattern] {
                if let Some(pos) = trimmed.find(pattern) {
                    // Count unescaped quotes before this position
                    let mut quotes = 0;
                    let mut prev_was_escape = false;
                    for (i, ch) in chars.iter().enumerate() {
                        if i >= pos { break; }
                        if prev_was_escape {
                            prev_was_escape = false;
                            continue;
                        }
                        if *ch == '\\' {
                            prev_was_escape = true;
                            continue;
                        }
                        if *ch == '"' {
                            quotes += 1;
                        }
                    }
                    // Odd number of quotes before = inside a string literal
                    if quotes % 2 == 1 {
                        return true;
                    }
                }
            }
        }

        // Skip assert!() macros containing todo!()
        if trimmed.contains("assert") && (trimmed.contains("todo!()") || trimmed.contains("unimplemented!()")) {
            return true;
        }

        false
    }

    fn scan_dir(dir: &std::path::Path, root: &std::path::Path, repo_name: &str, gaps: &mut Vec<OmniscienceGap>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if name != "target" && !name.starts_with('.') {
                        scan_dir(&path, root, repo_name, gaps);
                    }
                } else if path.extension().map_or(false, |e| e == "rs") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        let rel_path = path.strip_prefix(root)
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|_| path.display().to_string());

                        for (line_num, line) in content.lines().enumerate() {
                            if (line.contains("todo!()") || line.contains("unimplemented!()"))
                                && !is_false_positive(line, &rel_path)
                            {
                                gaps.push(OmniscienceGap {
                                    repo: repo_name.to_string(),
                                    description: format!("[{}] {}:{} — {}", repo_name, rel_path, line_num + 1, line.trim()),
                                    files: vec![rel_path.clone()],
                                    severity: "critical".into(),
                                    category: "missing_implementation".into(),
                                    suggested_fix: format!("Implement the stub at {}:{}", rel_path, line_num + 1),
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    scan_dir(root, root, repo_name, gaps);
}

/// Scan a TypeScript/JavaScript repo for unimplemented stubs.
///
/// Excludes false positives:
/// - Files in __tests__/, tests/, *.test.ts, *.spec.ts (test fixtures)
/// - Lines inside string literals (template output)
/// - Comments
pub(crate) fn scan_ts_stubs(root: &PathBuf, repo_name: &str, gaps: &mut Vec<OmniscienceGap>) {
    fn is_test_file(rel_path: &str) -> bool {
        rel_path.starts_with("tests/")
            || rel_path.starts_with("__tests__/")
            || rel_path.contains(".test.")
            || rel_path.contains(".spec.")
            || rel_path.starts_with("test/")
    }

    fn scan_dir(dir: &std::path::Path, root: &std::path::Path, repo_name: &str, gaps: &mut Vec<OmniscienceGap>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if name != "node_modules" && name != "dist" && name != ".next"
                        && !name.starts_with('.')
                    {
                        scan_dir(&path, root, repo_name, gaps);
                    }
                } else if path.extension().map_or(false, |e| e == "ts" || e == "tsx" || e == "js") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        let rel_path = path.strip_prefix(root)
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|_| path.display().to_string());

                        // Skip test files entirely
                        if is_test_file(&rel_path) {
                            continue;
                        }

                        for (line_num, line) in content.lines().enumerate() {
                            let trimmed = line.trim();
                            // Skip comments
                            if trimmed.starts_with("//") || trimmed.starts_with("*") || trimmed.starts_with("/*") {
                                continue;
                            }
                            if trimmed.contains("throw new Error") && trimmed.to_lowercase().contains("not implemented") {
                                gaps.push(OmniscienceGap {
                                    repo: repo_name.to_string(),
                                    description: format!("[{}] {}:{} — {}", repo_name, rel_path, line_num + 1, trimmed),
                                    files: vec![rel_path.clone()],
                                    severity: "critical".into(),
                                    category: "missing_implementation".into(),
                                    suggested_fix: format!("Implement the stub at {}:{}", rel_path, line_num + 1),
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    scan_dir(root, root, repo_name, gaps);
}

/// Calculate health score from gaps and file count.
pub(crate) fn calculate_health_score(gaps: &[OmniscienceGap], total_files: usize) -> f64 {
    let total = total_files.max(1) as f64;
    let critical = gaps.iter().filter(|g| g.severity == "critical").count() as f64;
    let high = gaps.iter().filter(|g| g.severity == "high").count() as f64;
    let medium = gaps.iter().filter(|g| g.severity == "medium").count() as f64;
    let penalty = (critical * 10.0 + high * 5.0 + medium * 2.0) / total;
    (1.0 - penalty).max(0.0).min(1.0)
}

/// Detect if user input is an omniscience intent.
pub fn is_omniscience_intent(text: &str) -> bool {
    let lower = text.to_lowercase();
    let patterns = [
        "omniscience", "read your own code", "read yourself",
        "analyze your code", "scan yourself", "full self-analysis",
        "understand your own", "code health", "gap analysis",
        "semantic repair", "deep self-repair", "full scan",
        "scan all sisters", "repair sisters", "fix sisters",
        "scan all repos", "check all systems",
    ];
    patterns.iter().any(|p| lower.contains(p))
}

#[cfg(test)]
#[path = "omniscience_scanners_tests.rs"]
mod omniscience_scanners_tests;
