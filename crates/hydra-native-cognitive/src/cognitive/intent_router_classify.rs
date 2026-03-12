//! Intent classification functions — classify, parse, resolve, emergency fallback.
//!
//! Split from intent_router.rs for file-size hygiene.

use super::intent_router::{
    ClassifiedIntent, IntentCategory, CLASSIFICATION_PROMPT, SISTER_NAMES,
};
use crate::sisters::connection::SisterConnection;
use tracing::{debug, warn};

// ═══════════════════════════════════════════════════════════════════
// Universal classifier — uses a micro-LLM call to understand meaning
// ═══════════════════════════════════════════════════════════════════

/// Classify user input using a tiny LLM call (~150 tokens).
///
/// Model priority: Haiku (cheapest) → Sonnet → whatever is available.
/// Temperature: 0.0 (deterministic). Max tokens: 60. No tools.
///
/// When no LLM key is available, falls back to emergency_classify()
/// which only handles greetings and "remember X".
pub async fn classify(
    input: &str,
    _veritas: Option<&SisterConnection>,
    context: &[(String, String)],
    llm_config: &hydra_model::LlmConfig,
) -> ClassifiedIntent {
    // Build minimal context (last 2 messages, truncated)
    let recent = context.iter().rev().take(2)
        .map(|(role, msg)| format!("{}: {}", role, hydra_native_state::utils::safe_truncate(msg, 100)))
        .collect::<Vec<_>>()
        .join("\n");

    let user_content = if recent.is_empty() {
        format!("User message: {}", input)
    } else {
        format!("Recent context:\n{}\n\nUser message: {}", recent, input)
    };

    // Try cheapest model first: Haiku → Sonnet → GPT-4o-mini → whatever
    let (model, provider) = pick_cheapest_model(llm_config);

    if model.is_empty() {
        // No API key available → emergency fallback
        debug!("[hydra:intent] No LLM key — emergency classify");
        return emergency_classify(input);
    }

    let request = hydra_model::CompletionRequest {
        model: model.clone(),
        messages: vec![hydra_model::providers::Message {
            role: "user".into(),
            content: user_content,
        }],
        max_tokens: 60,
        temperature: Some(0.0),
        system: Some(CLASSIFICATION_PROMPT.to_string()),
    };

    eprintln!("[hydra:intent] Classifying with {} (~150 tokens)", model);

    let classify_future = async {
        match provider {
            "anthropic" => {
                match hydra_model::providers::anthropic::AnthropicClient::new(llm_config) {
                    Ok(client) => client.complete(request).await.map(|r| r.content).map_err(|e| format!("{}", e)),
                    Err(e) => Err(format!("{}", e)),
                }
            }
            "openai" => {
                match hydra_model::providers::openai::OpenAiClient::new(llm_config) {
                    Ok(client) => client.complete(request).await.map(|r| r.content).map_err(|e| format!("{}", e)),
                    Err(e) => Err(format!("{}", e)),
                }
            }
            _ => Err("No provider".into()),
        }
    };

    // 15s timeout for classification — it's a tiny call, should be fast
    let result = match tokio::time::timeout(std::time::Duration::from_secs(15), classify_future).await {
        Ok(r) => r,
        Err(_) => {
            eprintln!("[hydra:intent] TIMEOUT after 15s — falling back to emergency classify");
            Err("Classification timed out".into())
        }
    };

    match result {
        Ok(response) => {
            eprintln!("[hydra:intent] LLM response: {}", response.trim());
            parse_classification(&response, input)
        }
        Err(e) => {
            eprintln!("[hydra:intent] LLM classify failed: {} — emergency fallback", e);
            emergency_classify(input)
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Model selection — always pick the cheapest available model
// ═══════════════════════════════════════════════════════════════════

fn pick_cheapest_model(config: &hydra_model::LlmConfig) -> (String, &'static str) {
    if config.anthropic_api_key.is_some() {
        // Haiku: $0.001/$0.005 per 1K — cheapest Anthropic model
        ("claude-haiku-4-5-20251001".into(), "anthropic")
    } else if config.openai_api_key.is_some() {
        // GPT-4o-mini: $0.00015/$0.0006 per 1K — cheapest OpenAI
        ("gpt-4o-mini".into(), "openai")
    } else {
        (String::new(), "none")
    }
}

// ═══════════════════════════════════════════════════════════════════
// Parse the LLM's JSON response into a ClassifiedIntent
// ═══════════════════════════════════════════════════════════════════

pub(crate) fn parse_classification(response: &str, input: &str) -> ClassifiedIntent {
    // Try to extract JSON from the response
    let json_str = response.trim();

    // Handle potential markdown code blocks
    let json_str = json_str
        .strip_prefix("```json").or_else(|| json_str.strip_prefix("```"))
        .and_then(|s| s.strip_suffix("```"))
        .unwrap_or(json_str)
        .trim();

    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
        let category_str = parsed.get("category")
            .and_then(|c| c.as_str())
            .unwrap_or("unknown");

        let confidence = parsed.get("confidence")
            .and_then(|c| c.as_f64())
            .unwrap_or(0.8) as f32;

        let target_raw = parsed.get("target")
            .and_then(|t| t.as_str())
            .map(|s| s.to_lowercase());

        let category = IntentCategory::from_str(category_str);

        // Resolve target to a sister name if applicable
        let target = resolve_target(&target_raw, input);

        // Extract payload for memory_store
        let payload = if category == IntentCategory::MemoryStore {
            extract_memory_payload(input)
        } else {
            None
        };

        ClassifiedIntent { category, confidence, target, payload }
    } else {
        warn!("[hydra:intent] Failed to parse LLM JSON: {}", json_str);
        emergency_classify(input)
    }
}

/// Resolve the target string to a known sister name if possible.
fn resolve_target(target: &Option<String>, input: &str) -> Option<String> {
    // Check the LLM's target first
    if let Some(ref t) = target {
        let lower = t.to_lowercase();
        if lower == "null" || lower == "none" || lower.is_empty() {
            // Fall through
        } else {
            // Check if it matches a sister name
            for name in SISTER_NAMES {
                if lower.contains(name) {
                    return Some(name.to_string());
                }
            }
            // Return as-is if not a sister (could be a file path, URL, app name, etc.)
            return Some(t.clone());
        }
    }

    // Fallback: check input for sister names
    let lower_input = input.to_lowercase();
    for name in SISTER_NAMES {
        if lower_input.contains(name) {
            return Some(name.to_string());
        }
    }

    None
}

pub(crate) fn extract_memory_payload(input: &str) -> Option<String> {
    let lower = input.to_lowercase();
    // Find the content after common memory-store prefixes
    for prefix in &["remember ", "note that ", "save that ", "don't forget "] {
        if let Some(pos) = lower.find(prefix) {
            return Some(input[pos + prefix.len()..].to_string());
        }
    }
    if let Some(pos) = lower.find("remember that ") {
        return Some(input[pos + 14..].to_string());
    }
    Some(input.to_string())
}

// ═══════════════════════════════════════════════════════════════════
// Emergency fallback — when no LLM key is available at all
// Only handles trivially recognizable patterns. Everything else → Unknown.
// ═══════════════════════════════════════════════════════════════════

pub(crate) fn emergency_classify(input: &str) -> ClassifiedIntent {
    let lower = input.to_lowercase();
    let trimmed = lower.trim().trim_end_matches('!');

    // Greetings — single word, trivially recognizable
    if matches!(trimmed, "hi" | "hey" | "hello" | "yo" | "sup" | "hola" | "howdy"
        | "good morning" | "good evening" | "good afternoon") {
        return ClassifiedIntent { category: IntentCategory::Greeting, confidence: 0.95, target: None, payload: None };
    }
    if matches!(trimmed, "bye" | "goodbye" | "see you" | "see ya" | "later" | "goodnight" | "cya" | "peace") {
        return ClassifiedIntent { category: IntentCategory::Farewell, confidence: 0.95, target: None, payload: None };
    }
    if lower.starts_with("thank") || trimmed == "ty" || trimmed == "thx" {
        return ClassifiedIntent { category: IntentCategory::Thanks, confidence: 0.95, target: None, payload: None };
    }

    // Memory store — "remember X" is unambiguous
    if lower.starts_with("remember ") || lower.starts_with("note that ") || lower.starts_with("don't forget ") {
        return ClassifiedIntent {
            category: IntentCategory::MemoryStore,
            confidence: 0.9,
            target: None,
            payload: extract_memory_payload(input),
        };
    }

    // Memory recall — "what's my ...", "do you remember ...", "remind me ..."
    // Exclude trust/receipt/identity queries that start with "what's my"
    let is_identity_query = lower.contains("trust") || lower.contains("receipt")
        || lower.contains("audit") || lower.contains("autonomy");
    if !is_identity_query && (lower.starts_with("what's my ") || lower.starts_with("whats my ")
        || lower.starts_with("what is my ") || lower.starts_with("do you remember")
        || lower.starts_with("remind me about") || lower.starts_with("remind me of")
        || (lower.contains("my") && lower.contains("favorite"))
        || (lower.contains("my") && lower.contains("favourite")))
    {
        return ClassifiedIntent { category: IntentCategory::MemoryRecall, confidence: 0.85, target: None, payload: None };
    }

    // Sister diagnose — "check sisters", "sister status", "yo check on the sisters"
    // But NOT "policy check", "rules check" — those are queries about what the sister does
    let is_policy_query = lower.contains("policy") || lower.contains("policies")
        || lower.contains("rules") || lower.contains("what rules")
        || lower.contains("what does") || lower.contains("capabilities");
    if !is_policy_query
        && ((lower.contains("sister") && (lower.contains("check") || lower.contains("status") || lower.contains("health")))
            || lower.contains("check sisters") || lower.contains("check on the sister"))
    {
        return ClassifiedIntent { category: IntentCategory::SisterDiagnose, confidence: 0.85, target: None, payload: None };
    }

    // Self scan / self repair
    if lower.contains("scan yourself") || lower.contains("scan your") || lower.contains("omniscience") {
        return ClassifiedIntent { category: IntentCategory::SelfScan, confidence: 0.85, target: None, payload: None };
    }
    if lower.contains("fix yourself") || lower.contains("repair yourself") || lower.contains("self-repair") {
        return ClassifiedIntent { category: IntentCategory::SelfRepair, confidence: 0.85, target: None, payload: None };
    }

    // Self-implement — "implement spec ...", "build this yourself", "add this capability to yourself"
    if lower.contains("implement spec") || lower.contains("implement this spec")
        || lower.contains("build this yourself")
        || lower.contains("add this capability") || lower.contains("implement yourself")
        || (lower.contains("implement") && lower.contains("yourself"))
        || (lower.contains("implement") && (lower.contains(".md") || lower.contains(".txt")))
    {
        return ClassifiedIntent { category: IntentCategory::SelfImplement, confidence: 0.85, target: None, payload: None };
    }

    // Sister repair — "fix X sister", "make X work again"
    if (lower.contains("fix") || lower.contains("repair") || lower.contains("work again"))
        && SISTER_NAMES.iter().any(|s| lower.contains(s))
    {
        return ClassifiedIntent { category: IntentCategory::SisterRepair, confidence: 0.80, target: None, payload: None };
    }

    // News/web — "news", "top stories", "latest news", "grab me the latest"
    if lower.contains("news") || lower.contains("top stories") || lower.contains("headlines")
        || lower.contains("trending") || (lower.contains("grab") && lower.contains("latest"))
    {
        return ClassifiedIntent { category: IntentCategory::WebBrowse, confidence: 0.80, target: None, payload: None };
    }

    // File listing — "what's in here", "what's in this folder"
    if (lower.contains("what's in") || lower.contains("whats in")) && !lower.contains("what's in my mind") {
        return ClassifiedIntent { category: IntentCategory::FileOperation, confidence: 0.80, target: None, payload: None };
    }

    // App open — "open X"
    if lower.starts_with("open ") || lower.starts_with("launch ") {
        return ClassifiedIntent { category: IntentCategory::SystemControl, confidence: 0.80, target: None, payload: None };
    }

    // Planning/goals/deadlines
    if lower.contains("goal") || lower.contains("deadline") || lower.contains("what should i do")
        || lower.contains("what's the plan") || lower.contains("next step")
    {
        return ClassifiedIntent { category: IntentCategory::PlanningQuery, confidence: 0.80, target: None, payload: None };
    }

    // Receipts/identity/trust — "show my receipts", "what did you do", "trust level"
    if lower.contains("receipt") || lower.contains("trust level") || lower.contains("prove what")
        || lower.starts_with("what did you") || lower.starts_with("what have you")
        || lower.contains("last action") || lower.contains("audit trail")
    {
        return ClassifiedIntent { category: IntentCategory::Question, confidence: 0.80, target: Some("identity".into()), payload: None };
    }

    // Belief statements — "we're using X", "I prefer X", "our stack is X"
    if lower.starts_with("we're using") || lower.starts_with("we use")
        || lower.starts_with("i prefer") || lower.starts_with("our ") || lower.starts_with("my favourite")
        || lower.starts_with("my favorite") || lower.starts_with("i like")
    {
        return ClassifiedIntent { category: IntentCategory::Question, confidence: 0.80, target: Some("belief".into()), payload: None };
    }

    // Belief corrections — "actually,", "that's wrong", "i meant"
    if lower.starts_with("actually,") || lower.starts_with("actually ") || lower.contains("that's wrong")
        || lower.contains("thats wrong") || lower.starts_with("i meant") || lower.contains("switched to")
        || lower.contains("changed to") || lower.contains("instead of")
    {
        return ClassifiedIntent { category: IntentCategory::Question, confidence: 0.85, target: Some("correction".into()), payload: None };
    }

    // Everything else → Unknown → main LLM handles it
    ClassifiedIntent {
        category: IntentCategory::Unknown,
        confidence: 0.0,
        target: None,
        payload: None,
    }
}
