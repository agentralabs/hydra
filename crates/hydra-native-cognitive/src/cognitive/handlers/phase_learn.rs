//! LEARN + DELIVER phase. Belief persistence in `phase_learn_beliefs`.

use std::sync::Arc;
use tokio::sync::mpsc;

use crate::cognitive::inventions::InventionEngine;
use crate::sisters::SistersHandle;
use hydra_native_state::state::hydra::{CognitivePhase, PhaseState, PhaseStatus};
use hydra_native_state::utils::{safe_truncate, strip_emojis};

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
    let rt = &config.runtime;
    if let Some(ref sh) = sisters_handle {
        if llm_result.is_ok() {
            // Only learn from corrections if the setting is enabled
            if rt.learn_corrections {
                sh.learn(&user_text, final_response).await;
            }

            // Planning: update goal progress from this interaction
            sh.learn_planning(&user_text, safe_truncate(final_response, 200)).await;

            // Comm: share learnings + command results with peers for cross-session context
            let comm_msg = if !all_exec_results.is_empty() {
                let exec_brief: String = all_exec_results.iter().take(3)
                    .map(|(c, _, ok)| format!("{}: {}", if *ok {"OK"} else {"FAIL"}, safe_truncate(c, 40)))
                    .collect::<Vec<_>>().join("; ");
                format!("{} [{}]", safe_truncate(&user_text, 80), exec_brief)
            } else { format!("Completed: {}", safe_truncate(&user_text, 100)) };
            sh.learn_comm_share(&comm_msg).await;

            // Phase 5.5 P1: Store decisions with proper edge types
            let lower = user_text.to_lowercase();
            if lower.contains("let's ") || lower.contains("i decided")
                || lower.contains("we should") || lower.contains("i want to")
            {
                sh.memory_store_decision(
                    &user_text,
                    safe_truncate(final_response, 200),
                    text,
                ).await;
            }

            // Phase 5.5 P1: Store command outputs as evidence
            for (cmd, output, success) in all_exec_results {
                sh.memory_store_evidence(cmd, output, *success).await;
            }

            // Memory: capture command execution details for recall
            for (cmd, output, success) in all_exec_results {
                sh.learn_capture_command(cmd, output, *success).await;
            }

            // Phase 5.5 P1: Ghost Writer summary for long sessions
            if config.history.len() >= 20 && config.history.len() % 10 == 0 {
                if let Some(summary) = sh.memory_ghost_write(&config.history).await {
                    let _ = tx.send(CognitiveUpdate::EvidenceMemory {
                        title: "Session Summary".into(),
                        content: summary,
                    });
                }
            }

            // Phase 5.5 P2: Crystallize receipt in Contract sister
            sh.contract_crystallize(
                &user_text,
                safe_truncate(final_response, 100),
                "low",
                true,
            ).await;

            // Phase 5.5 P7: Record action pattern in Evolve sister
            sh.evolve_record_pattern(
                &format!("{:?}:{}", intent.category, safe_truncate(text, 50)),
                true,
            ).await;

            // Phase 5.5 P7: Update user model in Cognition sister
            sh.cognition_model_update_session(
                safe_truncate(final_response, 200),
                config.history.len() as u32,
            ).await;

            // Evolve: suggest improvements based on interaction patterns
            if is_complex {
                if let Some(suggestion) = sh.evolve_suggest_improvement().await {
                    let _ = tx.send(CognitiveUpdate::EvidenceMemory {
                        title: "Evolution Suggestion".to_string(),
                        content: suggestion,
                    });
                }
            }

            // Contract: record the decision made in this interaction
            sh.contract_record_decision(
                &config.task_id,
                llm_result.is_ok(),
                safe_truncate(final_response, 100),
            ).await;

            // Planning: complete goal if one was created in ACT phase
            // (goal_id passed through ActResult)
            if is_complex {
                sh.planning_complete_goal(
                    &config.task_id,
                    safe_truncate(final_response, 100),
                ).await;
            }

            // Memory: store test results and error resolutions
            for (cmd, output, success) in all_exec_results {
                if !success {
                    sh.memory_store_resolution(output, "Failed — check logs").await;
                }
                if cmd.contains("test") {
                    let (pass, fail) = if *success { (1, 0) } else { (0, 1) };
                    sh.memory_store_test_results(cmd, "rust", pass, fail, 1).await;
                }
            }
        }
    }

    let rt = &config.runtime;
    if let Some(ref sh) = sisters_handle {
        eprintln!("[hydra:learn] capture mode={} memory_sister={}", rt.memory_capture, sh.memory.is_some());
        if rt.should_capture_all() { sh.memory_capture_exchange(text, final_response).await; sh.comm_session_log(text, final_response).await; }
        else if rt.should_capture_facts() { sh.comm_session_log(text, final_response).await; }
        if let Some(stats_raw) = sh.memory_stats().await {
            let (f, t, r) = parse_memory_stats(&stats_raw);
            eprintln!("[hydra:learn] memory_stats: facts={} tokens={} receipts={}", f, t, r);
            let _ = tx.send(CognitiveUpdate::MemoryStatsUpdate { facts: f, tokens_avg: t, receipts: r });
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

    if let Some(ref sh) = sisters_handle {
        sh.contract_context_log(safe_truncate(text, 200), safe_truncate(final_response, 200)).await;
        if sh.identity_fingerprint_build().await.is_none() {
            eprintln!("[hydra:learn] identity_fingerprint_build returned None");
        }
    }
    let msg_count = config.history.len();
    if msg_count > 0 {
        if let Some(ref sh) = sisters_handle {
            if msg_count % 10 == 0 {
                sh.memory_metabolism_process().await;
                for (title, result) in [
                    ("Belief Drift", sh.cognition_drift_track().await),
                    ("Pattern Optimization", sh.evolve_optimize().await),
                ] { if let Some(c) = result { let _ = tx.send(CognitiveUpdate::EvidenceMemory { title: title.into(), content: c }); } }
                let lc = hydra_model::llm_config::LlmConfig::from_env();
                super::phase_dream::run_light(&config.active_beliefs, sh, &lc, tx).await;
            }
            if msg_count % 15 == 0 {
                if let Some(c) = sh.planning_identify_themes().await {
                    let _ = tx.send(CognitiveUpdate::EvidenceMemory { title: "Recurring Themes".into(), content: c });
                }
            }
            if msg_count % 20 == 0 { let lc2 = hydra_model::llm_config::LlmConfig::from_env(); super::phase_dream::run_deep(&config.active_beliefs, sh, &lc2, tx).await; }
            if msg_count % 20 == 0 {
                for (title, result) in [
                    ("Knowledge Gaps", sh.memory_meta_gaps(text).await),
                    ("Trust Trajectory", sh.identity_trust_project("global").await),
                    ("Stale Knowledge", sh.time_decay_alert().await),
                    ("Blind Spots", sh.cognition_shadow_map().await),
                    ("Memory Health", sh.memory_immune_scan().await),
                    ("Calibration", sh.memory_meta_calibration().await),
                    ("Visual Memory", sh.vision_consolidate("session").await),
                ] { if let Some(c) = result { let _ = tx.send(CognitiveUpdate::EvidenceMemory { title: title.into(), content: c }); } }
                if sh.memory_dream_start("periodic consolidation").await.is_none() {
                    eprintln!("[hydra:learn] memory_dream_start returned None");
                }
            }
            if msg_count % 50 == 0 { let _ = sh.memory_crystal_create("session crystallization").await; }
        }
    }

    let learn_ms = learn_start.elapsed().as_millis() as u64;
    // Round 6: Economic tracking — auto-detect value from this interaction
    crate::knowledge::economics_tracker::auto_track(text, perceive_ms + think_ms + act_ms + learn_ms, llm_ok);
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
    // When LLM succeeded, StreamChunk already delivered the visible response.
    // Use css_class "history-only" to signal UIs not to add a duplicate bubble.
    // ═══════════════════════════════════════════════════════════
    let css = if llm_result.is_ok() { "history-only" } else { "message hydra" };
    let _ = tx.send(CognitiveUpdate::Message {
        role: "hydra".into(),
        content: strip_emojis(final_response),
        css_class: css.into(),
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

/// Parse memory stats from Memory sister MCP response. Returns (facts, tokens_avg, receipts).
pub(crate) fn parse_memory_stats(raw: &str) -> (u64, u64, u64) {
    let (mut facts, mut tokens, mut receipts) = (0u64, 0u64, 0u64);
    for line in raw.lines() {
        let l = line.to_lowercase();
        let num = || -> Option<u64> { l.split(|c: char| !c.is_ascii_digit()).find(|s| !s.is_empty())?.parse().ok() };
        if l.contains("node_count") || l.contains("total_events") { facts = num().unwrap_or(facts); }
        else if l.contains("session_count") { tokens = num().unwrap_or(tokens); }
        else if l.contains("edge_count") { receipts = num().unwrap_or(receipts); }
    }
    (facts, tokens, receipts)
}
