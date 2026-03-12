//! LLM-powered gap analysis and patch generation for the self-modification pipeline.
//!
//! Strategy: Try Forge sister first → fall back to direct LLM (Haiku/GPT-4o-mini).
//! Uses the same micro-LLM pattern as the intent classifier.

use std::path::PathBuf;

use crate::self_modify::{GapType, Patch, SpecGap};

// ═══════════════════════════════════════════════════════════
// SPEC FILE EXTRACTION
// ═══════════════════════════════════════════════════════════

/// Extract a spec file path from user text (e.g., "implement spec test-specs/FOO.md").
pub fn extract_spec_path(text: &str) -> Option<PathBuf> {
    // Look for a path ending in .md or .txt
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

// ═══════════════════════════════════════════════════════════
// GAP ANALYSIS — Forge first, LLM fallback
// ═══════════════════════════════════════════════════════════

/// System prompt for LLM-based gap analysis.
const GAP_ANALYSIS_PROMPT: &str = r#"You are analyzing a software spec for Hydra, a Rust AI agent built with a workspace of crates.
Given the spec below, identify what code needs to be implemented.

Return ONLY a JSON array (no markdown fences). Each element:
{"description": "what is missing", "target_file": "crates/crate-name/src/file.rs", "gap_type": "missing_function", "priority": 1}

gap_type must be one of: missing_function, missing_module, missing_test, missing_integration, incomplete_implementation
priority: 1 = critical, 2 = important, 3 = nice-to-have

Focus on concrete code gaps. Use the Implementation Location section if present.
If the spec mentions slash commands, target the slash command handler.
Keep the array short — max 5 gaps."#;

/// Analyze a spec to find implementation gaps.
/// Tries Forge sister first, falls back to direct LLM call.
pub async fn analyze_spec_gaps(
    spec: &str,
    forge_result: Option<Result<serde_json::Value, String>>,
    llm_config: &hydra_model::LlmConfig,
) -> Result<Vec<SpecGap>, String> {
    // Strategy 1: Use Forge sister result if provided
    if let Some(Ok(forge_response)) = forge_result {
        if let Some(gaps) = parse_gaps_from_json(&forge_response) {
            if !gaps.is_empty() {
                eprintln!("[hydra:self-impl] Forge returned {} gaps", gaps.len());
                return Ok(gaps);
            }
        }
    }

    // Strategy 2: Direct LLM call
    let response = call_llm(spec, GAP_ANALYSIS_PROMPT, 2000, llm_config).await?;
    let gaps = parse_gaps_from_response(&response);

    if gaps.is_empty() {
        Err("LLM returned no actionable gaps from spec".into())
    } else {
        eprintln!("[hydra:self-impl] LLM identified {} gaps", gaps.len());
        Ok(gaps)
    }
}

// ═══════════════════════════════════════════════════════════
// PATCH GENERATION — Forge first, LLM fallback
// ═══════════════════════════════════════════════════════════

/// System prompt for LLM-based patch generation.
const PATCH_GEN_PROMPT: &str = r#"You are generating Rust code for Hydra, an AI agent.
Given the spec and identified gaps, generate code patches.

Rules:
- Write valid, compilable Rust code
- Additive only — new functions, new modules, new match arms
- Follow standard Rust conventions (snake_case, pub visibility)
- Keep each patch under 50 lines of code
- No unsafe code, no unwrap() on user input, no panics
- Include necessary use/import statements at the top of each patch

Return ONLY a JSON array (no markdown fences). Each element:
{"target_file": "crates/.../src/file.rs", "diff_content": "the actual Rust code to append", "description": "what this patch does"}

Max 5 patches."#;

/// Generate code patches for identified gaps.
/// Tries Forge sister first, falls back to direct LLM call.
pub async fn generate_patches(
    gaps: &[SpecGap],
    spec: &str,
    forge_result: Option<Result<serde_json::Value, String>>,
    llm_config: &hydra_model::LlmConfig,
) -> Result<Vec<Patch>, String> {
    // Strategy 1: Use Forge sister result if provided
    if let Some(Ok(forge_response)) = forge_result {
        if let Some(patches) = parse_patches_from_json(&forge_response, gaps) {
            if !patches.is_empty() {
                eprintln!("[hydra:self-impl] Forge generated {} patches", patches.len());
                return Ok(patches);
            }
        }
    }

    // Strategy 2: Direct LLM call
    let gaps_json = serde_json::to_string_pretty(gaps).unwrap_or_default();
    let user_content = format!("Spec:\n{}\n\nGaps to fill:\n{}", spec, gaps_json);
    let response = call_llm(&user_content, PATCH_GEN_PROMPT, 4000, llm_config).await?;
    eprintln!("[hydra:self-impl] patch LLM raw response ({} chars): {}", response.len(), &response[..response.len().min(500)]);
    let patches = parse_patches_from_response(&response, gaps);

    if patches.is_empty() {
        Err(format!("LLM returned no parseable patches. Raw: {}", &response[..response.len().min(200)]))
    } else {
        eprintln!("[hydra:self-impl] LLM generated {} patches", patches.len());
        Ok(patches)
    }
}

// ═══════════════════════════════════════════════════════════
// LLM CALLING — reuses intent classifier pattern
// ═══════════════════════════════════════════════════════════

/// Pick the cheapest available model.
/// Prefers real API keys (sk-ant-, sk-) over OAuth tokens which may be expired.
/// Respects HYDRA_MODEL env var for OpenAI-compatible providers.
fn pick_cheapest_model(config: &hydra_model::LlmConfig) -> (String, &'static str) {
    let has_real_anthropic = config.anthropic_api_key.as_ref()
        .map_or(false, |k| k.starts_with("sk-ant-"));
    let has_openai = config.openai_api_key.is_some();

    // Prefer real Anthropic key (cheapest), then OpenAI, then OAuth Anthropic as last resort
    if has_real_anthropic {
        ("claude-haiku-4-5-20251001".into(), "anthropic")
    } else if has_openai {
        // Only use HYDRA_MODEL if it's OpenAI-compatible (not claude-*)
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

        // If primary fails and the other provider is available, try it
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

    // 45s timeout — code generation takes longer than classification
    match tokio::time::timeout(std::time::Duration::from_secs(45), call_future).await {
        Ok(result) => result,
        Err(_) => Err("LLM call timed out after 45s".into()),
    }
}

// ═══════════════════════════════════════════════════════════
// JSON PARSING
// ═══════════════════════════════════════════════════════════

/// Strip markdown fences from LLM response.
fn strip_fences(response: &str) -> &str {
    let trimmed = response.trim();
    let inner = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .and_then(|s| s.strip_suffix("```"))
        .unwrap_or(trimmed);
    inner.trim()
}

/// Parse gap analysis from LLM text response.
pub fn parse_gaps_from_response(response: &str) -> Vec<SpecGap> {
    let json_str = strip_fences(response);
    parse_gaps_json_str(json_str)
}

/// Parse gaps from a Forge sister JSON response.
fn parse_gaps_from_json(value: &serde_json::Value) -> Option<Vec<SpecGap>> {
    // Forge may return {content: [{type: "text", text: "..."}]}
    let text = extract_mcp_text(value)?;
    Some(parse_gaps_json_str(&text))
}

fn parse_gaps_json_str(json_str: &str) -> Vec<SpecGap> {
    let arr: Vec<serde_json::Value> = match serde_json::from_str(json_str) {
        Ok(a) => a,
        Err(_) => return Vec::new(),
    };

    arr.into_iter()
        .filter_map(|v| {
            // description and target_file are required
            let desc = v.get("description")?.as_str()?.to_string();
            let file = v.get("target_file")?.as_str()?.to_string();
            // gap_type and priority are optional — LLMs often omit them
            let gap_str = v.get("gap_type")
                .and_then(|g| g.as_str())
                .unwrap_or("missing_function");
            let priority = v.get("priority")
                .and_then(|p| p.as_u64())
                .unwrap_or(1) as u8;

            let gap_type = match gap_str {
                "missing_module" => GapType::MissingModule,
                "missing_test" => GapType::MissingTest,
                "missing_integration" => GapType::MissingIntegration,
                "incomplete_implementation" => GapType::IncompleteImplementation,
                _ => GapType::MissingFunction,
            };

            Some(SpecGap { description: desc, target_file: file, gap_type, priority })
        })
        .take(5)
        .collect()
}

/// Parse patches from LLM text response.
/// Handles common LLM JSON issues: literal newlines in strings, truncated arrays.
pub fn parse_patches_from_response(response: &str, gaps: &[SpecGap]) -> Vec<Patch> {
    let json_str = strip_fences(response);
    // Try direct parse first
    let patches = parse_patches_json_str(json_str, gaps);
    if !patches.is_empty() {
        return patches;
    }
    // LLMs often put literal newlines in "diff_content" — repair by escaping them
    let repaired = repair_json_newlines(json_str);
    parse_patches_json_str(&repaired, gaps)
}

/// Repair JSON with literal newlines inside string values.
/// Walks the string and escapes raw newlines between unescaped quotes.
fn repair_json_newlines(input: &str) -> String {
    let mut result = String::with_capacity(input.len() + 100);
    let mut in_string = false;
    let mut prev_backslash = false;
    for ch in input.chars() {
        if ch == '"' && !prev_backslash {
            in_string = !in_string;
            result.push(ch);
        } else if ch == '\n' && in_string {
            result.push_str("\\n");
        } else if ch == '\r' && in_string {
            // skip carriage returns
        } else if ch == '\t' && in_string {
            result.push_str("\\t");
        } else {
            result.push(ch);
        }
        prev_backslash = ch == '\\' && !prev_backslash;
    }
    result
}

/// Parse patches from a Forge sister JSON response.
fn parse_patches_from_json(value: &serde_json::Value, gaps: &[SpecGap]) -> Option<Vec<Patch>> {
    let text = extract_mcp_text(value)?;
    Some(parse_patches_json_str(&text, gaps))
}

fn parse_patches_json_str(json_str: &str, gaps: &[SpecGap]) -> Vec<Patch> {
    let arr: Vec<serde_json::Value> = match serde_json::from_str(json_str) {
        Ok(a) => a,
        Err(_) => return Vec::new(),
    };

    arr.into_iter()
        .enumerate()
        .filter_map(|(i, v)| {
            let target = v.get("target_file")?.as_str()?.to_string();
            let diff = v.get("diff_content")?.as_str()?.to_string();
            let desc = v.get("description")?.as_str().unwrap_or("patch").to_string();

            let gap = gaps.get(i).cloned().unwrap_or(SpecGap {
                description: desc.clone(),
                target_file: target.clone(),
                gap_type: GapType::MissingFunction,
                priority: 1,
            });

            Some(Patch {
                target_file: target,
                gap,
                diff_content: diff,
                description: desc,
                touches_critical: false,
            })
        })
        .take(5)
        .collect()
}

/// Extract text from MCP tool response format.
fn extract_mcp_text(value: &serde_json::Value) -> Option<String> {
    // Try direct text field
    if let Some(text) = value.as_str() {
        return Some(text.to_string());
    }
    // Try {content: [{type: "text", text: "..."}]}
    value
        .get("content")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|item| item.get("text"))
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
}

#[cfg(test)]
#[path = "self_modify_llm_tests.rs"]
mod tests;
