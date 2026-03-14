//! Pre-phase intent dispatch handlers — greeting, farewell, memory recall, settings, memory store.
//!
//! Each handler returns `true` if it handled the intent (caller should return early),
//! or `false` to fall through to the next handler / 5-phase loop.

use std::sync::Arc;
use tokio::sync::mpsc;

use crate::sisters::SistersHandle;
use hydra_native_state::utils::{safe_truncate, strip_emojis};

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
                content: strip_emojis(&greeting),
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
    let recall_model = hydra_model::pick_cheapest_model(&recall_llm_config);
    let recall_topic = extract_memory_topic(text);
    eprintln!("[hydra:recall] topic='{}' is_why={}", recall_topic, crate::sisters::memory_deep::is_why_question(text));

    // Strategy 0: Recent buffer — "what did I just..." questions
    // Check conversation history FIRST for anything that happened in this session
    if is_recent_query(text) {
        if let Some(answer) = find_in_recent_history(text, &config.history) {
            eprintln!("[hydra:recall] Found answer in recent buffer");
            return send_recall_response(&answer, tx);
        }
    }

    // Strategy 0b: Session history — "what did we talk about last?"
    if is_session_history_query(text) {
        eprintln!("[hydra:recall] Session history query detected");
        // Check if we have history from this session
        if !config.history.is_empty() {
            if let Some(answer) = summarize_recent_session(&config.history) {
                return send_recall_response(&answer, tx);
            }
        }
        let msg = "This is a fresh session — I don't have our previous conversation history to reference. \
                   But I do remember your preferences and decisions across sessions. Ask me about those!";
        return send_recall_response(msg, tx);
    }

    // Strategy 1: "Why" questions → decision query (deeper reasoning)
    // CRITICAL: If no causal data found, be honest — NEVER let LLM fabricate a reason
    if crate::sisters::memory_deep::is_why_question(text) {
        let mut found_causal = false;
        if let Some(ref sh) = sisters_handle {
            if let Some(causal_raw) = sh.memory_causal_query(text).await {
                eprintln!("[hydra:recall] Causal query returned: {}", safe_truncate(&causal_raw, 200));
                let facts = extract_memory_facts(&causal_raw);
                // STRICT filter for "why" questions — only facts containing the topic word.
                // Do NOT fall back to all facts, or the LLM will hallucinate reasons.
                let topic_lower = recall_topic.to_lowercase();
                let strict_facts: Vec<String> = if topic_lower.is_empty() {
                    facts
                } else {
                    facts.into_iter()
                        .filter(|f| f.to_lowercase().split(|c: char| !c.is_alphanumeric())
                            .any(|w| w == topic_lower))
                        .collect()
                };
                if !strict_facts.is_empty() {
                    found_causal = true;
                    let formatted = format_memory_recall_naturally(
                        text, &strict_facts, &config.user_name, &recall_llm_config, recall_model,
                    ).await;
                    return send_recall_response(&formatted, tx);
                }
            }
        }
        // No causal data for this specific topic — be honest, don't hallucinate
        eprintln!("[hydra:recall] No causal data for 'why' question — responding honestly");
        let msg = format!(
            "I know you prefer {} but I don't have the reason stored. \
             Want to tell me why so I remember next time?", recall_topic
        );
        return send_recall_response(&msg, tx);
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
        role: "hydra".into(), content: strip_emojis(content), css_class: "message hydra".into(),
    });
    let _ = tx.send(CognitiveUpdate::ResetIdle);
    true
}

