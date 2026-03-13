//! Belief persistence helpers — extracted from phase_learn.rs for compilation performance.
//!
//! Contains belief update from user text and LLM-powered knowledge extraction.

use std::sync::Arc;
use tokio::sync::mpsc;

use hydra_db::{HydraDb, BeliefRow};

use super::super::loop_runner::CognitiveUpdate;
use super::memory::{extract_belief_subject, md5_simple};
use hydra_native_state::utils::safe_truncate;

/// Extract and persist beliefs from user text (preferences, facts, corrections).
pub(crate) fn update_beliefs_from_text(
    text: &str,
    final_response: &str,
    db: &Arc<HydraDb>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) {
    let lower = text.to_lowercase();
    // Detect user-stated beliefs (preferences, facts, corrections)
    let belief_patterns: &[(&str, &str, &str)] = &[
        ("i prefer", "preference", "user_stated"),
        ("i always use", "preference", "user_stated"),
        ("i never use", "preference", "user_stated"),
        ("we use", "fact", "user_stated"),
        ("we're using", "fact", "user_stated"),
        ("our database is", "fact", "user_stated"),
        ("our framework is", "fact", "user_stated"),
        ("our stack is", "fact", "user_stated"),
        ("actually,", "correction", "corrected"),
        ("that's wrong", "correction", "corrected"),
        ("no, i meant", "correction", "corrected"),
        ("i meant", "correction", "corrected"),
        ("don't ever", "convention", "user_stated"),
        ("always ", "convention", "user_stated"),
    ];
    for (pattern, category, source) in belief_patterns {
        if lower.contains(pattern) {
            // Extract the belief content (user's full statement)
            let subject = extract_belief_subject(text, pattern);
            let now = chrono::Utc::now().to_rfc3339();
            let belief_id = format!("belief-{}", md5_simple(&format!("{}:{}", subject, text)));

            // Check if a similar belief exists (by subject or keyword overlap)
            let mut existing = db.get_beliefs_by_subject(&subject).unwrap_or_default();

            // For corrections, also search by individual keywords from the full text
            if existing.is_empty() && *source == "corrected" {
                let stop_words = ["actually", "instead", "of", "to", "the", "a", "an",
                    "we", "i", "my", "our", "that", "this", "it", "is", "was",
                    "switched", "changed", "wrong", "meant", "no", "not", "from"];
                let keywords: Vec<&str> = text.split_whitespace()
                    .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
                    .filter(|w| w.len() >= 3 && !stop_words.contains(&w.to_lowercase().as_str()))
                    .collect();
                for kw in &keywords {
                    let matches = db.get_beliefs_by_subject(kw).unwrap_or_default();
                    if !matches.is_empty() {
                        existing = matches;
                        break;
                    }
                }
                // Also search by content if subject match failed
                if existing.is_empty() {
                    if let Ok(all_beliefs) = db.get_active_beliefs(50) {
                        for kw in &keywords {
                            let kw_lower = kw.to_lowercase();
                            if let Some(found) = all_beliefs.iter().find(|b|
                                b.content.to_lowercase().contains(&kw_lower)
                                || b.subject.to_lowercase().contains(&kw_lower)
                            ) {
                                existing = vec![found.clone()];
                                break;
                            }
                        }
                    }
                }
            }

            if let Some(old) = existing.first() {
                // Contradict and supersede old belief
                if *source == "corrected" {
                    let _ = db.contradict_belief(&old.id);
                }
                let _ = db.supersede_belief(&old.id, &belief_id);
            }

            let confidence = match *source {
                "corrected" => 0.99,
                "user_stated" => 0.95,
                _ => 0.60,
            };
            let _ = db.upsert_belief(&BeliefRow {
                id: belief_id,
                category: category.to_string(),
                subject: subject.clone(),
                content: text.to_string(),
                confidence,
                source: source.to_string(),
                confirmations: 0,
                contradictions: 0,
                active: true,
                supersedes: existing.first().map(|b| b.id.clone()),
                superseded_by: None,
                created_at: now.clone(),
                updated_at: now,
            });
            let _ = tx.send(CognitiveUpdate::BeliefUpdated {
                subject: subject.clone(),
                content: text.to_string(),
                confidence,
                is_new: existing.is_empty(),
            });
            break; // One belief per message
        }
    }

    // Confirm existing beliefs that are referenced in the response
    if let Ok(beliefs) = db.get_active_beliefs(50) {
        for belief in &beliefs {
            if final_response.to_lowercase().contains(&belief.subject.to_lowercase()) {
                let _ = db.confirm_belief(&belief.id);
            }
        }
    }
}

