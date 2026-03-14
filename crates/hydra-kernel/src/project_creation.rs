//! Project creation mode for the SelfImplement pipeline.
//!
//! When a spec describes building a NEW project (not modifying existing code),
//! this module scaffolds a Cargo workspace, reads template context from
//! existing sisters, and drives code generation into the new project.

use std::path::{Path, PathBuf};
use regex::Regex;
use serde::{Deserialize, Serialize};

/// Configuration for a new project, extracted from the spec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Sister key (e.g., "echo", "data")
    pub key: String,
    /// Full name (e.g., "AgenticEcho")
    pub name: String,
    /// File extension (e.g., ".aecho")
    pub file_ext: String,
    /// CLI binary name (e.g., "aecho")
    pub cli_binary: String,
    /// MCP tool names
    pub tools: Vec<String>,
    /// Short description
    pub description: String,
    /// Target directory for the new project
    pub target_dir: PathBuf,
}

/// Detect whether a spec describes a new project (not modification of existing code).
/// Returns a ProjectConfig if detected, None otherwise.
pub fn detect_new_project(spec: &str) -> Option<ProjectConfig> {
    let lower = spec.to_lowercase();

    // Must mention building something new
    let is_new_project = lower.contains("new sister")
        || lower.contains("new project")
        || lower.contains("build agentic")
        || lower.contains("create agentic")
        || (lower.contains("build") && lower.contains("sister"))
        || lower.contains("validation test")  // factory validation spec
        || (lower.contains("agentic") && lower.contains("file format:"));

    if !is_new_project {
        return None;
    }

    // Extract the sister name: "AgenticEcho", "AgenticData", etc.
    let name_re = Regex::new(r"(?i)Agentic(\w+)").ok()?;
    let name_match = name_re.captures(spec)?;
    let suffix = name_match.get(1)?.as_str();

    // Skip if it's an existing sister we'd be modifying
    let existing = ["Memory", "Vision", "Codebase", "Identity", "Time",
        "Contract", "Comm", "Planning", "Cognition", "Reality",
        "Forge", "Aegis", "Veritas", "Evolve"];
    if existing.iter().any(|e| e.eq_ignore_ascii_case(suffix)) {
        return None;
    }

    let key = suffix.to_lowercase();
    let name = format!("Agentic{}", capitalize(suffix));
    let file_ext = format!(".a{}", &key[..key.len().min(4)]);
    let cli_binary = format!("a{}", &key[..key.len().min(4)]);

    // Extract tools from spec (lines like "echo_send — Store a message")
    let tool_re = Regex::new(r"(?m)^\s+(\w+_\w+)\s+[—–-]").ok()?;
    let tools: Vec<String> = tool_re.captures_iter(spec)
        .map(|c| c[1].to_string())
        .collect();

    // Extract description from first paragraph or title
    let description = spec.lines()
        .find(|l| l.starts_with("# "))
        .map(|l| l.trim_start_matches('#').trim().to_string())
        .unwrap_or_else(|| format!("{} sister", name));

    // Determine target directory (parent of hydra repo)
    let hydra_dir = std::env::current_dir().unwrap_or_default();
    let parent = hydra_dir.parent().unwrap_or(&hydra_dir);
    let target_dir = parent.join(format!("agentic-{}", key));

    Some(ProjectConfig {
        key, name, file_ext, cli_binary, tools, description, target_dir,
    })
}

