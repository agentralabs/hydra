//! Doc reader — finds and reads documentation files in a project.

use std::path::{Path, PathBuf};

/// A discovered documentation file.
#[derive(Debug, Clone)]
pub struct DocFile {
    pub path: PathBuf,
    pub kind: DocKind,
    pub priority: u8, // lower = higher priority
}

/// Type of documentation file.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DocKind {
    Readme,
    ApiDocs,
    Contributing,
    Changelog,
    CargoToml,
    PackageJson,
    Makefile,
    DocsDir,
    Other,
}

impl DocFile {
    pub fn is_readme(&self) -> bool {
        self.kind == DocKind::Readme
    }

    pub fn is_build_config(&self) -> bool {
        matches!(self.kind, DocKind::CargoToml | DocKind::PackageJson | DocKind::Makefile)
    }
}

/// Find documentation files in a project, ordered by priority.
pub fn find_docs(project_root: &Path) -> Vec<DocFile> {
    let candidates: Vec<(&str, DocKind, u8)> = vec![
        ("README.md", DocKind::Readme, 1),
        ("README", DocKind::Readme, 2),
        ("readme.md", DocKind::Readme, 2),
        ("Cargo.toml", DocKind::CargoToml, 3),
        ("package.json", DocKind::PackageJson, 3),
        ("pyproject.toml", DocKind::CargoToml, 3),
        ("go.mod", DocKind::CargoToml, 3),
        ("docs/README.md", DocKind::DocsDir, 4),
        ("doc/README.md", DocKind::DocsDir, 4),
        ("API.md", DocKind::ApiDocs, 5),
        ("USAGE.md", DocKind::ApiDocs, 5),
        ("GUIDE.md", DocKind::ApiDocs, 5),
        ("CONTRIBUTING.md", DocKind::Contributing, 6),
        ("Makefile", DocKind::Makefile, 7),
        ("justfile", DocKind::Makefile, 7),
        ("Taskfile.yml", DocKind::Makefile, 7),
        ("CHANGELOG.md", DocKind::Changelog, 8),
    ];

    let mut docs = Vec::new();
    for (name, kind, priority) in candidates {
        let path = project_root.join(name);
        if path.exists() {
            docs.push(DocFile { path, kind, priority });
        }
    }

    // Also scan docs/ directory for .md files
    let docs_dir = project_root.join("docs");
    if docs_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&docs_dir) {
            for entry in entries.flatten().take(10) {
                let p = entry.path();
                if p.extension().map_or(false, |e| e == "md") {
                    // Skip if already added
                    if !docs.iter().any(|d| d.path == p) {
                        docs.push(DocFile {
                            path: p,
                            kind: DocKind::DocsDir,
                            priority: 4,
                        });
                    }
                }
            }
        }
    }

    docs.sort_by_key(|d| d.priority);
    docs
}

/// Read a doc file and extract the most relevant content.
/// Truncates to max_chars, keeping headings and important sections.
pub fn extract_relevant(doc: &DocFile, max_chars: usize) -> String {
    let content = match std::fs::read_to_string(&doc.path) {
        Ok(c) => c,
        Err(_) => return String::new(),
    };

    if content.len() <= max_chars {
        return content;
    }

    // Smart truncation: prioritize certain sections
    match doc.kind {
        DocKind::Readme | DocKind::ApiDocs | DocKind::DocsDir => {
            extract_key_sections(&content, max_chars)
        }
        DocKind::CargoToml | DocKind::PackageJson => {
            // For config files, just take the beginning
            truncate_to(&content, max_chars)
        }
        DocKind::Makefile => {
            // For Makefiles, extract target names
            extract_makefile_targets(&content, max_chars)
        }
        _ => truncate_to(&content, max_chars),
    }
}

