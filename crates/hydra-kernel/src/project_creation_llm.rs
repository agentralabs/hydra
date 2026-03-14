//! LLM prompts and calls for new project creation mode.
//!
//! Different from self_modify_llm.rs: tells the LLM to target the NEW project directory,
//! provides template context from existing sisters, and uses project-aware prompts.

use std::path::Path;

use crate::self_modify::{Patch, SpecGap};
use crate::self_modify_llm::{call_llm, gather_workspace_context};
use crate::self_modify_llm_parse::{parse_gaps_from_response, parse_patches_from_response};
use crate::project_creation::ProjectConfig;

const PROJECT_GAP_PROMPT: &str = r#"You are analyzing a spec for building a NEW Rust sister project.
The project has already been scaffolded with a Cargo workspace containing:
- Core library crate with Store struct (all CRUD methods already implemented and tested)
- MCP server crate with JSON-RPC dispatch and ALL tool handlers already implemented
- CLI crate (working skeleton)
- FFI crate (working skeleton)

The scaffold is ALREADY COMPILABLE. Most code is complete. Only look for gaps in
things the scaffold does NOT cover (e.g. additional test coverage, missing edge case handling).

DO NOT report gaps for:
- Store methods (send/query/history/stats/clear) — already implemented
- MCP tool handlers — already implemented in registry.rs
- Cargo.toml files — already correct
- CLI or FFI skeletons — already working

ONLY report gaps for things genuinely missing from the scaffolded code.
If the scaffold covers everything the spec requires, return an EMPTY array: []

Return ONLY a JSON array (no markdown fences). Each element:
{"description": "what needs implementing", "target_file": "crates/agentic-<key>/src/lib.rs", "gap_type": "missing_test", "priority": 2}

gap_type: missing_function, missing_module, missing_test, incomplete_implementation
priority: 1 = critical, 2 = important, 3 = nice-to-have
Max 5 gaps."#;

const PROJECT_PATCH_PROMPT: &str = r#"You are generating Rust code for a NEW sister project.
The project is ALREADY scaffolded and compiles. Only generate code for genuinely missing pieces.

Store API (already implemented in core lib.rs — DO NOT regenerate):
  pub fn open(path: &Path) -> Result<Self, String>
  pub fn open_memory() -> Result<Self, String>
  pub fn send(&self, content: &str) -> Result<StoredMessage, String>
  pub fn query(&self, keyword: &str) -> Result<Vec<StoredMessage>, String>  // REQUIRES keyword arg
  pub fn history(&self, limit: usize) -> Result<Vec<StoredMessage>, String> // REQUIRES limit arg
  pub fn stats(&self) -> Result<Stats, String>
  pub fn clear(&self) -> Result<usize, String>

CRITICAL RULES:
- DO NOT overwrite files that already compile and work (core lib.rs, registry.rs, main.rs)
- Only add NEW files or modify files with genuine gaps
- If a gap says "missing_test", add tests to the appropriate file
- Keep each file under 400 lines total
- Use Store::open_memory() in tests
- If there are no real gaps, return an EMPTY array: []

Return ONLY a JSON array (no markdown fences). Each element:
{"target_file": "crates/agentic-<key>/src/lib.rs", "diff_content": "complete file content", "description": "what this implements"}
Max 5 patches. Return COMPLETE file contents (not diffs)."#;

const PROJECT_FIX_PROMPT: &str = r#"You are fixing Rust code that failed to compile in a NEW sister project.

Store API (correct signatures — use these EXACTLY):
  pub fn send(&self, content: &str) -> Result<StoredMessage, String>
  pub fn query(&self, keyword: &str) -> Result<Vec<StoredMessage>, String>
  pub fn history(&self, limit: usize) -> Result<Vec<StoredMessage>, String>
  pub fn stats(&self) -> Result<Stats, String>
  pub fn clear(&self) -> Result<usize, String>

CRITICAL RULES:
- Return COMPLETE corrected code for each file (previous code was reverted)
- Fix the specific errors shown (missing imports, wrong types, wrong arg count)
- query() takes 1 arg (keyword: &str), history() takes 1 arg (limit: usize)
- Keep file paths exactly as they were

Return ONLY a JSON array:
{"target_file": "crates/.../src/file.rs", "diff_content": "corrected complete Rust code", "description": "what was fixed"}
Return COMPLETE file content, not patches."#;