/// Query sister memory — generic two-query approach. No event_type filters.
/// Query 1: Semantic search (max 20). Query 2: Recent (max 10). Merge + dedup.
async fn query_sister_memory(
    text: &str,
    _topic: &str,
    sisters_handle: &Option<SistersHandle>,
) -> Option<Vec<String>> {
    let sh = sisters_handle.as_ref()?;
    let mem = sh.memory.as_ref()?;

    let mut all_facts = Vec::new();
    // 1. Semantic search — no event_type filter, max 20
    match mem.call_tool("memory_query", serde_json::json!({
        "query": text, "max_results": 20
    })).await {
        Ok(v) => {
            let raw = crate::sisters::extract_text(&v);
            if !raw.is_empty() && !raw.contains("No memories found") {
                all_facts.extend(extract_memory_facts(&raw));
            }
        }
        Err(e) => eprintln!("[hydra:recall] semantic query FAILED: {}", e),
    }
    // 2. Recent memories (max 10)
    match mem.call_tool("memory_query", serde_json::json!({
        "query": text, "max_results": 10, "sort_by": "most_recent"
    })).await {
        Ok(v) => {
            let raw = crate::sisters::extract_text(&v);
            if !raw.is_empty() && !raw.contains("No memories found") {
                for f in extract_memory_facts(&raw) {
                    if !all_facts.iter().any(|existing| existing == &f) { all_facts.push(f); }
                }
            }
        }
        Err(e) => eprintln!("[hydra:recall] recent query FAILED: {}", e),
    }
    if !all_facts.is_empty() { return Some(all_facts); }
    // 3. Beliefs fallback
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

/// Detect questions about things that happened in the current session.
fn is_recent_query(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("just test") || lower.contains("just do")
        || lower.contains("just run") || lower.contains("just happen")
        || lower.contains("i just") || lower.contains("we just")
        || lower.contains("did i just") || lower.contains("did we just")
        || (lower.contains("what did") && (lower.contains("test") || lower.contains("run") || lower.contains("do")))
        // "why did the tests pass/fail?" — about recent test results, not stored decisions
        || (lower.contains("why did") && (lower.contains("test") || lower.contains("pass") || lower.contains("fail")))
        || (lower.contains("the tests") && (lower.contains("why") || lower.contains("how")))
}

/// Search recent conversation history for answers to "what did I just..." questions.
fn find_in_recent_history(text: &str, history: &[(String, String)]) -> Option<String> {
    let lower = text.to_lowercase();
    // Look through recent assistant messages (last 15 turns) for relevant content
    for (_role, content) in history.iter().rev().take(15) {
        let content_lower = content.to_lowercase();
        // Match test reports
        if (lower.contains("test") || lower.contains("run"))
            && (content_lower.contains("passed") || content_lower.contains("success")
                || content_lower.contains("failed") || content_lower.contains("report"))
        {
            return Some(content.clone());
        }
        // Match swarm results
        if lower.contains("swarm") || lower.contains("agent") {
            if content_lower.contains("spawned") || content_lower.contains("terminated")
                || content_lower.contains("swarm") {
                return Some(content.clone());
            }
        }
        // Match improvement results
        if lower.contains("improve") {
            if content_lower.contains("improvement") || content_lower.contains("analyzing") {
                return Some(content.clone());
            }
        }
    }
    None
}

/// Summarize recent session for "what did we talk about" questions.
fn summarize_recent_session(history: &[(String, String)]) -> Option<String> {
    if history.is_empty() { return None; }
    let mut topics = Vec::new();
    for (role, content) in history.iter().rev().take(10) {
        if role == "user" {
            let brief = safe_truncate(content, 60);
            if !brief.is_empty() { topics.push(brief.to_string()); }
        }
    }
    if topics.is_empty() { return None; }
    topics.reverse();
    Some(format!("In this session, you asked about: {}", topics.join(", ")))
}

/// Detect if the user is asking about session history vs stored facts.
/// "what did we talk about last?" = session history (needs session events)
/// "what's my favorite color?" = stored facts (needs memory query)
fn is_session_history_query(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("talk about last") || lower.contains("talked about last")
        || lower.contains("last session") || lower.contains("last conversation")
        || lower.contains("last time") || lower.contains("previous session")
        || lower.contains("what did we discuss") || lower.contains("what were we doing")
}

// handle_settings and handle_memory_store live in dispatch_intents_store.rs