/// Extract key sections from a markdown document.
fn extract_key_sections(content: &str, max_chars: usize) -> String {
    let priority_headers = [
        "install", "setup", "getting started", "quick start",
        "usage", "api", "commands", "configuration", "config",
        "running", "test", "build", "development", "requirements",
        "prerequisites", "dependencies",
    ];
    let skip_headers = [
        "license", "changelog", "contributing", "authors",
        "acknowledgment", "credits", "faq",
    ];

    let mut result = String::new();
    let mut current_section = String::new();
    let mut current_header = String::new();
    let mut in_priority_section = false;

    for line in content.lines() {
        if line.starts_with('#') {
            // Save previous section if it was priority
            if in_priority_section && !current_section.is_empty() {
                result.push_str(&current_section);
                result.push('\n');
            }
            current_section.clear();
            current_header = line.to_lowercase();

            let is_skip = skip_headers.iter().any(|h| current_header.contains(h));
            in_priority_section = !is_skip
                && (priority_headers.iter().any(|h| current_header.contains(h))
                    || line.starts_with("# ")); // Always include top-level heading
        }

        if in_priority_section {
            current_section.push_str(line);
            current_section.push('\n');
        }

        if result.len() + current_section.len() > max_chars {
            break;
        }
    }

    // Add last section
    if in_priority_section {
        result.push_str(&current_section);
    }

    if result.is_empty() {
        // Fallback: just take the beginning
        truncate_to(content, max_chars)
    } else {
        truncate_to(&result, max_chars)
    }
}

/// Extract Makefile targets.
fn extract_makefile_targets(content: &str, max_chars: usize) -> String {
    let mut result = String::new();
    for line in content.lines() {
        if line.contains(':') && !line.starts_with('\t') && !line.starts_with('#') && !line.starts_with(' ') {
            result.push_str(line);
            result.push('\n');
            if result.len() > max_chars {
                break;
            }
        }
    }
    if result.is_empty() {
        truncate_to(content, max_chars)
    } else {
        result
    }
}

fn truncate_to(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...[truncated]", &s[..max])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_docs_in_current_project() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
        let docs = find_docs(root);
        // Should find Cargo.toml at minimum
        assert!(!docs.is_empty(), "Should find at least one doc file");
    }

    #[test]
    fn test_readme_priority() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
        let docs = find_docs(root);
        if docs.len() >= 2 {
            // First item should have lowest priority number
            assert!(docs[0].priority <= docs[1].priority);
        }
    }

    #[test]
    fn test_find_docs_cargo_toml() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
        let docs = find_docs(root);
        assert!(
            docs.iter().any(|d| d.kind == DocKind::CargoToml),
            "Should find Cargo.toml"
        );
    }

    #[test]
    fn test_extract_relevant_short() {
        let doc = DocFile {
            path: PathBuf::from("nonexistent"),
            kind: DocKind::Readme,
            priority: 1,
        };
        // Non-existent file returns empty
        let content = extract_relevant(&doc, 1000);
        assert!(content.is_empty());
    }

    #[test]
    fn test_extract_key_sections() {
        let md = "# My Project\nA cool project.\n\n## Installation\nRun `cargo install`.\n\n## License\nMIT\n\n## Usage\nUse it.\n";
        let result = extract_key_sections(md, 5000);
        assert!(result.contains("My Project"));
        assert!(result.contains("Installation"));
        assert!(result.contains("Usage"));
        assert!(!result.contains("License") || result.len() < md.len());
    }

    #[test]
    fn test_truncate_to() {
        assert_eq!(truncate_to("hello", 10), "hello");
        assert!(truncate_to("hello world this is long", 10).contains("[truncated]"));
    }

    #[test]
    fn test_doc_file_is_readme() {
        let doc = DocFile { path: PathBuf::from("README.md"), kind: DocKind::Readme, priority: 1 };
        assert!(doc.is_readme());

        let doc2 = DocFile { path: PathBuf::from("Cargo.toml"), kind: DocKind::CargoToml, priority: 3 };
        assert!(!doc2.is_readme());
        assert!(doc2.is_build_config());
    }

    #[test]
    fn test_makefile_targets() {
        let content = "# Makefile\nbuild:\n\tcargo build\ntest:\n\tcargo test\nclean:\n\trm -rf target\n";
        let targets = extract_makefile_targets(content, 5000);
        assert!(targets.contains("build:"));
        assert!(targets.contains("test:"));
    }
}
