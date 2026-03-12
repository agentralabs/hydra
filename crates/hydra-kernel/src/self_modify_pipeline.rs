//! Self-modification pipeline — the execution engine that drives spec -> gap -> patch -> verify.

use std::path::{Path, PathBuf};

use crate::self_modify::{extract_fn_name, FileCheckpoint, GapType, ModResult, Patch, SpecGap};
pub use crate::self_modify_llm::extract_spec_path;

/// The self-modification pipeline.
pub struct SelfModificationPipeline {
    pub project_dir: PathBuf,
    /// Maximum patches per run (safety limit).
    pub max_patches: usize,
}

impl SelfModificationPipeline {
    pub fn new(project_dir: impl Into<PathBuf>) -> Self {
        Self {
            project_dir: project_dir.into(),
            max_patches: 5,
        }
    }

    /// Read and parse a spec file.
    pub fn read_spec(&self, spec_path: &Path) -> Result<String, ModResult> {
        std::fs::read_to_string(spec_path).map_err(|e| ModResult::PipelineError {
            message: format!("Cannot read spec: {}", e),
        })
    }

    /// Find gaps from a spec when the LLM is unavailable.
    /// Parses markdown structure: Requirement/Implementation Location sections,
    /// plus literal `fn`/`mod`/`#[test]` declarations.
    pub fn find_gaps(&self, spec_content: &str) -> Vec<SpecGap> {
        let mut gaps = Vec::new();
        let mut current_section = String::new();
        let mut target_file = String::new();

        for line in spec_content.lines() {
            let trimmed = line.trim();

            // Track markdown sections
            if trimmed.starts_with("## ") || trimmed.starts_with("# ") {
                current_section = trimmed.trim_start_matches('#').trim().to_lowercase();
                continue;
            }

            // Extract target file from Implementation Location section
            if current_section.contains("implementation") || current_section.contains("location") {
                if let Some(path) = extract_file_path_from_line(trimmed) {
                    target_file = path;
                }
            }

            // Extract requirements from Requirement/Acceptance sections
            if (current_section.contains("requirement") || current_section.contains("acceptance"))
                && (trimmed.starts_with("- ") || trimmed.starts_with("* ")
                    || (trimmed.len() > 3 && trimmed.chars().next().map_or(false, |c| c.is_ascii_digit())))
            {
                let desc = trimmed.trim_start_matches(|c: char| c == '-' || c == '*' || c == '.' || c.is_ascii_digit() || c.is_whitespace());
                if !desc.is_empty() && desc.len() > 5 {
                    gaps.push(SpecGap {
                        description: desc.to_string(),
                        target_file: target_file.clone(),
                        gap_type: GapType::MissingFunction,
                        priority: 1,
                    });
                }
            }

            // Also detect literal fn/mod/test declarations
            if (trimmed.starts_with("fn ") || trimmed.starts_with("pub fn "))
                && !self.function_exists_in_source(trimmed)
            {
                let fn_name = extract_fn_name(trimmed);
                gaps.push(SpecGap {
                    description: format!("Missing function: {}", fn_name),
                    target_file: target_file.clone(),
                    gap_type: GapType::MissingFunction,
                    priority: 1,
                });
            }
        }

        // If no gaps found from sections but spec has content, create one from the title
        if gaps.is_empty() && !spec_content.is_empty() {
            if let Some(title) = spec_content.lines().find(|l| l.starts_with("# ")) {
                let desc = title.trim_start_matches('#').trim();
                gaps.push(SpecGap {
                    description: format!("Implement: {}", desc),
                    target_file: target_file,
                    gap_type: GapType::MissingFunction,
                    priority: 1,
                });
            }
        }

        gaps.into_iter().take(5).collect()
    }

    /// Check if a function already exists in the project source.
    fn function_exists_in_source(&self, fn_signature: &str) -> bool {
        let fn_name = extract_fn_name(fn_signature);
        if fn_name.is_empty() {
            return false;
        }

        // Quick grep through source files
        let output = std::process::Command::new("grep")
            .args(["-rl", &format!("fn {}", fn_name), &self.project_dir.to_string_lossy()])
            .output();

        match output {
            Ok(out) => !out.stdout.is_empty(),
            Err(_) => false,
        }
    }

    /// Create a checkpoint before applying patches.
    pub fn create_checkpoint(&self, patches: &[Patch]) -> FileCheckpoint {
        let paths: Vec<PathBuf> = patches
            .iter()
            .map(|p| self.project_dir.join(&p.target_file))
            .collect();
        FileCheckpoint::capture(&paths)
    }

    /// Apply a single patch to a file using the smart patch applicator.
    /// - New files: creates and registers `pub mod` in parent lib.rs/mod.rs
    /// - Existing files: skips duplicate functions, deduplicates imports
    pub fn apply_patch(&self, patch: &Patch) -> Result<(), String> {
        // Use smart_patch for intelligent application
        crate::smart_patch::apply_smart(
            &self.project_dir,
            &patch.target_file,
            &patch.diff_content,
        )?;

        // Safety: enforce 400-line limit per CLAUDE.md OOM rules
        let file_path = self.project_dir.join(&patch.target_file);
        let content = std::fs::read_to_string(&file_path).unwrap_or_default();
        let line_count = content.lines().count();
        if line_count > 400 {
            return Err(format!(
                "Patch pushed {} to {} lines (max 400). Split the target file first.",
                file_path.display(),
                line_count
            ));
        }

        Ok(())
    }

    /// Run the full pipeline: spec -> gaps -> patches -> checkpoint -> apply -> verify.
    /// Returns the result. Auto-reverts on failure.
    pub fn run_from_gaps(
        &self,
        gaps: Vec<SpecGap>,
        patches: Vec<Patch>,
    ) -> ModResult {
        if gaps.is_empty() {
            return ModResult::AlreadyImplemented;
        }

        // Safety: limit patches per run
        if patches.len() > self.max_patches {
            return ModResult::PipelineError {
                message: format!(
                    "Too many patches ({}) — max {} per run for safety.",
                    patches.len(),
                    self.max_patches
                ),
            };
        }

        // Check for critical file modifications
        for patch in &patches {
            if patch.requires_extra_approval() {
                return ModResult::ShadowFailed {
                    reason: format!(
                        "Patch touches critical safety file: {}. Human approval required.",
                        patch.target_file
                    ),
                    patch_summary: patch.description.clone(),
                };
            }
        }

        // Create checkpoint
        let checkpoint = self.create_checkpoint(&patches);

        // Apply patches
        for patch in &patches {
            if let Err(e) = self.apply_patch(patch) {
                let _ = checkpoint.revert();
                return ModResult::PipelineError {
                    message: format!("Patch apply failed: {}. Reverted.", e),
                };
            }
        }

        // Note: actual cargo check/test would be run by the user
        // (per the CARGO BAN). The pipeline reports what was applied.
        ModResult::Success {
            gaps_filled: gaps.len(),
            patches_applied: patches.len(),
            tests_passing: 0, // User must verify
        }
    }
}

/// Extract a file path (crates/...) from a markdown line.
fn extract_file_path_from_line(line: &str) -> Option<String> {
    // Look for crates/... paths or src/... paths
    for word in line.split_whitespace() {
        let cleaned = word.trim_matches(|c: char| c == '`' || c == '"' || c == '\'' || c == '(' || c == ')');
        if (cleaned.starts_with("crates/") || cleaned.starts_with("src/"))
            && cleaned.contains(".rs")
        {
            return Some(cleaned.to_string());
        }
    }
    None
}
