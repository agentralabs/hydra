//! Cognitive loop runner — 5-phase (Perceive→Think→Decide→Act→Learn) decoupled from UI.
//!
//! Sends `CognitiveUpdate` messages via `tokio::sync::mpsc` so the UI can
//! dispatch to Dioxus signals without the loop knowing about the rendering layer.
//!
//! CognitiveUpdate enum lives in `cognitive_update.rs` (extracted for file size).

use std::sync::Arc;
use tokio::sync::mpsc;

use crate::cognitive::decide::DecideEngine;
use crate::cognitive::inventions::InventionEngine;
use crate::cognitive::spawner::AgentSpawner;
use crate::sisters::{SistersHandle, SisterGateway};
use crate::swarm::SwarmManager;
use hydra_native_state::utils::safe_truncate;
use hydra_db::HydraDb;
use hydra_runtime::approval::ApprovalManager;
use hydra_runtime::undo::UndoStack;

// Re-export CognitiveUpdate from extracted module
pub use super::cognitive_update::CognitiveUpdate;

/// Configuration for the cognitive loop (read-only inputs).
#[derive(Debug, Clone)]
pub struct CognitiveLoopConfig {
    pub text: String,
    pub anthropic_key: String,
    pub openai_key: String,
    pub google_key: String,
    pub model: String,
    pub user_name: String,
    pub task_id: String,
    pub history: Vec<(String, String)>,
    pub session_count: u32,
    /// OAuth bearer token for Anthropic (from browser-based auth / Claude Max subscription).
    pub anthropic_oauth_token: Option<String>,
    /// Runtime behavior settings from UI
    pub runtime: super::runtime_settings::RuntimeSettings,
    /// Operational profile prompt overlay (Phase 6)
    pub prompt_overlay: Option<String>,
    /// Active profile beliefs for Living Knowledge Engine
    pub active_beliefs: Vec<hydra_native_state::operational_profile::ProfileBelief>,
}

