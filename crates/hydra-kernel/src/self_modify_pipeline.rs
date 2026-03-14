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
            max_patches: 10,
        }
    }

    /// Read a spec file. If not found at the given path, search common locations.
    pub fn read_spec(&self, spec_path: &Path) -> Result<String, ModResult> {
        // Try exact path first
        if let Ok(content) = std::fs::read_to_string(spec_path) {
            return Ok(content);
        }
        // Extract filename for search
        let filename = spec_path.file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();
        if filename.is_empty() {
            return Err(ModResult::PipelineError {
                message: format!("Cannot read spec: {}", spec_path.display()),
            });
        }
        // Try common alternative locations
        let candidates = [
            self.project_dir.join("test-specs").join(&filename),
            self.project_dir.join("specs").join(&filename),
            self.project_dir.join("spec").join(&filename),
            self.project_dir.join("docs").join(&filename),
            self.project_dir.join(&filename),
        ];
        for candidate in &candidates {
            if let Ok(content) = std::fs::read_to_string(candidate) {
                eprintln!("[hydra:self-impl] Spec found at {} (not {})",
                    candidate.display(), spec_path.display());
                return Ok(content);
            }
        }
        // Last resort: `find` from project root
        if let Ok(output) = std::process::Command::new("find")
            .args([self.project_dir.to_str().unwrap_or("."), "-name", &filename, "-type", "f"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(found) = stdout.lines().next() {
                let found_path = Path::new(found.trim());
                if let Ok(content) = std::fs::read_to_string(found_path) {
                    eprintln!("[hydra:self-impl] Spec found via search at {}", found_path.display());
                    return Ok(content);
                }
            }
        }
        Err(ModResult::PipelineError {
            message: format!("Spec file '{}' not found. Searched: {}, test-specs/, specs/, spec/, docs/, project root, and full filesystem.",
                filename, spec_path.display()),
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

        gaps.into_iter().take(10).collect()
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
        let file_path = self.project_dir.join(&patch.target_file);

        // Pre-flight: check if patch + existing would exceed 400 lines
        if file_path.exists() {
            let existing = std::fs::read_to_string(&file_path).unwrap_or_default();
            let existing_lines = existing.lines().count();
            let patch_lines = patch.diff_content.lines().count();
            if existing_lines + patch_lines > 420 {
                return Err(format!(
                    "Pre-flight: {} has {} lines + {} patch lines = {} (max 400). Split the file first.",
                    patch.target_file, existing_lines, patch_lines, existing_lines + patch_lines
                ));
            }
        }

        // Use smart_patch for intelligent application
        crate::smart_patch::apply_smart(
            &self.project_dir,
            &patch.target_file,
            &patch.diff_content,
        )?;

        // Post-check: verify final line count
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

        // Verify: run cargo check on affected crates
        let affected_crates = collect_affected_crates(&patches);
        let mut check_errors = Vec::new();
        for crate_name in &affected_crates {
            match run_cargo_check(crate_name, &self.project_dir) {
                Ok(_) => eprintln!("[hydra:self-impl] cargo check -p {} passed", crate_name),
                Err(e) => check_errors.push(format!("{}: {}", crate_name, e)),
            }
        }

        if !check_errors.is_empty() {
            let _ = checkpoint.revert();
            return ModResult::CompileFailed {
                error: format!("Compilation failed (reverted):\n{}", check_errors.join("\n")),
                reverted: true,
            };
        }

        ModResult::Success {
            gaps_filled: gaps.len(),
            patches_applied: patches.len(),
            tests_passing: 0, // cargo check passed; run `cargo test` to verify
        }
    }
}

/// Extract crate names from patch target paths (e.g., "crates/hydra-kernel/src/foo.rs" -> "hydra-kernel").
fn collect_affected_crates(patches: &[Patch]) -> Vec<String> {
    let mut crates: Vec<String> = patches
        .iter()
        .filter_map(|p| {
            let parts: Vec<&str> = p.target_file.split('/').collect();
            if parts.len() >= 2 && parts[0] == "crates" {
                Some(parts[1].to_string())
            } else {
                None
            }
        })
        .collect();
    crates.sort();
    crates.dedup();
    crates
}

/// Run `cargo check -p <crate> -j 1` and return Ok(()) on success.
pub fn run_cargo_check(crate_name: &str, project_dir: &Path) -> Result<(), String> {
    let output = std::process::Command::new("cargo")
        .args(["check", "-p", crate_name, "-j", "1", "--message-format=short"])
        .current_dir(project_dir)
        .output()
        .map_err(|e| format!("Failed to run cargo: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let errors: String = stderr
            .lines()
            .filter(|l| l.contains("error"))
            .take(10)
            .collect::<Vec<_>>()
            .join("\n");
        Err(if errors.is_empty() { stderr.to_string() } else { errors })
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