/// Scaffold the workspace directory structure for a new sister project.
/// Creates all directories and minimal Cargo.toml files so cargo check works.
pub fn scaffold_workspace(config: &ProjectConfig) -> Result<PathBuf, String> {
    let dir = &config.target_dir;

    // Don't overwrite existing projects
    if dir.join("Cargo.toml").exists() {
        eprintln!("[hydra:project] Project already exists at {}, reusing", dir.display());
        return Ok(dir.clone());
    }

    let templates = super::project_creation_templates::ProjectTemplates::render(config);

    // Create directory structure
    let dirs = [
        format!("crates/agentic-{}/src", config.key),
        format!("crates/agentic-{}-mcp/src/tools", config.key),
        format!("crates/agentic-{}-cli/src", config.key),
        format!("crates/agentic-{}-ffi/src", config.key),
    ];
    for d in &dirs {
        std::fs::create_dir_all(dir.join(d))
            .map_err(|e| format!("Failed to create {}: {}", d, e))?;
    }

    // Write workspace Cargo.toml
    write_file(dir, "Cargo.toml", &templates.workspace_cargo)?;

    // Write core crate
    let core = format!("crates/agentic-{}", config.key);
    write_file(dir, &format!("{}/Cargo.toml", core), &templates.core_cargo)?;
    write_file(dir, &format!("{}/src/lib.rs", core), &templates.core_lib)?;

    // Write MCP crate
    let mcp = format!("crates/agentic-{}-mcp", config.key);
    write_file(dir, &format!("{}/Cargo.toml", mcp), &templates.mcp_cargo)?;
    write_file(dir, &format!("{}/src/main.rs", mcp), &templates.mcp_main)?;
    write_file(dir, &format!("{}/src/tools/mod.rs", mcp), &templates.mcp_tools_mod)?;
    write_file(dir, &format!("{}/src/tools/registry.rs", mcp), &templates.mcp_registry)?;

    // Write CLI crate
    let cli = format!("crates/agentic-{}-cli", config.key);
    write_file(dir, &format!("{}/Cargo.toml", cli), &templates.cli_cargo)?;
    write_file(dir, &format!("{}/src/main.rs", cli), &templates.cli_main)?;

    // Write FFI crate
    let ffi = format!("crates/agentic-{}-ffi", config.key);
    write_file(dir, &format!("{}/Cargo.toml", ffi), &templates.ffi_cargo)?;
    write_file(dir, &format!("{}/src/lib.rs", ffi), &templates.ffi_lib)?;

    eprintln!("[hydra:project] Scaffolded workspace at {}", dir.display());
    Ok(dir.clone())
}

/// Apply patches to a new project by OVERWRITING files (not appending).
/// Unlike `SelfModificationPipeline::apply_patch()` which uses smart_patch (append mode),
/// new project patches are COMPLETE file replacements since the LLM generates
/// full file content, not diffs.
pub fn apply_project_patch(
    project_dir: &Path,
    patch: &crate::self_modify::Patch,
) -> Result<(), String> {
    let file_path = project_dir.join(&patch.target_file);

    // Create parent directories if needed
    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("mkdir {}: {}", parent.display(), e))?;
    }

    // OVERWRITE the file with complete content (not append)
    std::fs::write(&file_path, &patch.diff_content)
        .map_err(|e| format!("write {}: {}", file_path.display(), e))?;

    // Post-check: verify line count
    let line_count = patch.diff_content.lines().count();
    if line_count > 400 {
        return Err(format!(
            "Patch pushed {} to {} lines (max 400)",
            file_path.display(), line_count
        ));
    }

    Ok(())
}

/// Run `run_from_gaps` in overwrite mode for new projects.
/// Same as `SelfModificationPipeline::run_from_gaps` but overwrites files.
pub fn run_project_patches(
    project_dir: &Path,
    gaps: Vec<crate::self_modify::SpecGap>,
    patches: Vec<crate::self_modify::Patch>,
) -> crate::self_modify::ModResult {
    use crate::self_modify::{FileCheckpoint, ModResult};

    if gaps.is_empty() {
        return ModResult::AlreadyImplemented;
    }
    if patches.len() > 20 {
        return ModResult::PipelineError {
            message: format!("Too many patches ({}) — max 20", patches.len()),
        };
    }

    // Checkpoint files that will be overwritten
    let paths: Vec<std::path::PathBuf> = patches.iter()
        .map(|p| project_dir.join(&p.target_file))
        .collect();
    let checkpoint = FileCheckpoint::capture(&paths);

    // Apply patches (overwrite mode)
    for patch in &patches {
        if let Err(e) = apply_project_patch(project_dir, patch) {
            let _ = checkpoint.revert();
            return ModResult::PipelineError {
                message: format!("Patch apply failed: {}. Reverted.", e),
            };
        }
    }

    // Verify: cargo check on the new project
    match run_cargo_check_project(project_dir) {
        Ok(_) => eprintln!("[hydra:project] cargo check passed"),
        Err(e) => {
            let _ = checkpoint.revert();
            return ModResult::CompileFailed {
                error: format!("Compilation failed (reverted):\n{}", e),
                reverted: true,
            };
        }
    }

    ModResult::Success {
        gaps_filled: gaps.len(),
        patches_applied: patches.len(),
        tests_passing: 0,
    }
}