/// Analyze gaps in a new project (what tool handlers are missing).
pub async fn analyze_project_gaps(
    spec: &str,
    config: &ProjectConfig,
    llm_config: &hydra_model::LlmConfig,
    project_dir: &Path,
    template_context: &str,
) -> Result<Vec<SpecGap>, String> {
    let workspace_ctx = gather_workspace_context(project_dir);
    let tools_list = config.tools.iter()
        .map(|t| format!("- {}", t))
        .collect::<Vec<_>>()
        .join("\n");

    let user_content = format!(
        "## New Project: {} (key: {})\n\n## Tools to implement:\n{}\n\n## Project Structure\n{}\n\n{}\n\n## Spec\n{}",
        config.name, config.key, tools_list, workspace_ctx, template_context,
        &spec[..spec.len().min(3000)],
    );

    let response = call_llm(&user_content, PROJECT_GAP_PROMPT, 2000, llm_config).await?;
    let gaps = parse_gaps_from_response(&response);

    if gaps.is_empty() {
        // Scaffold already covers everything — this is fine.
        // Generate one minimal gap so the pipeline still runs (adds extra tests).
        let fallback = vec![SpecGap {
            description: "Add additional integration tests for MCP protocol compliance".into(),
            target_file: format!("crates/agentic-{}-mcp/src/tools/registry.rs", config.key),
            gap_type: crate::self_modify::GapType::MissingTest,
            priority: 3,
        }];
        eprintln!("[hydra:project] Scaffold complete, adding test gap only");
        Ok(fallback)
    } else {
        eprintln!("[hydra:project] LLM identified {} project gaps", gaps.len());
        Ok(gaps)
    }
}

/// Generate patches for a new project (tool handler implementations).
pub async fn generate_project_patches(
    gaps: &[SpecGap],
    spec: &str,
    config: &ProjectConfig,
    llm_config: &hydra_model::LlmConfig,
    project_dir: &Path,
    template_context: &str,
) -> Result<Vec<Patch>, String> {
    let workspace_ctx = gather_workspace_context(project_dir);

    // Read existing files for context
    let mut existing_code = String::new();
    let registry_path = project_dir.join(format!(
        "crates/agentic-{}-mcp/src/tools/registry.rs", config.key
    ));
    if let Ok(content) = std::fs::read_to_string(&registry_path) {
        existing_code.push_str(&format!(
            "### Existing: tools/registry.rs\n```rust\n{}\n```\n\n",
            &content[..content.len().min(2000)]
        ));
    }
    let core_path = project_dir.join(format!(
        "crates/agentic-{}/src/lib.rs", config.key
    ));
    if let Ok(content) = std::fs::read_to_string(&core_path) {
        existing_code.push_str(&format!(
            "### Existing: core lib.rs\n```rust\n{}\n```\n\n",
            &content[..content.len().min(2000)]
        ));
    }

    let gaps_json = serde_json::to_string_pretty(gaps).unwrap_or_default();
    let user_content = format!(
        "## Project: {} (key: {})\n\n## Project Structure\n{}\n\n## Existing Code\n{}\n\n{}\n\n## Spec\n{}\n\n## Gaps\n{}",
        config.name, config.key, workspace_ctx, existing_code,
        template_context, &spec[..spec.len().min(2000)], gaps_json,
    );

    let response = call_llm(&user_content, PROJECT_PATCH_PROMPT, 4000, llm_config).await?;
    let patches = parse_patches_from_response(&response, gaps);

    if patches.is_empty() {
        Err(format!(
            "LLM returned no patches for project. Raw: {}",
            &response[..response.len().min(200)]
        ))
    } else {
        eprintln!("[hydra:project] LLM generated {} project patches", patches.len());
        Ok(patches)
    }
}

/// Generate code for a SINGLE build phase.
/// Small, focused LLM call. Context includes actual compiled code from prior phases.
pub async fn generate_phase_code(
    phase: &crate::project_creation_phases::BuildPhase,
    spec: &str,
    config: &ProjectConfig,
    llm_config: &hydra_model::LlmConfig,
    project_dir: &Path,
    completed_context: &str,
) -> Result<Patch, String> {
    let phase_prompt = format!(
        r#"You are implementing ONE specific phase of a Rust project.

## Your task
{description}

## Target file
{target_file}

## Rules
- Write COMPLETE file content for {target_file} (not a diff)
- The code must compile with `cargo check`
- Keep under 400 lines
- Use types/functions from the already-compiled code shown below
- Include unit tests in the same file using #[cfg(test)] mod tests
- Return ONLY a JSON object (no markdown fences):
  {{"target_file": "{target_file}", "diff_content": "complete Rust source code", "description": "what this implements"}}
- Return exactly ONE JSON object, not an array"#,
        description = phase.description,
        target_file = phase.target_file,
    );

    let user_content = format!(
        "## Project: {} (key: {})\n\n## Compiled code from previous phases\n{}\n\n## Spec\n{}",
        config.name, config.key, completed_context,
        &spec[..spec.len().min(2000)],
    );

    let response = call_llm(&user_content, &phase_prompt, 2000, llm_config).await?;

    // Parse single JSON object (not array)
    parse_single_patch(&response, &phase.target_file, &phase.description)
}

