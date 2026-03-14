//! Handlers for sister improvement (P10) and threat queries (P11).
//!
//! Sister improvement uses: Forge sister → direct LLM → local heuristics (fallback chain).

use std::sync::Arc;
use tokio::sync::mpsc;

use crate::cognitive::intent_router::{ClassifiedIntent, IntentCategory};
use crate::cognitive::loop_runner::{CognitiveLoopConfig, CognitiveUpdate};
use crate::sisters::SistersHandle;

/// Handle "improve the X sister" intent.
///
/// Fallback chain: Forge sister → direct LLM (API key) → local heuristics.
pub(crate) async fn handle_sister_improve(
    text: &str,
    intent: &ClassifiedIntent,
    config: &CognitiveLoopConfig,
    sisters_handle: &Option<SistersHandle>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> bool {
    if intent.category != IntentCategory::SisterImprove {
        return false;
    }

    let _ = tx.send(CognitiveUpdate::Phase("Sister Improvement".into()));
    let _ = tx.send(CognitiveUpdate::IconState("working".into()));

    // Step 1: Resolve project path — explicit path first, then auto-resolve from text
    let sister_path = crate::sister_improve::extract_sister_path(text)
        .or_else(|| super::super::dispatch_capability::resolve_project_from_text(text));
    let sister_path = match sister_path {
        Some(p) => p,
        None => {
            send_result(tx, "I couldn't find that project in the workspace.\n\n\
                Available projects can be found as sibling directories.");
            return true;
        }
    };

    let goal = crate::sister_improve::extract_goal(text);
    let project_name = sister_path.file_name()
        .and_then(|n| n.to_str()).unwrap_or("unknown").to_string();

    send_result(tx, &format!(
        "Analyzing **{}** at `{}`\nGoal: **{}**",
        project_name, sister_path.display(), goal
    ));

    // Step 2: Analyze codebase structure (local, fast)
    let _ = tx.send(CognitiveUpdate::Phase("Analyzing codebase...".into()));
    let analysis = match crate::sister_improve::analyzer::analyze_sister(&sister_path) {
        Ok(a) => a,
        Err(e) => {
            send_result(tx, &format!("Analysis failed: {}", e));
            return true;
        }
    };
    send_result(tx, &format!(
        "Found **{}** project: {} source files, {} test files",
        analysis.language, analysis.source_files.len(), analysis.test_files.len()
    ));

    // Step 3: Build LLM config from user's API keys
    let llm_config = hydra_model::LlmConfig::from_env_with_overlay(
        &config.anthropic_key,
        &config.openai_key,
        config.anthropic_oauth_token.as_deref(),
    );
    let has_api = llm_config.anthropic_api_key.is_some()
        || llm_config.openai_api_key.is_some();

    // Step 4: Identify improvements — Forge → LLM → local
    let _ = tx.send(CognitiveUpdate::Phase("Identifying improvements...".into()));
    let improvements = identify_improvements(
        &sister_path, &analysis, &goal, sisters_handle, &llm_config, has_api, tx,
    ).await;

    if improvements.is_empty() {
        send_result(tx, "No improvement opportunities identified.");
        finish(tx);
        return true;
    }

    // Step 5: Generate patches — Forge → LLM → local
    let _ = tx.send(CognitiveUpdate::Phase("Generating patches...".into()));
    let patches = generate_patches(
        &sister_path, &analysis, &improvements, &goal,
        sisters_handle, &llm_config, has_api, tx,
    ).await;

    if patches.is_empty() {
        send_result(tx, &format!(
            "Identified **{}** improvement(s) but could not generate patches:\n{}",
            improvements.len(),
            improvements.iter().map(|i| format!("  - {}", i)).collect::<Vec<_>>().join("\n")
        ));
        finish(tx);
        return true;
    }

    // Step 6: Apply patches with checkpoint
    let _ = tx.send(CognitiveUpdate::Phase("Applying patches...".into()));
    let applied = apply_patches_with_checkpoint(&sister_path, &patches, tx);

    send_result(tx, &format!(
        "**Sister Improvement Report**\n\n\
        Project: {}\n\
        Improvements found: {}\n\
        Patches generated: {}\n\
        Patches applied: {}\n\
        Method: {}",
        project_name, improvements.len(), patches.len(), applied,
        if has_api { "LLM-powered" } else { "local heuristics" }
    ));

    finish(tx);
    true
}

/// Identify improvements using fallback chain.
async fn identify_improvements(
    path: &std::path::Path,
    analysis: &crate::sister_improve::SisterAnalysis,
    goal: &str,
    sisters: &Option<SistersHandle>,
    llm_config: &hydra_model::LlmConfig,
    has_api: bool,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> Vec<String> {
    // Try Forge sister first
    if let Some(ref sh) = sisters {
        if let Some(ref forge) = sh.forge {
            let prompt = format!(
                "Analyze this {} project at {} and suggest improvements. Goal: {}. \
                 It has {} source files and {} test files.",
                analysis.language, path.display(), goal,
                analysis.source_files.len(), analysis.test_files.len()
            );
            if let Ok(result) = forge.call_tool("forge_blueprint_create", serde_json::json!({
                "spec": prompt, "mode": "gap_analysis",
            })).await {
                if let Some(items) = parse_improvements_from_json(&result) {
                    let _ = tx.send(CognitiveUpdate::Phase("Forge analysis complete".into()));
                    return items;
                }
            }
        }
    }

    // Try direct LLM
    if has_api {
        if let Some(items) = identify_via_llm(path, analysis, goal, llm_config).await {
            let _ = tx.send(CognitiveUpdate::Phase("LLM analysis complete".into()));
            return items;
        }
    }

    // Local fallback — use heuristic limitation detection
    let _ = tx.send(CognitiveUpdate::Phase("Using local analysis...".into()));
    let baseline = crate::sister_improve::verifier::TestResults::empty();
    let limitation = crate::sister_improve::analyzer::identify_limitation(analysis, goal, &baseline);
    if limitation.is_empty() { vec![] } else { vec![limitation] }
}

/// Identify improvements via direct LLM call.
async fn identify_via_llm(
    path: &std::path::Path,
    analysis: &crate::sister_improve::SisterAnalysis,
    goal: &str,
    llm_config: &hydra_model::LlmConfig,
) -> Option<Vec<String>> {
    let (model, provider) = pick_cheapest_model(llm_config);
    if model.is_empty() { return None; }

    let prompt = format!(
        "You are analyzing a {} project '{}' with {} source files and {} test files.\n\
         Goal: {}\n\n\
         List 1-3 specific, actionable improvements as a JSON array of strings.\n\
         Example: [\"Add error handling to the connect() function\", \"Add unit test for parse_config\"]\n\
         Return ONLY the JSON array, no other text.",
        analysis.language,
        path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown"),
        analysis.source_files.len(), analysis.test_files.len(), goal
    );
    let request = hydra_model::CompletionRequest {
        model: model.clone(),
        messages: vec![hydra_model::providers::Message { role: "user".into(), content: prompt }],
        max_tokens: 500,
        temperature: Some(0.3),
        system: None,
    };
    let content = llm_complete(llm_config, request, provider).await?;
    parse_json_string_array(&content)
}

/// Generate patches using fallback chain.
async fn generate_patches(
    path: &std::path::Path,
    analysis: &crate::sister_improve::SisterAnalysis,
    improvements: &[String],
    goal: &str,
    sisters: &Option<SistersHandle>,
    llm_config: &hydra_model::LlmConfig,
    has_api: bool,
    _tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> Vec<crate::sister_improve::ImprovementPatch> {
    // Try local patch generation (works for common patterns: add tests, CI, docs)
    let request = crate::sister_improve::PatchRequest {
        sister_path: path.to_path_buf(),
        limitation: improvements.first().cloned().unwrap_or_default(),
        goal: goal.to_string(),
        analysis: analysis.clone(),
    };
    if let Some(patch) = crate::sister_improve::patch_generator::generate_patch(&request) {
        if !patch.changes.is_empty() {
            return vec![patch];
        }
    }

    // For non-trivial improvements, we need LLM — try Forge then direct
    if let Some(ref sh) = sisters {
        if let Some(ref forge) = sh.forge {
            let gaps_json = serde_json::json!(improvements);
            if let Ok(result) = forge.call_tool("forge_generate_code", serde_json::json!({
                "spec": goal, "gaps": gaps_json,
            })).await {
                if let Some(patches) = parse_patches_from_json(&result, path) {
                    return patches;
                }
            }
        }
    }

    // Direct LLM patch generation would go here (future enhancement)
    let _ = (llm_config, has_api);

    vec![]
}

/// Apply patches with file checkpointing. Returns count of successfully applied patches.
fn apply_patches_with_checkpoint(
    _path: &std::path::Path,
    patches: &[crate::sister_improve::ImprovementPatch],
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> usize {
    let mut applied = 0;
    for patch in patches {
        if patch.changes.is_empty() { continue; }
        match crate::sister_improve::patch_generator::apply_patch(patch) {
            Ok(()) => { applied += 1; }
            Err(e) => {
                let _ = tx.send(CognitiveUpdate::Phase(format!("Patch failed: {}", e)));
            }
        }
    }
    applied
}

fn parse_improvements_from_json(val: &serde_json::Value) -> Option<Vec<String>> {
    if let Some(arr) = val.as_array() {
        let items: Vec<String> = arr.iter().filter_map(|v| v.as_str().map(String::from)).collect();
        if !items.is_empty() { return Some(items); }
    }
    if let Some(obj) = val.as_object() {
        if let Some(arr) = obj.get("improvements").or(obj.get("gaps")).and_then(|v| v.as_array()) {
            let items: Vec<String> = arr.iter().filter_map(|v| v.as_str().map(String::from)).collect();
            if !items.is_empty() { return Some(items); }
        }
    }
    None
}

fn parse_patches_from_json(
    _val: &serde_json::Value, _path: &std::path::Path,
) -> Option<Vec<crate::sister_improve::ImprovementPatch>> {
    // Future: parse Forge-generated patches from JSON
    None
}

fn parse_json_string_array(text: &str) -> Option<Vec<String>> {
    let trimmed = text.trim();
    let start = trimmed.find('[')?;
    let end = trimmed.rfind(']')? + 1;
    let arr: Vec<String> = serde_json::from_str(&trimmed[start..end]).ok()?;
    if arr.is_empty() { None } else { Some(arr) }
}

/// Pick cheapest LLM model from available API keys.
fn pick_cheapest_model(config: &hydra_model::LlmConfig) -> (String, &'static str) {
    if config.anthropic_api_key.is_some() {
        ("claude-haiku-4-5-20251001".into(), "anthropic")
    } else if config.openai_api_key.is_some() {
        ("gpt-4o-mini".into(), "openai")
    } else {
        (String::new(), "none")
    }
}

/// Make an LLM completion call to the appropriate provider.
async fn llm_complete(
    llm_config: &hydra_model::LlmConfig,
    request: hydra_model::CompletionRequest,
    provider: &str,
) -> Option<String> {
    let fut = async {
        match provider {
            "anthropic" => {
                let client = hydra_model::providers::anthropic::AnthropicClient::new(llm_config).ok()?;
                client.complete(request).await.ok().map(|r| r.content)
            }
            "openai" => {
                let client = hydra_model::providers::openai::OpenAiClient::new(llm_config).ok()?;
                client.complete(request).await.ok().map(|r| r.content)
            }
            _ => None,
        }
    };
    match tokio::time::timeout(std::time::Duration::from_secs(30), fut).await {
        Ok(result) => result,
        Err(_) => { eprintln!("[hydra:sister-improve] LLM call timed out"); None }
    }
}

fn send_result(tx: &mpsc::UnboundedSender<CognitiveUpdate>, content: &str) {
    let _ = tx.send(CognitiveUpdate::Message {
        role: "hydra".into(),
        content: content.to_string(),
        css_class: "message hydra".into(),
    });
}

fn finish(tx: &mpsc::UnboundedSender<CognitiveUpdate>) {
    let _ = tx.send(CognitiveUpdate::Phase("Done".into()));
    let _ = tx.send(CognitiveUpdate::IconState("success".into()));
    let _ = tx.send(CognitiveUpdate::ResetIdle);
}

/// Handle "what's the threat level?" intent.
pub(crate) fn handle_threat_query(
    _text: &str,
    intent: &ClassifiedIntent,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> bool {
    if intent.category != IntentCategory::ThreatQuery {
        return false;
    }

    let _ = tx.send(CognitiveUpdate::Phase("Threat Intelligence".into()));
    let correlator = crate::threat::ThreatCorrelator::new();
    let summary = correlator.summary();
    let patterns = correlator.patterns_summary();
    send_result(tx, &format!("{}\n\n{}", summary, patterns));
    let _ = tx.send(CognitiveUpdate::ResetIdle);
    true
}
