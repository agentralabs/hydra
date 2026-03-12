//! Pre-phase intent dispatch handlers — greeting, farewell, memory recall, settings, memory store.
//!
//! Each handler returns `true` if it handled the intent (caller should return early),
//! or `false` to fall through to the next handler / 5-phase loop.

use std::sync::Arc;
use tokio::sync::mpsc;

use crate::sisters::SistersHandle;
use hydra_native_state::utils::safe_truncate;

use super::super::loop_runner::{CognitiveLoopConfig, CognitiveUpdate};
use super::super::intent_router::{IntentCategory, ClassifiedIntent};
use super::memory::{extract_memory_facts, extract_memory_topic, filter_facts_by_relevance, format_memory_recall_naturally, normalize_memory_for_storage};

/// Handle greeting, farewell, thanks — instant response, no LLM needed.
pub(crate) fn handle_greeting_farewell_thanks(
    intent: &ClassifiedIntent,
    config: &CognitiveLoopConfig,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> bool {
    if intent.confidence < 0.6 {
        return false;
    }
    match intent.category {
        IntentCategory::Greeting => {
            let _ = tx.send(CognitiveUpdate::Message {
                role: "hydra".into(),
                content: format!("Hey{}! What can I do for you?",
                    if config.user_name.is_empty() { String::new() }
                    else { format!(", {}", config.user_name) }),
                css_class: "message hydra".into(),
            });
            let _ = tx.send(CognitiveUpdate::ResetIdle);
            true
        }
        IntentCategory::Farewell => {
            let _ = tx.send(CognitiveUpdate::Message {
                role: "hydra".into(),
                content: "See you later! I'll be here when you need me.".into(),
                css_class: "message hydra".into(),
            });
            let _ = tx.send(CognitiveUpdate::ResetIdle);
            true
        }
        IntentCategory::Thanks => {
            let _ = tx.send(CognitiveUpdate::Message {
                role: "hydra".into(),
                content: "You're welcome! Anything else?".into(),
                css_class: "message hydra".into(),
            });
            let _ = tx.send(CognitiveUpdate::ResetIdle);
            true
        }
        _ => false,
    }
}

/// Handle memory recall — natural conversational response.
pub(crate) async fn handle_memory_recall(
    text: &str,
    intent: &ClassifiedIntent,
    config: &CognitiveLoopConfig,
    sisters_handle: &Option<SistersHandle>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> bool {
    if intent.category != IntentCategory::MemoryRecall || intent.confidence < 0.6 {
        return false;
    }

    let _ = tx.send(CognitiveUpdate::Phase("Recall".into()));
    let _ = tx.send(CognitiveUpdate::IconState("working".into()));

    let sh = match sisters_handle.as_ref() {
        Some(sh) => sh,
        None => return false,
    };
    let mem = match sh.memory.as_ref() {
        Some(mem) => mem,
        None => return false,
    };

    // Build LLM config for the micro-formatting call (sanitized keys)
    let recall_llm_config = hydra_model::LlmConfig::from_env_with_overlay(
        &config.anthropic_key,
        &config.openai_key,
        config.anthropic_oauth_token.as_deref(),
    );
    let recall_model = if recall_llm_config.anthropic_api_key.is_some() {
        "claude-haiku-4-5-20251001"
    } else {
        &config.model
    };

    // Extract topic for targeted query
    let recall_topic = extract_memory_topic(text);
    eprintln!("[hydra:recall] Extracted topic: '{}'", recall_topic);

    // Query facts first (high-signal), then general
    let facts_result = mem.call_tool("memory_query", serde_json::json!({
        "query": if recall_topic.is_empty() { text.to_string() } else { recall_topic.clone() },
        "event_types": ["fact", "correction", "decision"],
        "max_results": 5,
        "sort_by": "highest_confidence"
    })).await;

    let fact_text = facts_result.ok()
        .map(|v| crate::sisters::extract_text(&v))
        .filter(|t| !t.is_empty() && !t.contains("No memories found"));

    if let Some(ref raw_facts) = fact_text {
        eprintln!("[hydra:recall] Found facts: {}", safe_truncate(raw_facts, 200));
        let facts = extract_memory_facts(raw_facts);
        let facts = filter_facts_by_relevance(&facts, &recall_topic);
        if !facts.is_empty() {
            let formatted = format_memory_recall_naturally(
                text, &facts, &config.user_name, &recall_llm_config, recall_model
            ).await;
            let _ = tx.send(CognitiveUpdate::Message {
                role: "hydra".into(),
                content: formatted,
                css_class: "message hydra".into(),
            });
            let _ = tx.send(CognitiveUpdate::ResetIdle);
            return true;
        }
    }

    // No facts found — try general memory
    let general_result = mem.call_tool("memory_query", serde_json::json!({
        "query": text,
        "max_results": 5
    })).await;

    let general_text = general_result.ok()
        .map(|v| crate::sisters::extract_text(&v))
        .filter(|t| !t.is_empty() && !t.contains("No memories found"));

    if let Some(ref raw_general) = general_text {
        eprintln!("[hydra:recall] Found general memory: {}", safe_truncate(raw_general, 200));
        let facts = extract_memory_facts(raw_general);
        if !facts.is_empty() {
            let formatted = format_memory_recall_naturally(
                text, &facts, &config.user_name, &recall_llm_config, recall_model
            ).await;
            let _ = tx.send(CognitiveUpdate::Message {
                role: "hydra".into(),
                content: formatted,
                css_class: "message hydra".into(),
            });
            let _ = tx.send(CognitiveUpdate::ResetIdle);
            return true;
        }
    }

    // Also check beliefs
    if let Some(ref cog) = sh.cognition {
        let beliefs_result = cog.call_tool("cognition_belief_query", serde_json::json!({"query": text})).await;
        let belief_text = beliefs_result.ok()
            .map(|v| crate::sisters::extract_text(&v))
            .filter(|t| !t.is_empty());
        if let Some(ref raw_beliefs) = belief_text {
            let facts = extract_memory_facts(raw_beliefs);
            if !facts.is_empty() {
                let formatted = format_memory_recall_naturally(
                    text, &facts, &config.user_name, &recall_llm_config, recall_model
                ).await;
                let _ = tx.send(CognitiveUpdate::Message {
                    role: "hydra".into(),
                    content: formatted,
                    css_class: "message hydra".into(),
                });
                let _ = tx.send(CognitiveUpdate::ResetIdle);
                return true;
            }
        }
    }

    // Nothing found — let it fall through to LLM
    eprintln!("[hydra:recall] No memories found, falling through to LLM");
    false
}

/// Handle natural language settings detection.
pub(crate) fn handle_settings(
    text: &str,
    intent: &ClassifiedIntent,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> bool {
    if intent.category != IntentCategory::Settings {
        return false;
    }
    let mut settings = hydra_native_state::state::settings::SettingsStore::default();
    if let Some(confirmation) = settings.apply_natural_language(text) {
        let _ = tx.send(CognitiveUpdate::SettingsApplied { confirmation: confirmation.clone() });
        let _ = tx.send(CognitiveUpdate::Message {
            role: "hydra".into(),
            content: confirmation,
            css_class: "message hydra settings-applied".into(),
        });
        let _ = tx.send(CognitiveUpdate::ResetIdle);
        return true;
    }
    false
}

/// Handle direct memory store — "remember X" / "note that X".
pub(crate) async fn handle_memory_store(
    text: &str,
    intent: &ClassifiedIntent,
    sisters_handle: &Option<SistersHandle>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> bool {
    let memory_payload = if intent.category == IntentCategory::MemoryStore {
        intent.payload.clone().or_else(|| Some(text.to_string()))
    } else {
        return false;
    };
    let fact = match memory_payload {
        Some(f) => f,
        None => return false,
    };

    let _ = tx.send(CognitiveUpdate::Phase("Learn (direct)".into()));
    let _ = tx.send(CognitiveUpdate::IconState("working".into()));

    // Normalize pronouns before storage
    let fact = normalize_memory_for_storage(&fact);
    eprintln!("[hydra:memory] Saving directly: {}", safe_truncate(&fact, 80));

    let mut saved = false;
    if let Some(ref sh) = sisters_handle {
        if let Some(ref mem) = sh.memory {
            let payload = serde_json::json!({
                "event_type": "fact",
                "content": format!("User preference: {}", fact),
                "confidence": 0.95
            });
            match mem.call_tool("memory_add", payload).await {
                Ok(v) => {
                    saved = true;
                    eprintln!("[hydra:memory] memory_add OK: {}", serde_json::to_string(&v).unwrap_or_default());
                }
                Err(e) => { eprintln!("[hydra:memory] memory_add FAILED: {}", e); }
            }
        }
        // Also store as a belief via cognition
        if let Some(ref cog) = sh.cognition {
            let _ = cog.call_tool("cognition_belief_add", serde_json::json!({
                "subject": "user_preference",
                "content": fact,
                "confidence": 1.0,
                "source": "explicit_user_statement"
            })).await;
        }
    }

    let msg = if saved {
        format!("Got it! I'll remember that: **{}**", fact)
    } else {
        format!("I'll remember: **{}** (note: memory sister may be offline)", fact)
    };
    let _ = tx.send(CognitiveUpdate::Message {
        role: "hydra".into(),
        content: msg,
        css_class: "message hydra".into(),
    });
    let _ = tx.send(CognitiveUpdate::ResetIdle);
    true
}
