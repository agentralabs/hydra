//! Memory store and settings handlers — split from dispatch_intents.rs for 400-line limit.

use tokio::sync::mpsc;

use crate::sisters::SistersHandle;
use hydra_native_state::utils::{safe_truncate, strip_emojis};

use super::super::loop_runner::CognitiveUpdate;
use super::super::intent_router::{IntentCategory, ClassifiedIntent};
use super::memory::normalize_memory_for_storage;

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

    let fact = normalize_memory_for_storage(&fact);
    eprintln!("[hydra:memory] Saving directly: {}", safe_truncate(&fact, 80));

    // RULE 1: Store everything as episode. No classification. Let Memory sister sort it out.
    let mut saved = false;
    if let Some(ref sh) = sisters_handle {
        if let Some(ref mem) = sh.memory {
            match mem.call_tool("memory_add", serde_json::json!({
                "event_type": "episode",
                "content": fact,
                "confidence": 0.8,
            })).await {
                Ok(v) => {
                    saved = true;
                    eprintln!("[hydra:memory] memory_add OK: {}",
                        serde_json::to_string(&v).unwrap_or_default());
                }
                Err(e) => eprintln!("[hydra:memory] memory_add FAILED: {}", e),
            }
        }
        if let Some(ref cog) = sh.cognition {
            if let Err(e) = cog.call_tool("cognition_belief_add", serde_json::json!({
                "subject": "user_statement", "content": fact,
                "confidence": 1.0, "source": "explicit_user_statement"
            })).await { eprintln!("[hydra:memory] cognition_belief_add FAILED: {}", e); }
        }
    }

    let msg = if saved {
        format!("Got it! I'll remember that: **{}**", fact)
    } else {
        format!("I'll remember: **{}** (note: memory sister may be offline)", fact)
    };
    let _ = tx.send(CognitiveUpdate::Message {
        role: "hydra".into(), content: strip_emojis(&msg), css_class: "message hydra".into(),
    });
    let _ = tx.send(CognitiveUpdate::ResetIdle);
    true
}
