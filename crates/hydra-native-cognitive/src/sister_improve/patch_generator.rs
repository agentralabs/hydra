//! Patch generation for sister improvements.
//!
//! Generates improvement patches based on limitation analysis.
//! In the full pipeline, this calls the LLM (via Forge sister or direct).
//! For now, generates structural patches for common improvement patterns.

use std::path::PathBuf;
use super::analyzer::SisterAnalysis;

/// Request for patch generation.
#[derive(Debug, Clone)]
pub struct PatchRequest {
    pub sister_path: PathBuf,
    pub limitation: String,
    pub goal: String,
    pub analysis: SisterAnalysis,
}

/// A generated improvement patch.
#[derive(Debug, Clone)]
pub struct ImprovementPatch {
    pub description: String,
    pub target_files: Vec<PathBuf>,
    pub changes: Vec<FileChange>,
}

/// A single file change within a patch.
#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: PathBuf,
    pub kind: ChangeKind,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChangeKind {
    /// Create a new file with this content.
    Create,
    /// Append content to existing file.
    Append,
    /// Replace old_content with new_content.
    Replace { old: String },
}

/// Generate an improvement patch for the given request.
/// Returns None if no patch can be generated for the limitation.
pub fn generate_patch(request: &PatchRequest) -> Option<ImprovementPatch> {
    let limitation = request.limitation.to_lowercase();

    // Pattern: add test coverage
    if limitation.contains("test coverage") || limitation.contains("add test") {
        return generate_test_patch(request);
    }

    // Pattern: add CI
    if limitation.contains("ci ") || limitation.contains("ci configuration") {
        return generate_ci_patch(request);
    }

    // Pattern: add README
    if limitation.contains("readme") || limitation.contains("documentation") {
        return generate_docs_patch(request);
    }

    // Pattern: fix failing tests — can't auto-generate, needs LLM
    if limitation.contains("failing test") || limitation.contains("fix ") {
        return generate_fix_stub(request);
    }

    // Pattern: add retry logic, error handling, etc. — needs LLM
    if limitation.contains("retry") || limitation.contains("error handling")
        || limitation.contains("connection pool") {
        return generate_enhancement_stub(request);
    }

    // For custom goals, generate a stub that the LLM would fill
    Some(ImprovementPatch {
        description: format!("Improvement: {}", request.limitation),
        target_files: vec![],
        changes: vec![],
    })
}

/// Apply an improvement patch to disk.
pub fn apply_patch(patch: &ImprovementPatch) -> Result<(), String> {
    for change in &patch.changes {
        match &change.kind {
            ChangeKind::Create => {
                if let Some(parent) = change.path.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| format!("mkdir failed: {}", e))?;
                }
                std::fs::write(&change.path, &change.content)
                    .map_err(|e| format!("write failed {}: {}", change.path.display(), e))?;
            }
            ChangeKind::Append => {
                let existing = std::fs::read_to_string(&change.path).unwrap_or_default();
                let new_content = format!("{}\n{}", existing, change.content);
                std::fs::write(&change.path, new_content)
                    .map_err(|e| format!("append failed {}: {}", change.path.display(), e))?;
            }
            ChangeKind::Replace { old } => {
                let existing = std::fs::read_to_string(&change.path)
                    .map_err(|e| format!("read failed {}: {}", change.path.display(), e))?;
                let new_content = existing.replace(old, &change.content);
                std::fs::write(&change.path, new_content)
                    .map_err(|e| format!("replace failed {}: {}", change.path.display(), e))?;
            }
        }
    }
    Ok(())
}

fn generate_test_patch(request: &PatchRequest) -> Option<ImprovementPatch> {
    use super::analyzer::SisterLanguage;
    let test_file = match &request.analysis.language {
        SisterLanguage::Rust => request.sister_path.join("tests/suite/main.rs"),
        SisterLanguage::TypeScript => request.sister_path.join("tests/basic.test.ts"),
        SisterLanguage::Python => request.sister_path.join("tests/test_basic.py"),
        SisterLanguage::Go => request.sister_path.join("basic_test.go"),
        SisterLanguage::Unknown(_) => return None,
    };

    let content = match &request.analysis.language {
        SisterLanguage::Rust => "// Auto-generated test stub\n#[test]\nfn test_basic() {\n    assert!(true);\n}\n".to_string(),
        SisterLanguage::TypeScript => "describe('basic', () => {\n  it('should work', () => {\n    expect(true).toBe(true);\n  });\n});\n".to_string(),
        SisterLanguage::Python => "def test_basic():\n    assert True\n".to_string(),
        _ => return None,
    };

    Some(ImprovementPatch {
        description: "Add basic test coverage".to_string(),
        target_files: vec![test_file.clone()],
        changes: vec![FileChange {
            path: test_file,
            kind: ChangeKind::Create,
            content,
        }],
    })
}

fn generate_ci_patch(request: &PatchRequest) -> Option<ImprovementPatch> {
    use super::analyzer::SisterLanguage;
    let ci_path = request.sister_path.join(".github/workflows/ci.yml");
    let test_cmd = match &request.analysis.language {
        SisterLanguage::Rust => "cargo test",
        SisterLanguage::TypeScript => "npm test",
        SisterLanguage::Python => "pytest",
        SisterLanguage::Go => "go test ./...",
        SisterLanguage::Unknown(_) => "make test",
    };

    let content = format!(
        "name: CI\non: [push, pull_request]\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n      - run: {}\n",
        test_cmd
    );

    Some(ImprovementPatch {
        description: "Add GitHub Actions CI".to_string(),
        target_files: vec![ci_path.clone()],
        changes: vec![FileChange {
            path: ci_path,
            kind: ChangeKind::Create,
            content,
        }],
    })
}

fn generate_docs_patch(request: &PatchRequest) -> Option<ImprovementPatch> {
    let readme_path = request.sister_path.join("README.md");
    let content = format!(
        "# {}\n\nA sister project in the Agentra ecosystem.\n\n## Getting Started\n\nSee documentation for setup and usage instructions.\n",
        request.analysis.project_name
    );

    Some(ImprovementPatch {
        description: "Add README documentation".to_string(),
        target_files: vec![readme_path.clone()],
        changes: vec![FileChange {
            path: readme_path,
            kind: ChangeKind::Create,
            content,
        }],
    })
}

fn generate_fix_stub(request: &PatchRequest) -> Option<ImprovementPatch> {
    // Fixing tests requires LLM analysis — return a stub
    Some(ImprovementPatch {
        description: format!("Fix: {}", request.limitation),
        target_files: vec![],
        changes: vec![],
    })
}

fn generate_enhancement_stub(request: &PatchRequest) -> Option<ImprovementPatch> {
    Some(ImprovementPatch {
        description: format!("Enhancement: {}", request.limitation),
        target_files: vec![],
        changes: vec![],
    })
}
