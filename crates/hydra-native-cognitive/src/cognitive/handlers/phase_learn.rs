//! LEARN + DELIVER phase — extracted from loop_runner.rs for compilation performance.
//!
//! Stores learnings, revises beliefs, crystallizes patterns, delivers final response.
//! Belief persistence logic lives in `phase_learn_beliefs`.

use std::sync::Arc;
use tokio::sync::mpsc;

use crate::cognitive::inventions::InventionEngine;
use crate::sisters::SistersHandle;
use hydra_native_state::state::hydra::{CognitivePhase, PhaseState, PhaseStatus};
use hydra_native_state::utils::safe_truncate;

use super::super::loop_runner::CognitiveUpdate;
use super::llm_helpers::extract_primary_topic;
use super::phase_learn_beliefs;

/// Run the LEARN + DELIVER phase: store learnings, deliver response, reset state.
pub(crate) async fn run_learn(
    text: &str,
    config: &super::super::loop_runner::CognitiveLoopConfig,
    final_response: &str,
    is_simple: bool,
    is_complex: bool,
    llm_ok: bool,
    llm_config: &hydra_model::LlmConfig,
    active_model: &str,
    intent: &super::super::intent_router::ClassifiedIntent,
    sisters_handle: &Option<SistersHandle>,
    inventions: &Option<Arc<InventionEngine>>,
    db: &Option<Arc<hydra_db::HydraDb>>,
    federation: &Option<Arc<hydra_native_state::federation::FederationManager>>,
    all_exec_results: &[(String, String, bool)],
    is_action_request: bool,
    perceive_ms: u64,
    think_ms: u64,
    decide_ms: u64,
    act_ms: u64,
    input_tokens: u64,
    output_tokens: u64,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) {
    let _ = tx.send(CognitiveUpdate::Phase("Learn".into()));
    let learn_start = std::time::Instant::now();

    // Alias for compatibility
    let llm_result: Result<(), String> = if llm_ok { Ok(()) } else { Err("LLM failed".into()) };

    let user_text = config.history.last().map(|(_, c)| c.clone()).unwrap_or_default();
    if let Some(ref sh) = sisters_handle {
        if llm_result.is_ok() {
            sh.learn(&user_text, final_response).await;

            // Planning: update goal progress from this interaction
            sh.learn_planning(&user_text, safe_truncate(final_response, 200)).await;

            // Comm: share significant learnings with peers
            sh.learn_comm_share(&format!("Completed: {}", safe_truncate(&user_text, 100))).await;
        }
    }

    // ── BELIEF UPDATE: Extract and persist beliefs from this interaction ──
    if let Some(ref db) = db {
        phase_learn_beliefs::update_beliefs_from_text(text, final_response, db, tx);
    }

    // ── FEDERATION SYNC: Sync learnings with federated peers ──
    if let Some(ref fed) = federation {
        if fed.is_enabled() && llm_result.is_ok() {
            // Sync the interaction as a state entry
            let entry = hydra_federation::sync::SyncEntry {
                key: format!("interaction:{}", config.task_id),
                value: serde_json::json!({
                    "input": safe_truncate(text, 200),
                    "response_summary": safe_truncate(final_response, 200),
                }),
                version: fed.sync.version() + 1,
                timestamp: chrono::Utc::now().to_rfc3339(),
                origin_peer: "self".to_string(),
            };
            fed.sync.local_put(&entry.key, entry.value, "self");

            // Update DB sync version
            if let Some(ref db) = db {
                for peer in fed.registry.list() {
                    let _ = db.update_federation_sync(&peer.id, fed.sync.version() as i64);
                }
            }
        }
    }

    // ── Phase 1: Deep LEARN with LLM-powered knowledge extraction ──
    phase_learn_beliefs::extract_and_persist_knowledge(
        text, final_response, llm_ok, active_model, llm_config, db, tx,
    ).await;

    // Sprint 4: Metacognition — reflect on this interaction
    if let Some(ref inv) = inventions {
        let success = llm_result.is_ok();
        let confidence = if success { 0.8 } else { 0.3 };
        let insights = inv.reflect(text, confidence, success);
        for insight in insights {
            let _ = tx.send(CognitiveUpdate::ReflectionInsight { insight });
        }

        // Sprint 4: Crystallization — record action pattern, auto-create skill if repeated
        let actions = vec![
            format!("perceive:{}", text),
            format!("think:{}", active_model),
            format!("act:{}", if is_complex { "execute_plan" } else { "respond" }),
        ];
        let learn_so_far = learn_start.elapsed().as_millis() as u64;
        if let Some(skill_name) = inv.record_action(text, &actions, success, perceive_ms + think_ms + act_ms + learn_so_far) {
            let _ = tx.send(CognitiveUpdate::SkillCrystallized {
                name: skill_name,
                actions_count: actions.len(),
            });
        }

        // Sprint 4: Store interaction in temporal memory (with outcome for future reference)
        let outcome_summary: String = if success {
            safe_truncate(final_response, 100).to_string()
        } else {
            "FAILED".to_string()
        };
        let temporal_content = format!("{} → {}", text, outcome_summary);

        // Phase 2, L1: Weighted Temporal Storage
        let temporal_importance = {
            let mut importance: f64 = if success { 0.5 } else { 0.3 };
            let lower = text.to_lowercase();

            // Corrections are highly important (user correcting Hydra)
            if lower.contains("actually") || lower.contains("no, ") || lower.contains("that's wrong") {
                importance = 0.95;
            }
            // Code generation / complex tasks are important
            else if is_complex {
                importance += 0.2;
            }
            // Commands were executed — important to remember
            if !all_exec_results.is_empty() {
                importance += 0.1;
            }
            // Failed interactions are important to remember for T3
            if !success {
                importance = 0.4;
            }
            // Greetings/small talk are low importance
            if matches!(intent.category, super::super::intent_router::IntentCategory::Greeting | super::super::intent_router::IntentCategory::Farewell | super::super::intent_router::IntentCategory::Thanks) {
                importance = 0.15;
            }
            importance.min(1.0)
        };

        inv.store_temporal(&temporal_content, "user_interaction", temporal_importance);

        // Phase 2, L2: Learning Feedback Loop — track hot topics
        let topic_key = extract_primary_topic(text);
        if !topic_key.is_empty() {
            let hot_topic_key = format!("hot_topic:{}", topic_key);
            inv.store_temporal(&hot_topic_key, "hot_topic", 0.6);
        }

        // Phase 2, L3: Session Momentum Tracking — record outcome
        if success {
            inv.record_session_success();
        } else {
            inv.record_session_failure();
        }
        // Detect corrections from user text
        let lower_text = text.to_lowercase();
        if lower_text.contains("actually") || lower_text.contains("no, ") || lower_text.contains("that's wrong")
            || lower_text.contains("i meant") || lower_text.contains("not what i")
        {
            inv.record_session_correction();
        }

        let _ = tx.send(CognitiveUpdate::TemporalStored {
            category: "user_interaction".to_string(),
            content: text.to_string(),
        });

        // Pattern evolution — evolve tracked patterns periodically
        if inv.pattern_count() >= 3 {
            if let Some(evo_summary) = inv.evolve_patterns() {
                let _ = tx.send(CognitiveUpdate::PatternEvolved {
                    summary: evo_summary,
                });
            }
        }
    }

    let learn_ms = learn_start.elapsed().as_millis() as u64;
    let _ = tx.send(CognitiveUpdate::Typing(false));
    let _ = tx.send(CognitiveUpdate::PhaseStatuses(vec![
        PhaseStatus { phase: CognitivePhase::Perceive, state: PhaseState::Completed, tokens_used: Some(0), duration_ms: Some(perceive_ms) },
        PhaseStatus { phase: CognitivePhase::Think, state: PhaseState::Completed, tokens_used: Some(input_tokens + output_tokens), duration_ms: Some(think_ms) },
        PhaseStatus { phase: CognitivePhase::Decide, state: PhaseState::Completed, tokens_used: Some(0), duration_ms: Some(decide_ms) },
        PhaseStatus { phase: CognitivePhase::Act, state: PhaseState::Completed, tokens_used: Some(0), duration_ms: Some(act_ms) },
        PhaseStatus { phase: CognitivePhase::Learn, state: PhaseState::Completed, tokens_used: Some(0), duration_ms: Some(learn_ms) },
    ]));

    // ═══════════════════════════════════════════════════════════
    // DELIVER — Show final response to user
    // ═══════════════════════════════════════════════════════════
    let _ = tx.send(CognitiveUpdate::Message {
        role: "hydra".into(),
        content: final_response.to_string(),
        css_class: "message hydra".into(),
    });

    if !is_simple {
        let _ = tx.send(CognitiveUpdate::PlanStepComplete { index: usize::MAX, duration_ms: Some(learn_ms) });
    }

    if llm_result.is_ok() {
        let _ = tx.send(CognitiveUpdate::Phase("Done".into()));
        let _ = tx.send(CognitiveUpdate::IconState("success".into()));
        if !is_simple {
            let _ = tx.send(CognitiveUpdate::Celebrate("Done".into()));
        }
    } else {
        let _ = tx.send(CognitiveUpdate::Phase("Error".into()));
        let _ = tx.send(CognitiveUpdate::IconState("error".into()));
    }
    let _ = tx.send(CognitiveUpdate::SidebarCompleteTask(config.task_id.clone()));

    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
    let _ = tx.send(CognitiveUpdate::ResetIdle);
}
