//! LLM-powered gap analysis and patch generation for the self-modification pipeline.
//!
//! Strategy: Try Forge sister first -> fall back to direct LLM (Haiku/GPT-4o-mini).
//! Uses the same micro-LLM pattern as the intent classifier.
//! Injects workspace structure context so the LLM targets real crates and files.

use std::path::{Path, PathBuf};

use crate::self_modify::{Patch, SpecGap};
pub use crate::self_modify_llm_parse::{
    extract_mcp_text, parse_gaps_from_response, parse_gaps_json_str,
    parse_patches_from_response,
};

// ---- re-export parse helpers used by tests ----
use crate::self_modify_llm_parse::{parse_gaps_from_json, parse_patches_from_json};

// ===============================================================
// SPEC FILE EXTRACTION
// ===============================================================

/// Extract a spec file path from user text (e.g., "implement spec test-specs/FOO.md").
pub fn extract_spec_path(text: &str) -> Option<PathBuf> {
    for word in text.split_whitespace() {
        let trimmed = word.trim_matches(|c: char| c == '"' || c == '\'' || c == '`');
        if (trimmed.ends_with(".md") || trimmed.ends_with(".txt"))
            && (trimmed.contains('/') || trimmed.contains('\\'))
        {
            return Some(PathBuf::from(trimmed));
        }
    }
    None
}

// ===============================================================
// CODEBASE CONTEXT GATHERING
// ===============================================================

/// Build a compact workspace map for LLM context.
/// Reads root Cargo.toml to list workspace members, then lists src/*.rs for each.
/// Output kept under 2000 chars to fit in prompts without blowing token budgets.
pub fn gather_workspace_context(project_dir: &Path) -> String {
    let cargo_path = project_dir.join("Cargo.toml");
    let cargo_content = match std::fs::read_to_string(&cargo_path) {
        Ok(c) => c,
        Err(_) => return String::from("(workspace Cargo.toml not found)"),
    };

    let mut members = Vec::new();
    let mut in_members = false;
    for line in cargo_content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("members") && trimmed.contains('[') {
            in_members = true;
            continue;
        }
        if in_members {
            if trimmed.contains(']') { break; }
            let member = trimmed.trim_matches(|c: char| {
                c == '"' || c == '\'' || c == ',' || c.is_whitespace() || c == '#'
            });
            // skip comment-only lines
            if trimmed.starts_with('#') || member.is_empty() { continue; }
            // strip inline comments
            let member = member.split('#').next().unwrap_or(member).trim_matches(|c: char| {
                c == '"' || c == '\'' || c == ',' || c.is_whitespace()
            });
            if !member.is_empty() {
                members.push(member.to_string());
            }
        }
    }

    let mut result = String::from("Workspace crates:\n");
    let mut total_len = result.len();

    for member in &members {
        let src_dir = project_dir.join(member).join("src");
        let files: Vec<String> = std::fs::read_dir(&src_dir)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path().extension().map_or(false, |ext| ext == "rs")
            })
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();

        let crate_name = member.rsplit('/').next().unwrap_or(member);
        let line = format!("- {}: {}\n", crate_name, files.join(", "));

        if total_len + line.len() > 1950 {
            result.push_str("... (truncated)\n");
            break;
        }
        total_len += line.len();
        result.push_str(&line);
    }

    result
}

/// Read the first 100 lines of a target file for LLM context.
/// Returns None if the file does not exist.
pub fn read_target_context(project_dir: &Path, target_file: &str) -> Option<String> {
    let path = project_dir.join(target_file);
    let content = std::fs::read_to_string(&path).ok()?;
    let lines: Vec<&str> = content.lines().take(100).collect();
    if lines.is_empty() {
        return None;
    }
    Some(lines.join("\n"))
}

// ===============================================================
// GAP ANALYSIS -- Forge first, LLM fallback
// ===============================================================

const GAP_ANALYSIS_PROMPT: &str = r#"You are analyzing a software spec for Hydra, a Rust AI agent.