/// Run `cargo check` in the new project directory.
pub fn run_cargo_check_project(project_dir: &Path) -> Result<(), String> {
    let output = std::process::Command::new("cargo")
        .args(["check", "-j", "1", "--message-format=short"])
        .current_dir(project_dir)
        .output()
        .map_err(|e| format!("Failed to run cargo: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let errors: String = stderr.lines()
            .filter(|l| l.contains("error"))
            .take(10)
            .collect::<Vec<_>>()
            .join("\n");
        Err(if errors.is_empty() { stderr.to_string() } else { errors })
    }
}

/// Run `cargo test` in the new project directory.
pub fn run_cargo_test_project(project_dir: &Path) -> Result<String, String> {
    let output = std::process::Command::new("cargo")
        .args(["test", "-j", "1", "--", "--nocapture"])
        .current_dir(project_dir)
        .output()
        .map_err(|e| format!("Failed to run cargo test: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    if output.status.success() {
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("{}\n{}", stdout, stderr))
    }
}

/// Gather template context by reading an existing sister's structure.
/// Reads agentic-memory's MCP tool registry as a reference pattern.
pub fn gather_template_context(hydra_parent_dir: &Path) -> String {
    let memory_dir = hydra_parent_dir.join("agentic-memory");
    let mut ctx = String::from("## Reference: agentic-memory structure\n\n");

    // Read MCP tool registry pattern
    let registry = memory_dir.join("crates/agentic-memory-mcp/src/tools/registry.rs");
    if let Ok(content) = std::fs::read_to_string(&registry) {
        let lines: Vec<&str> = content.lines().take(60).collect();
        ctx.push_str(&format!("### MCP Tool Registry Pattern\n```rust\n{}\n```\n\n", lines.join("\n")));
    }

    // Read MCP error types
    let error_types = memory_dir.join("crates/agentic-memory-mcp/src/types/error.rs");
    if let Ok(content) = std::fs::read_to_string(&error_types) {
        let lines: Vec<&str> = content.lines().take(40).collect();
        ctx.push_str(&format!("### MCP Error Types\n```rust\n{}\n```\n\n", lines.join("\n")));
    }

    // Read core lib structure
    let core_lib = memory_dir.join("crates/agentic-memory/src/lib.rs");
    if let Ok(content) = std::fs::read_to_string(&core_lib) {
        let lines: Vec<&str> = content.lines().take(30).collect();
        ctx.push_str(&format!("### Core Library Pattern\n```rust\n{}\n```\n\n", lines.join("\n")));
    }

    if ctx.len() < 100 {
        ctx.push_str("(agentic-memory not found locally — use standard Rust MCP patterns)\n");
    }

    ctx
}

/// Read NEW-SISTER-PLAYBOOK.md for structure requirements.
pub fn gather_playbook_context(hydra_parent_dir: &Path) -> String {
    let playbook = hydra_parent_dir.join("goals/NEW-SISTER-PLAYBOOK.md");
    if let Ok(content) = std::fs::read_to_string(&playbook) {
        // Extract just the crate structure section (keep it concise)
        let mut result = String::from("## Sister Structure Requirements\n");
        let mut in_structure = false;
        for line in content.lines().take(120) {
            if line.contains("Crate structure") || line.contains("2.1") {
                in_structure = true;
            }
            if in_structure {
                result.push_str(line);
                result.push('\n');
                if line.trim().is_empty() && result.len() > 200 {
                    break;
                }
            }
        }
        result
    } else {
        String::from("(NEW-SISTER-PLAYBOOK.md not found)\n")
    }
}

fn write_file(base: &Path, rel_path: &str, content: &str) -> Result<(), String> {
    let path = base.join(rel_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("mkdir {}: {}", parent.display(), e))?;
    }
    std::fs::write(&path, content)
        .map_err(|e| format!("write {}: {}", path.display(), e))
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().to_string() + &c.as_str().to_lowercase(),
    }
}

#[cfg(test)]
#[path = "project_creation_tests.rs"]
mod tests;