/// Parse a single patch from LLM response (not an array).
fn parse_single_patch(response: &str, target_file: &str, description: &str) -> Result<Patch, String> {
    // Try parsing as JSON object first
    let cleaned = response.trim()
        .trim_start_matches("```json").trim_start_matches("```")
        .trim_end_matches("```").trim();

    if let Ok(val) = serde_json::from_str::<serde_json::Value>(cleaned) {
        if let Some(obj) = val.as_object() {
            let file = obj.get("target_file")
                .and_then(|v| v.as_str())
                .unwrap_or(target_file);
            let content = obj.get("diff_content")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if !content.is_empty() {
                return Ok(Patch {
                    target_file: file.to_string(),
                    diff_content: content.to_string(),
                    description: description.to_string(),
                    gap: SpecGap {
                        description: description.to_string(),
                        target_file: file.to_string(),
                        gap_type: crate::self_modify::GapType::MissingFunction,
                        priority: 1,
                    },
                    touches_critical: false,
                });
            }
        }
        // Try as array with one element
        if let Some(arr) = val.as_array() {
            if let Some(obj) = arr.first().and_then(|v| v.as_object()) {
                let file = obj.get("target_file")
                    .and_then(|v| v.as_str())
                    .unwrap_or(target_file);
                let content = obj.get("diff_content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if !content.is_empty() {
                    return Ok(Patch {
                        target_file: file.to_string(),
                        diff_content: content.to_string(),
                        description: description.to_string(),
                        gap: SpecGap {
                            description: description.to_string(),
                            target_file: file.to_string(),
                            gap_type: crate::self_modify::GapType::MissingFunction,
                            priority: 1,
                        },
                        touches_critical: false,
                    });
                }
            }
        }
    }

    Err(format!("Could not parse phase response: {}", &response[..response.len().min(200)]))
}

/// Fix a SINGLE phase that failed to compile.
pub async fn fix_phase_code(
    failed_code: &str,
    compile_errors: &str,
    phase: &crate::project_creation_phases::BuildPhase,
    config: &ProjectConfig,
    llm_config: &hydra_model::LlmConfig,
    completed_context: &str,
) -> Result<Patch, String> {
    let fix_prompt = format!(
        r#"Fix this Rust code that failed to compile.

## Target file: {target_file}
## Errors
```
{errors}
```

## Rules
- Return the COMPLETE corrected file content
- Fix ONLY the errors shown — don't rewrite unrelated code
- Return ONLY a JSON object:
  {{"target_file": "{target_file}", "diff_content": "corrected code", "description": "what was fixed"}}"#,
        target_file = phase.target_file,
        errors = &compile_errors[..compile_errors.len().min(500)],
    );

    let user_content = format!(
        "## Failed code\n```rust\n{}\n```\n\n## Working code from previous phases\n{}",
        &failed_code[..failed_code.len().min(2000)],
        completed_context,
    );

    let response = call_llm(&user_content, &fix_prompt, 2000, llm_config).await?;
    parse_single_patch(&response, &phase.target_file, "compile fix")
}

/// Fix patches that failed compilation in a new project.
pub async fn fix_project_compile_errors(
    original_patches: &[Patch],
    compile_errors: &str,
    spec: &str,
    config: &ProjectConfig,
    llm_config: &hydra_model::LlmConfig,
    project_dir: &Path,
) -> Result<Vec<Patch>, String> {
    let workspace_ctx = gather_workspace_context(project_dir);
    let patches_desc: String = original_patches.iter()
        .map(|p| format!("### {}\n```rust\n{}\n```", p.target_file, p.diff_content))
        .collect::<Vec<_>>()
        .join("\n\n");

    let user_content = format!(
        "## Project: {} (key: {})\n\n## Structure\n{}\n\n## Patches (reverted)\n{}\n\n## Errors\n```\n{}\n```\n\n## Spec\n{}",
        config.name, config.key, workspace_ctx, patches_desc,
        compile_errors, &spec[..spec.len().min(1500)],
    );

    let response = call_llm(&user_content, PROJECT_FIX_PROMPT, 4000, llm_config).await?;
    let gaps: Vec<SpecGap> = original_patches.iter().map(|p| p.gap.clone()).collect();
    let patches = parse_patches_from_response(&response, &gaps);

    if patches.is_empty() {
        Err("LLM returned no corrected patches for project".into())
    } else {
        eprintln!("[hydra:project] LLM generated {} corrected project patches", patches.len());
        Ok(patches)
    }
}
