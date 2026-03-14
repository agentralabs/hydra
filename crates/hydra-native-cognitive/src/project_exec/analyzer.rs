//! Project analyzer — uses KnowledgeAcquirer to understand a cloned project.

use std::path::Path;
use crate::knowledge::{self, KnowledgeAcquirer, ProjectKnowledge, DocFile, DocKind};

/// Analysis of a project's structure and requirements.
#[derive(Debug, Clone)]
pub struct ProjectAnalysis {
    pub project_name: String,
    pub docs_found: Vec<DocSummary>,
    pub knowledge: Option<ProjectKnowledge>,
    pub detected_language: Option<String>,
    pub has_tests: bool,
    pub has_build_config: bool,
}

/// Summary of a found documentation file.
#[derive(Debug, Clone)]
pub struct DocSummary {
    pub name: String,
    pub kind: DocKind,
    pub size_bytes: u64,
}

impl ProjectAnalysis {
    /// One-line summary for progress reporting.
    pub fn summary(&self) -> String {
        let lang = self.detected_language.as_deref().unwrap_or("unknown");
        let purpose = self.knowledge.as_ref()
            .map(|k| k.purpose.as_str())
            .filter(|p| !p.is_empty())
            .unwrap_or("purpose unknown");
        format!("{} ({}) — {}", self.project_name, lang, purpose)
    }

    /// Get setup commands from knowledge, or infer from project structure.
    pub fn setup_commands(&self) -> Vec<String> {
        if let Some(k) = &self.knowledge {
            if !k.setup_commands.is_empty() {
                return k.setup_commands.clone();
            }
        }
        // Infer from detected language/build config
        infer_setup_commands(self)
    }

    /// Get test commands from knowledge, or infer from project structure.
    pub fn test_commands(&self) -> Vec<String> {
        if let Some(k) = &self.knowledge {
            if !k.test_commands.is_empty() {
                return k.test_commands.clone();
            }
        }
        infer_test_commands(self)
    }
}

/// Analyze a project directory without LLM (structure-only).
pub fn analyze_project(project_dir: &Path) -> ProjectAnalysis {
    let acquirer = KnowledgeAcquirer::new();
    let docs = acquirer.find_docs(project_dir);

    let project_name = project_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let docs_summary: Vec<DocSummary> = docs.iter().map(|d| {
        let size = std::fs::metadata(&d.path).map(|m| m.len()).unwrap_or(0);
        DocSummary {
            name: d.path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?")
                .to_string(),
            kind: d.kind,
            size_bytes: size,
        }
    }).collect();

    let detected_language = detect_primary_language(project_dir, &docs);
    let detected_lang = Some(detected_language);
    let has_tests = detect_has_tests(project_dir, &detected_lang);
    let has_build_config = docs.iter().any(|d| d.is_build_config());

    // Try to extract purpose from README without LLM
    let purpose = extract_purpose_from_readme(project_dir);
    let knowledge = if !purpose.is_empty() {
        Some(knowledge::ProjectKnowledge {
            project_name: project_name.clone(),
            purpose,
            setup_commands: vec![],
            test_commands: vec![],
            api_endpoints: vec![],
            dependencies: vec![],
            learned_at: chrono::Utc::now(),
        })
    } else {
        None
    };

    ProjectAnalysis {
        project_name,
        docs_found: docs_summary,
        knowledge,
        detected_language: detected_lang,
        has_tests,
        has_build_config,
    }
}

/// Build the LLM prompts needed to understand this project.
/// Returns Vec<(prompt, label)> for the caller to send to the LLM.
pub fn build_learn_prompts(project_dir: &Path) -> Vec<(String, String)> {
    let acquirer = KnowledgeAcquirer::new();
    acquirer.plan_learning(project_dir)
}

/// Parse an LLM response into ProjectKnowledge.
pub fn parse_knowledge(project_name: &str, response: &str) -> ProjectKnowledge {
    let acquirer = KnowledgeAcquirer::new();
    acquirer.parse_readme_response(project_name, response)
}

/// Detect the primary programming language from project structure.
fn detect_primary_language(dir: &Path, docs: &[DocFile]) -> String {
    // Check build config files
    for doc in docs {
        match doc.kind {
            DocKind::CargoToml => {
                if doc.path.file_name().and_then(|n| n.to_str()) == Some("Cargo.toml") {
                    return "Rust".to_string();
                }
                if doc.path.file_name().and_then(|n| n.to_str()) == Some("pyproject.toml") {
                    return "Python".to_string();
                }
                if doc.path.file_name().and_then(|n| n.to_str()) == Some("go.mod") {
                    return "Go".to_string();
                }
            }
            DocKind::PackageJson => return "JavaScript".to_string(),
            _ => {}
        }
    }

    // Fallback: check for common files
    if dir.join("Cargo.toml").exists() { return "Rust".to_string(); }
    if dir.join("package.json").exists() { return "JavaScript".to_string(); }
    if dir.join("pyproject.toml").exists() || dir.join("setup.py").exists() { return "Python".to_string(); }
    if dir.join("go.mod").exists() { return "Go".to_string(); }
    if dir.join("Gemfile").exists() { return "Ruby".to_string(); }
    if dir.join("pom.xml").exists() || dir.join("build.gradle").exists() { return "Java".to_string(); }

    "unknown".to_string()
}

/// Detect whether the project has tests.
fn detect_has_tests(dir: &Path, language: &Option<String>) -> bool {
    let lang = language.as_deref().unwrap_or("");
    match lang {
        "Rust" => dir.join("tests").exists() || has_test_pattern(dir, "src", "#[test]"),
        "JavaScript" => dir.join("test").exists() || dir.join("__tests__").exists(),
        "Python" => dir.join("tests").exists() || dir.join("test").exists(),
        "Go" => has_file_extension(dir, "_test.go"),
        _ => dir.join("tests").exists() || dir.join("test").exists(),
    }
}

