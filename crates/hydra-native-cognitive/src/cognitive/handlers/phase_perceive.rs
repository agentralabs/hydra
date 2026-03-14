//! PERCEIVE phase — queries sisters for context, memories, beliefs, MCP tools, federation state.

use std::sync::Arc;
use tokio::sync::mpsc;

use crate::cognitive::inventions::InventionEngine;
use crate::sisters::SistersHandle;
use hydra_native_state::state::hydra::{CognitivePhase, PhaseState, PhaseStatus};
use hydra_native_state::utils::generate_deliverable_steps;
use hydra_db::{HydraDb, FederationStateRow};

use super::super::loop_runner::CognitiveUpdate;
use super::memory_intent;

/// Output of the PERCEIVE phase, consumed by THINK and ACT.
pub(crate) struct PerceiveResult {
    pub perceived: serde_json::Value,
    pub always_on_memory: Option<String>,
    pub beliefs_context: Option<String>,
    pub federation_context: Option<String>,
    pub skills_context: Option<String>,
    pub code_index_context: Option<String>,
    pub perceive_ms: u64,
    /// Hash of memory response for dedup detection across queries.
    pub memory_hash: u64,
    /// UCU: Task plan from iterative_planner (complex tasks only).
    pub task_plan: Option<crate::cognitive::iterative_planner::TaskPlan>,
    /// Cognitive Amplification: cross-sister synapse context.
    pub synapse_context: Option<String>,
    /// Cognitive Amplification: causal model context for "why"/"what if" queries.
    pub causal_context: Option<String>,
    /// Production orchestrator: detected deliverable plan.
    pub production_context: Option<String>,
    /// Cognitive Amplification: meta-reasoning strategy for this query.
    pub meta_strategy: Option<String>,
}