/// Run the 5-phase cognitive loop, sending updates via the channel.
pub async fn run_cognitive_loop(
    config: CognitiveLoopConfig,
    sisters_handle: Option<SistersHandle>,
    tx: mpsc::UnboundedSender<CognitiveUpdate>,
    decide_engine: Arc<DecideEngine>,
    undo_stack: Option<Arc<parking_lot::Mutex<UndoStack>>>,
    inventions: Option<Arc<InventionEngine>>,
    proactive_notifier: Option<Arc<parking_lot::Mutex<hydra_native_state::proactive::ProactiveNotifier>>>,
    spawner: Option<Arc<AgentSpawner>>,
    approval_manager: Option<Arc<ApprovalManager>>,
    db: Option<Arc<HydraDb>>,
    federation: Option<Arc<hydra_native_state::federation::FederationManager>>,
    swarm_manager: Option<Arc<SwarmManager>>,
) {
    use crate::sisters::Sisters;

    // Create SisterGateway — enforces sister-first pattern with stats tracking
    let gateway: Option<Arc<SisterGateway>> = sisters_handle.as_ref()
        .map(|sh| Arc::new(SisterGateway::new(sh.clone())));

    let text = &config.text;
    let debug = config.runtime.debug_mode;
    eprintln!("[hydra:loop] INPUT: {:?}", safe_truncate(text, 120));
    if debug { eprintln!("[hydra:debug] runtime_settings: risk={} dispatch={} sister_timeout={}", config.runtime.risk_threshold, config.runtime.dispatch_mode, config.runtime.sister_timeout); }

    // Register as active agent in comm network (once per session)
    if config.session_count == 0 { if let Some(ref sh) = &sisters_handle { sh.comm_register_agent(&config.user_name, "primary").await; } }

    // ── MORNING BRIEFING — show on first message of session ──
    if config.session_count == 0 && !config.active_beliefs.is_empty() {
        let b = crate::knowledge::morning_briefing::generate(&config.user_name, &config.active_beliefs, "active");
        let _ = tx.send(CognitiveUpdate::EvidenceMemory { title: "Morning Briefing".into(), content: crate::knowledge::morning_briefing::format_briefing(&b) });
    }
    // ── CAPABILITY REGISTRY — pattern-match BEFORE LLM classification ──
    use super::handlers::dispatch_capability;
    // Shared threat correlator — persistent across queries in this loop
    let threat_correlator: Option<Arc<parking_lot::RwLock<crate::threat::ThreatCorrelator>>> = Some(
        Arc::new(parking_lot::RwLock::new(crate::threat::ThreatCorrelator::new()))
    );
    let remote_executor: Option<Arc<parking_lot::RwLock<crate::remote::RemoteExecutor>>> = Some(
        Arc::new(parking_lot::RwLock::new(crate::remote::RemoteExecutor::new()))
    );
    if dispatch_capability::handle_capability_match(text, &swarm_manager, &threat_correlator, &remote_executor, &sisters_handle, &tx).await {
        return;
    }

    // ── UTILITY SISTER PRE-DISPATCH — call Data/Connect/Workflow directly (0 LLM tokens) ──
    if super::handlers::dispatch_utility::handle_data_operation(text, &sisters_handle, &tx).await {
        return;
    }
    if super::handlers::dispatch_utility::handle_connect_operation(text, &sisters_handle, &tx).await {
        return;
    }

    // ── INTENT CLASSIFICATION — Micro-LLM classifier (~150 tokens) ──
    let classify_llm_config = hydra_model::LlmConfig::from_env_with_overlay(
        &config.anthropic_key,
        &config.openai_key,
        config.anthropic_oauth_token.as_deref(),
    );
    let has_classify_key = classify_llm_config.anthropic_api_key.is_some()
        || classify_llm_config.openai_api_key.is_some();
    eprintln!("[hydra:intent] classifier_mode={} anthropic_key={} openai_key={}",
        if has_classify_key { "MICRO_LLM" } else { "EMERGENCY_FALLBACK" },
        if classify_llm_config.anthropic_api_key.is_some() { "SET" } else { "NONE" },
        if classify_llm_config.openai_api_key.is_some() { "SET" } else { "NONE" },
    );
    let veritas_ref = sisters_handle.as_ref().and_then(|sh| sh.veritas.as_ref());
    let intent = super::intent_router::classify(text, veritas_ref, &config.history, &classify_llm_config).await;
    eprintln!("[hydra:intent] category={:?} confidence={:.2} target={:?}",
        intent.category, intent.confidence, intent.target);

    // ── PRE-PHASE DISPATCH — route by classified intent ──
    use super::handlers::dispatch;
    use super::handlers::sister_ops;

    let pre_dispatched =
        dispatch::handle_crystallized_skill(text, &inventions, &decide_engine, &tx).await
        || dispatch::handle_greeting_farewell_thanks(&intent, &config, &tx)
        || dispatch::handle_dep_query_precheck(text, &sisters_handle, &tx).await
        || dispatch::handle_memory_recall(text, &intent, &config, &sisters_handle, &tx).await
        || dispatch::handle_settings(text, &intent, &tx)
        || sister_ops::handle_self_repair(text, &intent, &tx).await
        || sister_ops::handle_omniscience_scan(&intent, &sisters_handle, &tx).await
        || sister_ops::handle_build_system(text, &intent, &config, &sisters_handle, &approval_manager, &tx).await
        || sister_ops::handle_self_implement(text, &intent, &config, &sisters_handle, &approval_manager, &tx).await
        || sister_ops::handle_sister_diagnose(text, &intent, &sisters_handle, &tx).await
        || sister_ops::handle_sister_repair(text, &intent, &sisters_handle, &tx).await
        || sister_ops::handle_sister_improve(text, &intent, &config, &sisters_handle, &tx).await
        || sister_ops::handle_threat_query(text, &intent, &tx)
        || dispatch::handle_memory_store(text, &intent, &sisters_handle, &tx).await
        || dispatch::handle_project_exec_natural(text, &sisters_handle, &tx).await
        || dispatch::handle_slash_command(text, &decide_engine, &tx).await
        || dispatch::handle_direct_action(text, &sisters_handle, &decide_engine, &tx).await;

    if pre_dispatched {
        // Memory capture for pre-dispatch handlers (they bypass the LEARN phase)
        if let Some(ref sh) = sisters_handle {
            if config.runtime.should_capture_all() {
                eprintln!("[hydra:memory] Pre-dispatch capture for: {}", safe_truncate(text, 80));
                sh.memory_capture_exchange(text, "(quick response)").await;
            }
            // Emit memory stats so UI dashboard stays current
            if let Some(stats_raw) = sh.memory_stats().await {
                let (f, t, r) = super::handlers::phase_learn::parse_memory_stats(&stats_raw);
                let _ = tx.send(CognitiveUpdate::MemoryStatsUpdate { facts: f, tokens_avg: t, receipts: r });
            }
        }
        return;
    }

    // ── CLASSIFY — Determine complexity and risk ──
    let complexity = Sisters::classify_complexity(text);
    let risk_level = Sisters::assess_risk(text);
    // Action detection is now intent-based — categories with direct handlers are "actions"
    let is_action_request = intent.category.has_direct_handler() && intent.confidence >= 0.6;
    // "simple" AND "moderate" use the lightweight path (few sisters, small prompt).
    // Only "complex" or explicit action intents get full 15-sister treatment.
    let is_simple = (complexity == "simple" || complexity == "moderate") && !is_action_request;
    let is_complex = complexity == "complex" || is_action_request;
    eprintln!("[hydra:classify] complexity={:?} is_action={} is_simple={} is_complex={}", complexity, is_action_request, is_simple, is_complex);

    // Step 4.7: Auto-suggest mode based on complexity
    let suggested_mode = if is_simple { "companion" } else { "workspace" };
    let _ = tx.send(CognitiveUpdate::SuggestMode(suggested_mode.into()));

    // ── INTELLIGENCE INIT — Phases 3/6/7: Outcome tracking + calibration ──
    use super::handlers::phase_learn_intelligence;
    let mut outcome_tracker = super::outcome_tracker::OutcomeTracker::new();
    let mut calibration_tracker = super::metacognition::CalibrationTracker::new();
    // Load persisted intelligence from DB (cross-session memory)
    if let Some(ref db) = db {
        phase_learn_intelligence::load_from_db(
            &mut outcome_tracker, &mut calibration_tracker, db,
        );
    }
    // Supplement with current session history
    phase_learn_intelligence::populate_from_history(
        &mut outcome_tracker, &mut calibration_tracker, &config.history,
    );
    let category_success_rate = outcome_tracker.category_success_rate(intent.category);
    eprintln!("[hydra:intelligence] cat_success_rate={:.2} total_tracked={}",
        category_success_rate, outcome_tracker.total_interactions());

    // Phase 7: Metacognitive assessment before THINK
    phase_learn_intelligence::assess_and_report(
        &intent, &complexity, &outcome_tracker, &calibration_tracker, &tx,
    );

    // ── USER MODEL — adaptive personalization ──
    let mut user_model = super::user_model::UserModel::new();
    if let Some(ref db) = db {
        if let Ok(traits) = db.load_user_traits() {
            let rows: Vec<(String, String, f64, i64)> = traits.iter()
                .map(|t| (t.trait_key.clone(), t.trait_value.clone(), t.confidence, t.observation_count))
                .collect();
            user_model.load_from_db(&rows);
            if !rows.is_empty() {
                eprintln!("[hydra:user_model] Loaded {} traits from DB", rows.len());
            }
        }
    }

    // ── PHASE 1: PERCEIVE ──
    use super::handlers::phase_perceive;
    use super::handlers::phase_think;

    let perceive = phase_perceive::run_perceive(
        text, &config, is_simple, is_complex,
        &sisters_handle, &inventions, &proactive_notifier, &federation, &db, &tx,
    ).await;
    let perceive_ms = perceive.perceive_ms;
    if debug { eprintln!("[hydra:debug] PERCEIVE completed in {}ms", perceive_ms); }

    // ── PHASE 1.5: COMPILED REASONING — record pattern for future compilation ──
    {
        let ps = crate::knowledge::compiled_reasoning::pattern_store();
        if let Ok(mut store) = ps.lock() {
            let steps = vec![format!("{:?}:{}", intent.category, complexity)];
            if let Some(name) = store.record_observation(&complexity, &steps, true) {
                eprintln!("[hydra:compiled] New pattern compiled: {}", name);
            }
        }
    }

    // ── PHASE 2: THINK ──
    let think = phase_think::run_think(
        text, &config, &intent, &perceive,
        is_simple, is_complex, is_action_request, &complexity, risk_level,
        &sisters_handle, &decide_engine, &inventions, &spawner, &gateway, &tx,
    ).await;
    let response_text = think.response_text;
    let active_model = think.active_model;
    let provider = &think.provider;
    let input_tokens = think.input_tokens;
    let output_tokens = think.output_tokens;
    let think_ms = think.think_ms;
    let llm_config = think.llm_config;
    let llm_ok = think.llm_ok;

    // ── PHASE 3: DECIDE ──
    use super::handlers::phase_decide;
    use super::handlers::phase_act;
    use super::handlers::phase_learn;

    let decide = match phase_decide::run_decide(
        text, risk_level, is_simple, is_action_request,
        &intent, &config, &decide_engine, &inventions,
        &sisters_handle, &approval_manager, &db,
        &llm_config, &active_model,
        perceive_ms, think_ms, input_tokens, output_tokens,
        &tx,
    ).await {
        Some(d) => d,
        None => return, // Aborted (approval denied, clarification, timeout)
    };
    let decide_ms = decide.decide_ms;

    // ── PHASE 4: ACT ──
    let act = phase_act::run_act(
        text, &config, &response_text,
        is_simple, is_complex, llm_ok,
        &llm_config, provider, &active_model,
        risk_level, decide.gate_decision,
        &perceive.always_on_memory,
        &perceive.task_plan,
        &decide_engine, &sisters_handle, &undo_stack, &db,
        input_tokens, output_tokens,
        perceive_ms, think_ms, decide_ms,
        &gateway, &tx,
    ).await;

    // ── PHASE 4b: VERIFY RESPONSE (Phase 2 — Claim-Level Verification) ──
    let verified_response = if let Some(ref sh) = sisters_handle {
        use super::handlers::verify_response;
        let verification = verify_response::verify_response(
            &act.final_response, text, sh, &intent,
        ).await;
        if verification.claims_corrected > 0 {
            let _ = tx.send(CognitiveUpdate::VerificationApplied {
                checked: verification.claims_checked,
                corrected: verification.claims_corrected,
            });
            eprintln!("[hydra:verify] {}/{} claims corrected in {}ms",
                verification.claims_corrected, verification.claims_checked, verification.verification_ms);
            verification.verified_response
        } else {
            act.final_response.clone()
        }
    } else {
        act.final_response.clone()
    };

    // ── PHASE 4c: BELIEF VERIFICATION (Cognitive Amplification) ──
    let verified_response = {
        let beliefs = config.prompt_overlay.as_ref()
            .map(|_| Vec::new()) // Beliefs are in the overlay
            .unwrap_or_default();
        let vr = crate::knowledge::reasoning_verifier::verify_response(
            &verified_response, &beliefs, &config.history,
        );
        if let Some(modified) = vr.modified_response {
            eprintln!("[hydra:belief_verify] {} issues found, response modified", vr.issues.len());
            modified
        } else {
            verified_response
        }
    };

    // ── PHASE 5: LEARN + DELIVER ──
    phase_learn::run_learn(
        text, &config, &verified_response,
        is_simple, is_complex, llm_ok,
        &llm_config, &active_model, &intent,
        &sisters_handle, &inventions, &db, &federation,
        &act.all_exec_results, is_action_request,
        perceive_ms, think_ms, decide_ms, act.act_ms,
        input_tokens, output_tokens,
        &tx,
    ).await;

    // ── GATEWAY STATS — emit sister-first usage metrics ──
    if let Some(ref gw) = gateway {
        let (s, f) = gw.stats();
        if s + f > 0 {
            eprintln!("[hydra:gateway] Session stats: {} sister calls, {} fallbacks", s, f);
            let _ = tx.send(CognitiveUpdate::GatewayStats { display: gw.stats_display() });
        }
    }

    // ── UCU OUTCOME VALIDATION — functional result verification ──
    let validation = super::outcome_validator::validate_outcome(
        &super::outcome_validator::ValidationContext {
            user_input: text,
            intent: intent.category,
            response: &verified_response,
            exec_results: &act.all_exec_results,
            plan: perceive.task_plan.as_ref(),
            llm_ok,
        }
    );
    eprintln!("[hydra:validate] score={:.2} valid={} passed={} failed={}",
        validation.score, validation.valid, validation.checks_passed.len(), validation.checks_failed.len());

    // ── POST-LEARN INTELLIGENCE — Phases 3/5/6 ──
    // Phase 3: Record this interaction's outcome (using UCU validation)
    let llm_outcome = match validation.suggested_outcome {
        super::outcome_validator::SuggestedOutcome::Success => super::outcome_tracker::Outcome::Success,
        super::outcome_validator::SuggestedOutcome::PartialSuccess => super::outcome_tracker::Outcome::Success,
        super::outcome_validator::SuggestedOutcome::Failure => super::outcome_tracker::Outcome::Failure,
    };
    let topic = hydra_native_state::utils::safe_truncate(text, 50).to_string();
    outcome_tracker.record(
        intent.category, &topic, &active_model,
        llm_outcome.clone(), input_tokens + output_tokens,
    );

    // Phase 3+7: Persist intelligence to DB (cross-session memory)
    if let Some(ref db) = db {
        phase_learn_intelligence::save_to_db(
            &intent, &topic, &active_model, &llm_outcome,
            input_tokens + output_tokens, &calibration_tracker, db,
        );
    }

    // User model: observe this interaction and persist
    user_model.observe_interaction(text, &verified_response, llm_ok);
    if let Some(ref db) = db {
        for (key, value, confidence) in user_model.traits_for_db() {
            let _ = db.save_user_trait(key, value, confidence);
        }
    }

    // Phase 6: Self-improvement check
    phase_learn_intelligence::check_self_improvement(&outcome_tracker, &tx);

    // Phase 5: Background scheduler — mark idle, check due tasks
    let mut bg_scheduler = super::background_tasks::BackgroundScheduler::new();
    bg_scheduler.user_idle();
    let due_info: Vec<(String, String)> = bg_scheduler.due_tasks().iter()
        .map(|t| (t.name.clone(), format!("{:?} ({:?})", t.task_type, t.priority)))
        .collect();
    for (name, summary) in &due_info {
        let _ = tx.send(CognitiveUpdate::BackgroundTaskComplete {
            task_name: name.clone(),
            summary: format!("Scheduled: {}", summary),
        });
        eprintln!("[hydra:background] Due task: {}", name);
        bg_scheduler.mark_completed(name);
    }
}