CRITICAL RULES:
- Hydra is a Cargo workspace. Only target files in EXISTING crates listed below.
- NEVER create new crates. Target existing crates.
- Max 400 lines per .rs file. If a target file is near 400 lines, target a new sibling file instead.
- hydra-native depends ONLY on hydra-native-state and hydra-native-cognitive. NEVER add other hydra-* deps to it.
- New cognitive features go in hydra-native-cognitive.
- New DB methods go in hydra-db.
- New kernel logic goes in hydra-kernel.
- Tests go inline (#[cfg(test)] mod tests) or in existing tests/suite/.

Given the workspace structure and spec below, identify what code needs to be implemented.

Return ONLY a JSON array (no markdown fences). Each element:
{"description": "what is missing", "target_file": "crates/existing-crate/src/file.rs", "gap_type": "missing_function", "priority": 1}

gap_type: missing_function, missing_module, missing_test, missing_integration, incomplete_implementation
priority: 1 = critical, 2 = important, 3 = nice-to-have
Max 10 gaps."#;

/// Analyze a spec to find implementation gaps.
/// Tries Forge sister first, falls back to direct LLM call.
pub async fn analyze_spec_gaps(
    spec: &str,
    forge_result: Option<Result<serde_json::Value, String>>,
    llm_config: &hydra_model::LlmConfig,
    project_dir: &Path,
) -> Result<Vec<SpecGap>, String> {
    if let Some(Ok(forge_response)) = forge_result {
        if let Some(gaps) = parse_gaps_from_json(&forge_response) {
            if !gaps.is_empty() {
                eprintln!("[hydra:self-impl] Forge returned {} gaps", gaps.len());
                return Ok(gaps);
            }
        }
    }

    let workspace_ctx = gather_workspace_context(project_dir);
    let user_content = format!(
        "## Workspace Structure\n{}\n\n## Spec\n{}",
        workspace_ctx, spec
    );
    let response = call_llm(&user_content, GAP_ANALYSIS_PROMPT, 2000, llm_config).await?;
    let gaps = parse_gaps_from_response(&response);

    if gaps.is_empty() {
        Err("LLM returned no actionable gaps from spec".into())
    } else {
        eprintln!("[hydra:self-impl] LLM identified {} gaps", gaps.len());
        Ok(gaps)
    }
}

// ===============================================================
// PATCH GENERATION -- Forge first, LLM fallback
// ===============================================================

const PATCH_GEN_PROMPT: &str = r#"You are generating Rust code for Hydra, an AI agent workspace.

CRITICAL RULES:
- Write valid, compilable Rust code that fits the existing codebase
- Each target file MUST be in an existing crate (see workspace structure)
- Max 400 lines per file. If the target file already has content, keep total under 400
- Include all necessary `use` imports
- Follow existing patterns (snake_case, pub(crate) for internal APIs)
- No unsafe code, no unwrap() on user input
- Keep each patch under 80 lines of code
- For new files: include a module doc comment (//!) at the top

Return ONLY a JSON array (no markdown fences). Each element:
{"target_file": "crates/.../src/file.rs", "diff_content": "the actual Rust code", "description": "what this patch does"}
Max 10 patches."#;

/// Generate code patches for identified gaps.
/// Tries Forge sister first, falls back to direct LLM call.
pub async fn generate_patches(
    gaps: &[SpecGap],
    spec: &str,
    forge_result: Option<Result<serde_json::Value, String>>,
    llm_config: &hydra_model::LlmConfig,
    project_dir: &Path,
) -> Result<Vec<Patch>, String> {
    if let Some(Ok(forge_response)) = forge_result {
        if let Some(patches) = parse_patches_from_json(&forge_response, gaps) {
            if !patches.is_empty() {
                eprintln!("[hydra:self-impl] Forge generated {} patches", patches.len());
                return Ok(patches);
            }
        }
    }

    let workspace_ctx = gather_workspace_context(project_dir);
    let mut target_ctx = String::new();
    for gap in gaps {
        if let Some(content) = read_target_context(project_dir, &gap.target_file) {
            target_ctx.push_str(&format!(
                "\n### Existing: {}\n```rust\n{}\n```\n",
                gap.target_file, content
            ));
        }
    }

    let gaps_json = serde_json::to_string_pretty(gaps).unwrap_or_default();
    let user_content = format!(
        "## Workspace\n{}\n\n## Existing Code\n{}\n\n## Spec\n{}\n\n## Gaps\n{}",
        workspace_ctx, target_ctx, spec, gaps_json
    );
    let response = call_llm(&user_content, PATCH_GEN_PROMPT, 4000, llm_config).await?;
    eprintln!(
        "[hydra:self-impl] patch LLM raw response ({} chars): {}",
        response.len(),
        &response[..response.len().min(500)]
    );
    let patches = parse_patches_from_response(&response, gaps);

    if patches.is_empty() {
        Err(format!(
            "LLM returned no parseable patches. Raw: {}",
            &response[..response.len().min(200)]
        ))
    } else {
        eprintln!("[hydra:self-impl] LLM generated {} patches", patches.len());
        Ok(patches)
    }
}

// ===============================================================
// LLM CALLING -- reuses intent classifier pattern
// ===============================================================

/// Pick the cheapest available model.
/// Prefers real API keys (sk-ant-, sk-) over OAuth tokens which may be expired.
/// Respects HYDRA_MODEL env var for OpenAI-compatible providers.
pub(crate) fn pick_cheapest_model(config: &hydra_model::LlmConfig) -> (String, &'static str) {
    let has_real_anthropic = config.anthropic_api_key.as_ref()
        .map_or(false, |k| k.starts_with("sk-ant-"));
    let has_openai = config.openai_api_key.is_some();

    if has_real_anthropic {
        ("claude-haiku-4-5-20251001".into(), "anthropic")
    } else if has_openai {
        let model = std::env::var("HYDRA_MODEL")
            .ok()
            .filter(|m| !m.starts_with("claude"))
            .unwrap_or_else(|| "gpt-4o-mini".into());
        (model, "openai")
    } else if config.anthropic_api_key.is_some() {
        ("claude-haiku-4-5-20251001".into(), "anthropic")
    } else {
        (String::new(), "none")
    }
}

/// Try a single provider call.
async fn try_provider(
    provider: &str,
    request: hydra_model::CompletionRequest,
    llm_config: &hydra_model::LlmConfig,
) -> Result<String, String> {
    match provider {
        "anthropic" => {
            let client = hydra_model::providers::anthropic::AnthropicClient::new(llm_config)
                .map_err(|e| format!("{}", e))?;
            client.complete(request).await.map(|r| r.content).map_err(|e| format!("{}", e))
        }
        "openai" => {
            let client = hydra_model::providers::openai::OpenAiClient::new(llm_config)
                .map_err(|e| format!("{}", e))?;
            client.complete(request).await.map(|r| r.content).map_err(|e| format!("{}", e))
        }
        _ => Err("No provider".into()),
    }
}

/// Build a CompletionRequest with the given model.
fn build_request(
    model: &str, user_content: &str, system_prompt: &str, max_tokens: u32,
) -> hydra_model::CompletionRequest {
    hydra_model::CompletionRequest {
        model: model.to_string(),
        messages: vec![hydra_model::providers::Message {
            role: "user".into(),
            content: user_content.to_string(),
        }],
        max_tokens,
        temperature: Some(0.0),
        system: Some(system_prompt.to_string()),
    }
}

/// Make a direct LLM call. Tries preferred provider first, falls back to the other.
pub async fn call_llm(
    user_content: &str,
    system_prompt: &str,
    max_tokens: u32,
    llm_config: &hydra_model::LlmConfig,
) -> Result<String, String> {
    let (model, provider) = pick_cheapest_model(llm_config);
    if model.is_empty() {
        return Err("No LLM API key available".into());
    }

    let call_future = async {
        let request = build_request(&model, user_content, system_prompt, max_tokens);
        let result = try_provider(provider, request, llm_config).await;

        if result.is_err() {
            let fallback = match provider {
                "anthropic" if llm_config.openai_api_key.is_some() => {
                    let fb_model = std::env::var("HYDRA_MODEL")
                        .unwrap_or_else(|_| "gpt-4o-mini".into());
                    Some((fb_model, "openai"))
                }
                "openai" if llm_config.anthropic_api_key.is_some() => {
                    Some(("claude-haiku-4-5-20251001".into(), "anthropic"))
                }
                _ => None,
            };
            if let Some((fb_model, fb_provider)) = fallback {
                eprintln!("[hydra:self-impl] Primary ({}) failed, trying {}", provider, fb_provider);
                let fb_req = build_request(&fb_model, user_content, system_prompt, max_tokens);
                let fb_result = try_provider(fb_provider, fb_req, llm_config).await;
                if fb_result.is_ok() {
                    return fb_result;
                }
            }
        }
        result
    };

    match tokio::time::timeout(std::time::Duration::from_secs(45), call_future).await {
        Ok(result) => result,
        Err(_) => Err("LLM call timed out after 45s".into()),
    }
}

// ---- ERROR CORRECTION -- fix patches that failed cargo check ----

const ERROR_CORRECTION_PROMPT: &str = r#"You are fixing Rust code that failed to compile.
Given the original patches, cargo check errors, and workspace structure, fix the patches.
Common issues: missing imports, wrong types, missing deps, wrong module paths.
Return ONLY a JSON array: {"target_file": "crates/.../src/file.rs", "diff_content": "corrected Rust code", "description": "what was fixed"}
Return COMPLETE corrected code for each file (previous code was reverted)."#;

/// Fix patches that failed compilation by feeding errors to the LLM.
pub async fn fix_compile_errors(
    original_patches: &[Patch], compile_errors: &str, spec: &str,
    llm_config: &hydra_model::LlmConfig, project_dir: &Path,
) -> Result<Vec<Patch>, String> {
    let workspace_ctx = gather_workspace_context(project_dir);
    let patches_desc: String = original_patches.iter()
        .map(|p| format!("### {}\n```rust\n{}\n```", p.target_file, p.diff_content))
        .collect::<Vec<_>>().join("\n\n");
    let user_content = format!(
        "## Workspace\n{}\n\n## Original Patches (reverted)\n{}\n\n## Compile Errors\n```\n{}\n```\n\n## Original Spec\n{}",
        workspace_ctx, patches_desc, compile_errors, &spec[..spec.len().min(2000)]
    );
    let response = call_llm(&user_content, ERROR_CORRECTION_PROMPT, 4000, llm_config).await?;
    let gaps: Vec<SpecGap> = original_patches.iter().map(|p| p.gap.clone()).collect();
    let patches = parse_patches_from_response(&response, &gaps);
    if patches.is_empty() {
        Err("LLM returned no corrected patches".into())
    } else {
        eprintln!("[hydra:self-impl] LLM generated {} corrected patches", patches.len());
        Ok(patches)
    }
}

#[cfg(test)]
#[path = "self_modify_llm_tests.rs"]
mod tests;