/// Run the PERCEIVE phase: gather context from sisters, memory, beliefs, MCP, federation.
pub(crate) async fn run_perceive(
    text: &str,
    config: &super::super::loop_runner::CognitiveLoopConfig,
    is_simple: bool,
    is_complex: bool,
    sisters_handle: &Option<SistersHandle>,
    inventions: &Option<Arc<InventionEngine>>,
    proactive_notifier: &Option<Arc<parking_lot::Mutex<hydra_native_state::proactive::ProactiveNotifier>>>,
    federation: &Option<Arc<hydra_native_state::federation::FederationManager>>,
    db: &Option<Arc<HydraDb>>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> PerceiveResult {
    use std::time::Instant;

    let _ = tx.send(CognitiveUpdate::Phase("Perceive".into()));
    let _ = tx.send(CognitiveUpdate::IconState("listening".into()));
    let _ = tx.send(CognitiveUpdate::PhaseStatuses(vec![
        PhaseStatus { phase: CognitivePhase::Perceive, state: PhaseState::Running, tokens_used: None, duration_ms: None },
    ]));
    let perceive_start = Instant::now();

    // UCU input_decomposer: detect and report large inputs for context-aware processing
    if crate::cognitive::input_decomposer::needs_decomposition(text, 8000) {
        let content_type = crate::cognitive::input_decomposer::detect_content_type(text);
        let chunks = crate::cognitive::input_decomposer::decompose_input(text, 4000);
        eprintln!("[hydra:input] Large input: {} chars, {:?}, {} chunks", text.len(), content_type, chunks.len());
        let _ = tx.send(CognitiveUpdate::EvidenceMemory {
            title: "Input Analysis".into(),
            content: format!("{} chars of {:?} content ({} sections detected)", text.len(), content_type, chunks.len()),
        });
    }

    // SESSION RESUME: Bootstrap from last session for continuity ("where did we stop?")
    let session_context = if config.session_count == 0 {
        if let Some(ref sh) = sisters_handle {
            let (resume_ctx, _, _, _, contract_ctx, _) = tokio::join!(
                sh.memory_session_resume(), sh.memory_session_start(&config.user_name),
                sh.comm_session_start(&config.user_name), sh.time_session_start(&config.user_name),
                sh.contract_session_resume(), sh.aegis_session_create(&config.user_name));
            for (title, ctx) in [("Previous Session", &resume_ctx), ("Contract Context", &contract_ctx)] {
                if let Some(ref c) = ctx {
                    let _ = tx.send(CognitiveUpdate::EvidenceMemory { title: title.into(), content: c.clone() });
                }
            }
            resume_ctx
        } else { None }
    } else { None };

    // Surface dream insights from idle processing (gated by runtime settings)
    if config.runtime.dream_state {
        if let Some(ref inv) = inventions {
            inv.tick_idle(0);
            inv.reset_idle();
            if let Some(insights) = inv.surface_insights(0.6) {
                let _ = tx.send(CognitiveUpdate::EvidenceMemory {
                    title: "Dream Insights".to_string(),
                    content: insights,
                });
            }
            if let Some(dream_text) = inv.maybe_dream() {
                let _ = tx.send(CognitiveUpdate::DreamInsight {
                    category: "idle_processing".to_string(),
                    description: dream_text.clone(),
                    confidence: 0.7,
                });
            }
        }
    }

    // Planning sister: recover interrupted goals for context
    if let Some(ref sh) = sisters_handle {
        if let Some(goals) = sh.planning_list_active().await {
            for goal in &goals {
                let _ = tx.send(CognitiveUpdate::EvidenceMemory {
                    title: format!("Active goal: {}", goal.name),
                    content: format!("{:.0}% complete", goal.progress * 100.0),
                });
            }
        }
    }

    // Comm: check inbox for pending messages during perceive
    if let Some(ref sh) = sisters_handle {
        if let Some(messages) = sh.comm_check_inbox(&config.user_name, 10).await {
            if !messages.is_empty() {
                let summary = messages.iter()
                    .map(|m| format!("[{}] {}: {}", m.message_type, m.from, m.content))
                    .collect::<Vec<_>>().join("\n");
                let _ = tx.send(CognitiveUpdate::EvidenceMemory { title: "Pending Messages".into(), content: summary });
            }
        }
    }

    // Contract: check for any pending approvals
    if let Some(ref sh) = sisters_handle {
        if let Some(pending) = sh.contract_query_approvals("last_24h").await {
            let _ = tx.send(CognitiveUpdate::EvidenceMemory {
                title: "Pending Approvals".to_string(),
                content: pending,
            });
        }
    }

    // Setup workspace panels — UCU iterative_planner for complex task decomposition
    let task_plan = if is_simple {
        let _ = tx.send(CognitiveUpdate::PlanClear);
        let _ = tx.send(CognitiveUpdate::TimelineClear);
        let _ = tx.send(CognitiveUpdate::EvidenceClear);
        None
    } else {
        let complexity = if is_complex { "complex" } else { "moderate" };
        let intent_for_plan = crate::cognitive::intent_router::ClassifiedIntent {
            category: crate::cognitive::intent_router::IntentCategory::Unknown,
            confidence: 0.5, target: None, payload: None,
        };
        let plan = crate::cognitive::iterative_planner::decompose_task(text, &intent_for_plan, complexity);
        let steps: Vec<String> = plan.steps.iter().map(|s| s.description.clone()).collect();
        eprintln!("[hydra:planner] Decomposed into {} steps, est {} tokens, {} parallel groups",
            plan.steps.len(), plan.estimated_total_tokens, plan.parallelizable_groups.len());
        let _ = tx.send(CognitiveUpdate::PlanInit { goal: text.to_string(), steps });
        let _ = tx.send(CognitiveUpdate::PlanStepStart(0));
        let _ = tx.send(CognitiveUpdate::TimelineClear);
        let _ = tx.send(CognitiveUpdate::EvidenceClear);
        Some(plan)
    };

    // Query sisters for perceived context
    let perceived = if let Some(ref sh) = sisters_handle {
        if is_simple {
            eprintln!("[hydra:perceive] SIMPLE mode — memory + cognition only");
            sh.perceive_simple(text).await
        } else {
            eprintln!("[hydra:perceive] COMPLEX mode — all sisters");
            sh.perceive(text).await
        }
    } else {
        serde_json::json!({
            "input": text,
            "involves_code": false,
            "involves_vision": false,
        })
    };

    // Smart memory retrieval — intent-aware tool selection
    // Phase 5.5 P1: Causal chain queries for "why" questions
    let always_on_memory = if let Some(ref sh) = sisters_handle {
        if crate::sisters::memory_deep::is_why_question(text) {
            eprintln!("[hydra:perceive] P5.5: causal chain query for 'why' question");
            let causal = sh.memory_causal_query(text).await;
            if causal.is_some() { causal }
            else { memory_intent::smart_memory_recall(text, sh, is_simple).await }
        } else if crate::sisters::memory_deep::is_past_reference(text) {
            eprintln!("[hydra:perceive] P5.5: specific node retrieval for past reference");
            let node = sh.memory_get_node(text).await;
            if node.is_some() { node }
            else { memory_intent::smart_memory_recall(text, sh, is_simple).await }
        } else {
            memory_intent::smart_memory_recall(text, sh, is_simple).await
        }
    } else { None };

    // Enrich memory with session context + déjà vu + prediction for continuity
    let always_on_memory = {
        let mut parts: Vec<String> = Vec::new();
        if let Some(ref s) = session_context { parts.push(format!("### Previous Session:\n{}", s)); }
        if let Some(ref m) = always_on_memory { parts.push(m.clone()); }
        if let Some(dj) = perceived.get("dejavu_context").and_then(|v| v.as_str()) {
            parts.push(format!("### Returning Topic:\n{}", dj));
        }
        if let Some(p) = perceived.get("memory_prediction").and_then(|v| v.as_str()) {
            parts.push(format!("### Predicted Context:\n{}", p));
        }
        if parts.is_empty() { None } else { Some(parts.join("\n\n")) }
    };

    // Belief loading from DB
    let belief_limit = if is_simple { 5 } else { 20 };
    let beliefs_context = if let Some(ref db) = db {
        match db.get_active_beliefs(belief_limit) {
            Ok(beliefs) if !beliefs.is_empty() => {
                let summary: String = beliefs.iter()
                    .map(|b| format!("- {} [{}]: {} (confidence: {:.0}%)", b.subject, b.category, b.content, b.confidence * 100.0))
                    .collect::<Vec<_>>()
                    .join("\n");
                let _ = tx.send(CognitiveUpdate::BeliefsLoaded {
                    count: beliefs.len(),
                    summary: summary.clone(),
                });
                Some(summary)
            }
            _ => None,
        }
    } else { None };

    // Aegis: validate user input for security threats
    if let Some(ref sh) = sisters_handle {
        if let Some(validation) = sh.aegis_validate_input(text).await {
            if !validation.safe {
                let _ = tx.send(CognitiveUpdate::EvidenceMemory {
                    title: "Input Security Check".to_string(),
                    content: format!("[{:?}] {}", validation.severity, validation.reason),
                });
            }
        }
    }

    // Code index: inject relevant symbols for complex queries
    let code_index_context = if !is_simple {
        if let Some(ref db) = db {
            super::code_index_query::query_relevant_symbols(text, db)
        } else { None }
    } else { None };

    // MCP skill discovery (complex only) — delegated to perceive_mcp
    let _mcp_context = if !is_complex {
        None
    } else if let Some(ref sh) = sisters_handle {
        super::perceive_mcp::discover_mcp_skills(sh, db, tx)
    } else { None };

    // Skill discovery — match registered skills against user input
    let skills_context = {
        let registry = hydra_skills::SkillRegistry::new();
        for skill in hydra_skills::builtin_skills() {
            if let Err(e) = registry.register(skill) { eprintln!("[hydra:skills] Failed to register skill: {}", e); }
        }
        let matches = registry.discover(text);
        if !matches.is_empty() {
            let summary: String = matches.iter()
                .map(|m| format!("- {} (confidence: {:.0}%, trigger: {:?})", m.name, m.confidence * 100.0, m.trigger))
                .collect::<Vec<_>>()
                .join("\n");
            Some(summary)
        } else { None }
    };

    // Federation context (complex only)
    let federation_context = if !is_complex {
        None
    } else if let Some(ref fed) = federation {
        if fed.is_enabled() {
            let peer_count = fed.peer_count();
            let available = fed.registry.available_peers().len();
            let federation_state = fed.sync.version();
            if let Some(ref db) = db {
                for peer in fed.registry.list() {
                    let _ = db.upsert_federation_peer(&FederationStateRow {
                        peer_id: peer.id.clone(),
                        peer_name: Some(peer.name.clone()),
                        endpoint: peer.endpoint.clone(),
                        trust_level: format!("{:?}", peer.trust_level),
                        capabilities: Some(serde_json::to_string(&peer.capabilities.sisters).unwrap_or_default()),
                        federation_type: format!("{:?}", peer.federation_type),
                        last_sync_version: 0,
                        last_seen: peer.last_seen.clone(),
                        active_tasks: peer.active_tasks as i64,
                        active: true,
                    });
                }
            }
            let _ = tx.send(CognitiveUpdate::FederationSync {
                peers_online: peer_count,
                last_sync_version: federation_state as i64,
            });
            if peer_count > 0 {
                Some(format!("Federation: {} peers registered, {} available for delegation (sync v{})", peer_count, available, federation_state))
            } else { None }
        } else { None }
    } else { None };

    // Time: check for upcoming deadlines (complex only)
    if !is_simple {
        if let Some(ref sh) = sisters_handle {
            if let Some(deadline_info) = sh.time_check_deadline(text).await {
                let _ = tx.send(CognitiveUpdate::EvidenceMemory {
                    title: "Deadline Check".to_string(),
                    content: deadline_info,
                });
            }
        }
    }

    // Reality: probe environment for grounding context (complex only)
    if !is_simple {
        if let Some(ref sh) = sisters_handle {
            let env_profile = sh.reality_probe_environment().await;
            let env_info = env_profile.summary();
            if !env_info.is_empty() {
                let _ = tx.send(CognitiveUpdate::EvidenceMemory {
                    title: "Environment Context".to_string(),
                    content: env_info,
                });
            }
        }
    }

    let perceive_ms = perceive_start.elapsed().as_millis() as u64;

    // Proactive: anticipate needs (gated by runtime settings)
    if config.runtime.proactive {
        if let Some(ref notifier) = proactive_notifier {
            let mut n = notifier.lock();
            n.anticipate(text);
            for alert in n.drain() {
                let _ = tx.send(CognitiveUpdate::ProactiveAlert {
                    title: alert.title,
                    message: alert.message,
                    priority: format!("{:?}", alert.priority),
                });
            }
        }
    }

    if !is_simple {
        if let Some(mem) = perceived["memory_context"].as_str() {
            let _ = tx.send(CognitiveUpdate::EvidenceMemory { title: "Relevant memories".into(), content: mem.into() });
        }
        if let Some(code) = perceived["codebase_context"].as_str() {
            let _ = tx.send(CognitiveUpdate::EvidenceCode { title: "Codebase analysis".into(), content: code.into(), language: None, file_path: None });
        }
        for (key, title) in [("deadlines", "Upcoming Deadlines"), ("commitments_due", "Commitments Due")] {
            if let Some(d) = perceived[key].as_str() { let _ = tx.send(CognitiveUpdate::EvidenceMemory { title: title.into(), content: d.into() }); }
        }
        let _ = tx.send(CognitiveUpdate::PlanStepComplete { index: 0, duration_ms: Some(perceive_ms) });
        let _ = tx.send(CognitiveUpdate::PlanStepStart(1));
    }

    let _ = tx.send(CognitiveUpdate::PhaseStatuses(vec![
        PhaseStatus { phase: CognitivePhase::Perceive, state: PhaseState::Completed, tokens_used: Some(0), duration_ms: Some(perceive_ms) },
        PhaseStatus { phase: CognitivePhase::Think, state: PhaseState::Running, tokens_used: None, duration_ms: None },
    ]));
    let memory_hash = always_on_memory.as_deref().map(memory_intent::hash_memory_response).unwrap_or(0);

    // Cognitive Amplification: sister synapse for complex queries
    let synapse_context = match (is_complex, sisters_handle.as_ref()) {
        (true, Some(sh)) => {
            let cfg = crate::knowledge::sister_synapse::SynapseConfig::default();
            let r = crate::knowledge::sister_synapse::synapse_query(text, sh, &cfg).await;
            crate::knowledge::sister_synapse::format_for_prompt(&r)
        }
        _ => None,
    };
    // Cognitive Amplification: causal model + meta-reasoning + production
    let tl = text.to_lowercase();
    let causal_context = if tl.contains("why") || tl.contains("what if") || tl.contains("cause") {
        let g = crate::knowledge::causal_model::CausalGraph::new();
        let w = tl.split_whitespace().find(|w| w.len() >= 4).unwrap_or("query");
        let t = g.propagate(w, 4, 0.3); if t.total_nodes > 1 { Some(crate::knowledge::causal_model::format_causal_tree(&t)) } else { None }
    } else { None };
    let meta_strategy = if is_complex { let mr = crate::knowledge::meta_reasoning::MetaReasoner::new();
        let d = tl.split_whitespace().find(|w| w.len() >= 4).unwrap_or("general");
        Some(mr.format_for_prompt(mr.select_strategy(d)))
    } else { None };
    let production_context = crate::knowledge::production_orchestrator::detect_production_intent(text)
        .map(|dt| crate::knowledge::production_orchestrator::format_plan(
            &crate::knowledge::production_orchestrator::plan_production(&dt, text)));
    PerceiveResult { perceived, always_on_memory, beliefs_context, federation_context,
        skills_context, code_index_context, perceive_ms, memory_hash,
        task_plan, synapse_context, causal_context, meta_strategy, production_context }
}
