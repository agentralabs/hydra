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

/// Handle greeting, farewell, thanks — varied, personal responses.
///
/// Instead of static "What can I do for you?" every time, use varied
/// greetings that feel natural and reference what Hydra knows about the user.
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
            let name = if config.user_name.is_empty() {
                String::new()
            } else {
                format!(" {}", config.user_name)
            };
            let greeting = pick_greeting(&name);
            let _ = tx.send(CognitiveUpdate::Message {
                role: "hydra".into(),
                content: greeting,
                css_class: "message hydra".into(),
            });
            let _ = tx.send(CognitiveUpdate::ResetIdle);
            true
        }
        IntentCategory::Farewell => {
            let farewells = [
                "See you later! I'll keep an eye on things.",
                "Take care! I'll be here whenever you're back.",
                "Later! Your workspace will be right where you left it.",
                "Catch you later. I'll keep dreaming in the background.",
            ];
            let idx = tick_index(farewells.len());
            let _ = tx.send(CognitiveUpdate::Message {
                role: "hydra".into(),
                content: farewells[idx].into(),
                css_class: "message hydra".into(),
            });
            let _ = tx.send(CognitiveUpdate::ResetIdle);
            true
        }
        IntentCategory::Thanks => {
            let thanks = [
                "Anytime! What's next?",
                "Happy to help. Need anything else?",
                "You got it. Ready when you are.",
                "No problem! Let me know if something else comes up.",
            ];
            let idx = tick_index(thanks.len());
            let _ = tx.send(CognitiveUpdate::Message {
                role: "hydra".into(),
                content: thanks[idx].into(),
                css_class: "message hydra".into(),
            });
            let _ = tx.send(CognitiveUpdate::ResetIdle);
            true
        }
        _ => false,
    }
}

/// Pick a varied greeting — never the same one twice in a row.
fn pick_greeting(name: &str) -> String {
    let greetings: &[&str] = &[
        "Hey{}! Good to see you.",
        "What's up{}! Ready to build something?",
        "Hey{}! How's it going?",
        "Welcome back{}! What are we working on?",
        "Hey{}! I'm here — what's on your mind?",
        "Yo{}! What are we tackling today?",
    ];
    let idx = tick_index(greetings.len());
    greetings[idx].replacen("{}", name, 1)
}

/// Simple rotating index based on elapsed time — avoids repeating.
fn tick_index(len: usize) -> usize {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    (secs as usize) % len
}

/// Handle memory recall — natural conversational response.
///
/// Three strategies: (1) causal chain for "why" questions, (2) sister memory query,
/// (3) conversation history fallback when sisters are offline.
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

    let recall_llm_config = hydra_model::LlmConfig::from_env_with_overlay(
        &config.anthropic_key, &config.openai_key, config.anthropic_oauth_token.as_deref(),
    );
    let recall_model = if recall_llm_config.anthropic_api_key.is_some() {
        "claude-haiku-4-5-20251001"
    } else { &config.model };
    let recall_topic = extract_memory_topic(text);
    eprintln!("[hydra:recall] topic='{}' is_why={}", recall_topic, crate::sisters::memory_deep::is_why_question(text));

    // Strategy 1: "Why" questions → decision query (deeper reasoning)
    if crate::sisters::memory_deep::is_why_question(text) {
        if let Some(ref sh) = sisters_handle {
            if let Some(causal_raw) = sh.memory_causal_query(text).await {
                eprintln!("[hydra:recall] Causal query returned: {}", safe_truncate(&causal_raw, 200));
                let facts = extract_memory_facts(&causal_raw);
                let facts = filter_facts_by_relevance(&facts, &recall_topic);
                if !facts.is_empty() {
                    let formatted = format_memory_recall_naturally(
                        text, &facts, &config.user_name, &recall_llm_config, recall_model,
                    ).await;
                    return send_recall_response(&formatted, tx);
                }
            }
        }
    }

    // Strategy 2: Sister memory query (facts, general, beliefs)
    if let Some(facts) = query_sister_memory(text, &recall_topic, sisters_handle).await {
        let facts = filter_facts_by_relevance(&facts, &recall_topic);
        if !facts.is_empty() {
            let formatted = format_memory_recall_naturally(
                text, &facts, &config.user_name, &recall_llm_config, recall_model,
            ).await;
            return send_recall_response(&formatted, tx);
        }
    }

    // Strategy 3: Conversation history fallback (works without sisters)
    let history_facts = extract_facts_from_history(&config.history);
    if !history_facts.is_empty() {
        eprintln!("[hydra:recall] Using {} facts from conversation history", history_facts.len());
        let relevant = filter_facts_by_relevance(&history_facts, &recall_topic);
        let formatted = format_memory_recall_naturally(
            text, &relevant, &config.user_name, &recall_llm_config, recall_model,
        ).await;
        return send_recall_response(&formatted, tx);
    }

    eprintln!("[hydra:recall] No memories found, falling through to LLM");
    false
}