/// Check if any file in a subdir contains a pattern (shallow, fast).
fn has_test_pattern(dir: &Path, subdir: &str, _pattern: &str) -> bool {
    // Just check if the subdir exists with files — avoid scanning content
    let sub = dir.join(subdir);
    sub.exists() && sub.is_dir()
}

/// Extract a purpose description from README.md without LLM.
///
/// Reads the first meaningful paragraph (skips title/badges/blank lines)
/// and uses it as the project purpose. Fast, no API calls.
fn extract_purpose_from_readme(dir: &Path) -> String {
    let readme_path = if dir.join("README.md").exists() {
        dir.join("README.md")
    } else if dir.join("readme.md").exists() {
        dir.join("readme.md")
    } else if dir.join("README.rst").exists() {
        dir.join("README.rst")
    } else {
        return String::new();
    };

    let content = match std::fs::read_to_string(&readme_path) {
        Ok(c) => c,
        Err(_) => return String::new(),
    };

    // Find the first non-title, non-badge, non-blank line
    for line in content.lines() {
        let trimmed = line.trim();
        // Skip empty lines, markdown headers, badges, HTML tags, horizontal rules
        if trimmed.is_empty()
            || trimmed.starts_with('#')
            || trimmed.starts_with("![")
            || trimmed.starts_with("[![")
            || trimmed.starts_with('<')
            || trimmed.starts_with("---")
            || trimmed.starts_with("===")
            || trimmed.starts_with("```")
        {
            continue;
        }
        // Found a content line — use it as purpose (truncate to reasonable length)
        let purpose = if trimmed.len() > 120 {
            format!("{}...", &trimmed[..117])
        } else {
            trimmed.to_string()
        };
        return purpose;
    }
    String::new()
}

/// Check if any file with the given extension suffix exists (shallow).
fn has_file_extension(dir: &Path, suffix: &str) -> bool {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten().take(50) {
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(suffix) {
                    return true;
                }
            }
        }
    }
    false
}

/// Infer setup commands from project structure.
fn infer_setup_commands(analysis: &ProjectAnalysis) -> Vec<String> {
    match analysis.detected_language.as_deref() {
        Some("Rust") => vec!["cargo build".to_string()],
        Some("JavaScript") => vec!["npm install".to_string()],
        Some("Python") => vec!["pip install -e .".to_string()],
        Some("Go") => vec!["go build ./...".to_string()],
        Some("Ruby") => vec!["bundle install".to_string()],
        _ => vec![],
    }
}

/// Infer test commands from project structure.
fn infer_test_commands(analysis: &ProjectAnalysis) -> Vec<String> {
    match analysis.detected_language.as_deref() {
        Some("Rust") => vec!["cargo test".to_string()],
        Some("JavaScript") => vec!["npm test".to_string()],
        Some("Python") => vec!["pytest".to_string()],
        Some("Go") => vec!["go test ./...".to_string()],
        Some("Ruby") => vec!["bundle exec rspec".to_string()],
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_current_project() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
        let analysis = analyze_project(root);
        assert_eq!(analysis.detected_language, Some("Rust".to_string()));
        assert!(analysis.has_build_config);
        assert!(!analysis.docs_found.is_empty());
    }

    #[test]
    fn test_analysis_summary() {
        let analysis = ProjectAnalysis {
            project_name: "myapp".into(),
            docs_found: vec![],
            knowledge: None,
            detected_language: Some("Rust".into()),
            has_tests: true,
            has_build_config: true,
        };
        let s = analysis.summary();
        assert!(s.contains("myapp"));
        assert!(s.contains("Rust"));
    }

    #[test]
    fn test_infer_rust_commands() {
        let analysis = ProjectAnalysis {
            project_name: "test".into(),
            docs_found: vec![],
            knowledge: None,
            detected_language: Some("Rust".into()),
            has_tests: true,
            has_build_config: true,
        };
        assert_eq!(analysis.setup_commands(), vec!["cargo build"]);
        assert_eq!(analysis.test_commands(), vec!["cargo test"]);
    }

    #[test]
    fn test_infer_js_commands() {
        let analysis = ProjectAnalysis {
            project_name: "test".into(),
            docs_found: vec![],
            knowledge: None,
            detected_language: Some("JavaScript".into()),
            has_tests: false,
            has_build_config: true,
        };
        assert_eq!(analysis.setup_commands(), vec!["npm install"]);
        assert_eq!(analysis.test_commands(), vec!["npm test"]);
    }

    #[test]
    fn test_knowledge_overrides_inference() {
        let knowledge = crate::knowledge::ProjectKnowledge {
            project_name: "test".into(),
            purpose: "test framework".into(),
            setup_commands: vec!["make build".into()],
            test_commands: vec!["make test".into()],
            api_endpoints: vec![],
            dependencies: vec![],
            learned_at: chrono::Utc::now(),
        };
        let analysis = ProjectAnalysis {
            project_name: "test".into(),
            docs_found: vec![],
            knowledge: Some(knowledge),
            detected_language: Some("Rust".into()),
            has_tests: true,
            has_build_config: true,
        };
        assert_eq!(analysis.setup_commands(), vec!["make build"]);
        assert_eq!(analysis.test_commands(), vec!["make test"]);
    }

    #[test]
    fn test_build_learn_prompts() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
        let prompts = build_learn_prompts(root);
        // Should generate at least one prompt for README
        if root.join("README.md").exists() {
            assert!(!prompts.is_empty());
        }
    }
}