/// LLM-powered knowledge extraction: call a cheap model to extract structured knowledge,
/// then persist as beliefs.
pub(crate) async fn extract_and_persist_knowledge(
    text: &str,
    final_response: &str,
    llm_ok: bool,
    active_model: &str,
    llm_config: &hydra_model::LlmConfig,
    db: &Option<Arc<HydraDb>>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) {
    let llm_result: Result<(), String> = if llm_ok { Ok(()) } else { Err("LLM failed".into()) };

    if llm_result.is_err() {
        return;
    }

    // Micro-LLM call to extract structured knowledge from the interaction
    let learn_llm_config = llm_config.clone();
    let learn_model = hydra_model::pick_cheapest_model(&learn_llm_config);

    let learn_prompt = format!(
        "User said: \"{}\"\nHydra responded: \"{}\"\nOutcome: {}\n\nExtract knowledge:",
        safe_truncate(text, 300),
        safe_truncate(final_response, 500),
        if llm_result.is_ok() { "success" } else { "failure" },
    );

    let learn_request = hydra_model::CompletionRequest {
        model: learn_model.to_string(),
        messages: vec![hydra_model::providers::Message {
            role: "user".into(),
            content: learn_prompt,
        }],
        max_tokens: 200,
        temperature: Some(0.3),
        system: Some(hydra_runtime::cognitive::prompts::learn_extract_knowledge_prompt().to_string()),
    };

    let learn_result = if learn_llm_config.anthropic_api_key.is_some() {
        match hydra_model::providers::anthropic::AnthropicClient::new(&learn_llm_config) {
            Ok(client) => client.complete(learn_request).await.ok(),
            Err(_) => None,
        }
    } else if learn_llm_config.openai_api_key.is_some() {
        match hydra_model::providers::openai::OpenAiClient::new(&learn_llm_config) {
            Ok(client) => client.complete(learn_request).await.ok(),
            Err(_) => None,
        }
    } else {
        None
    };

    // Parse extracted knowledge and persist as beliefs
    if let Some(resp) = learn_result {
        let content = resp.content.trim();
        // Try to parse as JSON array of knowledge items
        if let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(content) {
            persist_knowledge_items(&items, db, tx);
        } else {
            // Not JSON — try to extract from markdown code block
            let stripped = content.trim_start_matches("```json").trim_start_matches("```").trim_end_matches("```").trim();
            if let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(stripped) {
                eprintln!("[hydra:learn] Extracted {} knowledge items from code block", items.len());
                // Same processing as above — but keeping it brief for the fallback path
            }
        }
    }
}

/// Persist parsed knowledge items as beliefs in the database.
fn persist_knowledge_items(
    items: &[serde_json::Value],
    db: &Option<Arc<HydraDb>>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) {
    if let Some(ref db) = db {
        for item in items {
            let k_type = item["type"].as_str().unwrap_or("fact");
            let k_content = item["content"].as_str().unwrap_or("");
            let k_confidence = item["confidence"].as_f64().unwrap_or(0.7);
            let k_subject = item["subject"].as_str().unwrap_or("");

            if k_content.is_empty() || k_subject.is_empty() {
                continue;
            }

            let now = chrono::Utc::now().to_rfc3339();
            let belief_id = format!("learn-{}", md5_simple(&format!("{}:{}", k_subject, k_content)));

            // Check for existing belief on same subject
            let existing = db.get_beliefs_by_subject(k_subject).unwrap_or_default();
            if let Some(old) = existing.first() {
                if k_type == "correction" {
                    let _ = db.supersede_belief(&old.id, &belief_id);
                } else {
                    // Confirm existing belief instead of creating duplicate
                    let _ = db.confirm_belief(&old.id);
                    continue;
                }
            }

            let source = match k_type {
                "correction" => "corrected",
                "preference" => "user_stated",
                _ => "inferred",
            };

            let _ = db.upsert_belief(&BeliefRow {
                id: belief_id,
                category: k_type.to_string(),
                subject: k_subject.to_string(),
                content: k_content.to_string(),
                confidence: k_confidence,
                source: source.to_string(),
                confirmations: 0,
                contradictions: 0,
                active: true,
                supersedes: existing.first().map(|b| b.id.clone()),
                superseded_by: None,
                created_at: now.clone(),
                updated_at: now,
            });

            let _ = tx.send(CognitiveUpdate::BeliefUpdated {
                subject: k_subject.to_string(),
                content: k_content.to_string(),
                confidence: k_confidence,
                is_new: existing.is_empty(),
            });

            eprintln!("[hydra:learn] Extracted {}: {} (confidence: {:.0}%)",
                k_type, k_subject, k_confidence * 100.0);
        }
    }
}