/// Send a recall response and reset idle.
fn send_recall_response(content: &str, tx: &mpsc::UnboundedSender<CognitiveUpdate>) -> bool {
    let _ = tx.send(CognitiveUpdate::Message {
        role: "hydra".into(), content: content.to_string(), css_class: "message hydra".into(),
    });
    let _ = tx.send(CognitiveUpdate::ResetIdle);
    true
}

/// Query sister memory — facts, general, beliefs (returns None if sisters offline).
async fn query_sister_memory(
    text: &str,
    topic: &str,
    sisters_handle: &Option<SistersHandle>,
) -> Option<Vec<String>> {
    let sh = sisters_handle.as_ref()?;
    let mem = sh.memory.as_ref()?;
    let query = if topic.is_empty() { text.to_string() } else { topic.to_string() };

    // Try high-signal facts first
    if let Ok(v) = mem.call_tool("memory_query", serde_json::json!({
        "query": query, "event_types": ["fact", "correction", "decision"],
        "max_results": 5, "sort_by": "highest_confidence"
    })).await {
        let raw = crate::sisters::extract_text(&v);
        if !raw.is_empty() && !raw.contains("No memories found") {
            let facts = extract_memory_facts(&raw);
            if !facts.is_empty() { return Some(facts); }
        }
    }
    // Try general memory
    if let Ok(v) = mem.call_tool("memory_query", serde_json::json!({"query": text, "max_results": 5})).await {
        let raw = crate::sisters::extract_text(&v);
        if !raw.is_empty() && !raw.contains("No memories found") {
            let facts = extract_memory_facts(&raw);
            if !facts.is_empty() { return Some(facts); }
        }
    }
    // Try beliefs
    if let Some(ref cog) = sh.cognition {
        if let Ok(v) = cog.call_tool("cognition_belief_query", serde_json::json!({"query": text})).await {
            let raw = crate::sisters::extract_text(&v);
            if !raw.is_empty() {
                let facts = extract_memory_facts(&raw);
                if !facts.is_empty() { return Some(facts); }
            }
        }
    }
    None
}

/// Extract useful facts from conversation history when sisters are offline.
fn extract_facts_from_history(history: &[(String, String)]) -> Vec<String> {
    let mut facts = Vec::new();
    for (role, content) in history.iter().rev().take(20) {
        if role == "user" {
            // Extract decision statements
            let lower = content.to_lowercase();
            if lower.contains("decided") || lower.contains("chose") || lower.contains("prefer")
                || lower.contains("my favorite") || lower.contains("i use ")
                || lower.contains("i work") || lower.contains("i'm a ")
            {
                facts.push(normalize_memory_for_storage(content));
            }
        }
    }
    facts.truncate(5);
    facts
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

    // Detect decision vs. preference for proper event typing
    let lower_fact = fact.to_lowercase();
    let is_decision = lower_fact.contains("decided") || lower_fact.contains("chose")
        || lower_fact.contains("going with") || lower_fact.contains("switching to")
        || text.to_lowercase().contains("decided") || text.to_lowercase().contains("chose");
    let (event_type, prefix) = if is_decision {
        ("decision", "User decision: ")
    } else {
        ("fact", "User preference: ")
    };

    let mut saved = false;
    if let Some(ref sh) = sisters_handle {
        if let Some(ref mem) = sh.memory {
            let payload = serde_json::json!({
                "event_type": event_type,
                "content": format!("{}{}", prefix, fact),
                "confidence": 0.95
            });
            match mem.call_tool("memory_add", payload).await {
                Ok(v) => {
                    saved = true;
                    eprintln!("[hydra:memory] memory_add OK ({}): {}", event_type, serde_json::to_string(&v).unwrap_or_default());
                }
                Err(e) => { eprintln!("[hydra:memory] memory_add FAILED: {}", e); }
            }
        }
        // Also store as a belief via cognition
        if let Some(ref cog) = sh.cognition {
            let subject = if is_decision { "user_decision" } else { "user_preference" };
            let _ = cog.call_tool("cognition_belief_add", serde_json::json!({
                "subject": subject,
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
