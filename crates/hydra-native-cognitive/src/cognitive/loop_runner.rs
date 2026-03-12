//! Cognitive loop runner — 5-phase (Perceive→Think→Decide→Act→Learn) decoupled from UI.
//!
//! Sends `CognitiveUpdate` messages via `tokio::sync::mpsc` so the UI can
//! dispatch to Dioxus signals without the loop knowing about the rendering layer.

use std::sync::Arc;
use tokio::sync::mpsc;

use crate::cognitive::decide::DecideEngine;
use crate::cognitive::inventions::InventionEngine;
use crate::cognitive::spawner::AgentSpawner;
use crate::sisters::SistersHandle;
use hydra_native_state::state::hydra::PhaseStatus;
use hydra_native_state::utils::safe_truncate;
use hydra_db::HydraDb;
use hydra_runtime::approval::ApprovalManager;
use hydra_runtime::undo::UndoStack;

/// Updates emitted by the cognitive loop for the UI to consume.
#[derive(Debug, Clone)]
pub enum CognitiveUpdate {
    /// Set the current phase label (e.g. "Perceive", "Think").
    Phase(String),
    /// Set the icon state (e.g. "listening", "working", "success").
    IconState(String),
    /// Replace the full phase status vector.
    PhaseStatuses(Vec<PhaseStatus>),
    /// Show/hide the typing indicator.
    Typing(bool),

    // -- Plan panel --
    /// Initialize plan panel for a complex task.
    PlanInit { goal: String, steps: Vec<String> },
    /// Clear plan panel (simple task).
    PlanClear,
    /// Mark a plan step as started.
    PlanStepStart(usize),
    /// Mark a plan step as completed with optional duration.
    PlanStepComplete { index: usize, duration_ms: Option<u64> },

    // -- Evidence panel --
    /// Clear evidence panel.
    EvidenceClear,
    /// Add a memory context evidence item.
    EvidenceMemory { title: String, content: String },
    /// Add a code evidence item.
    EvidenceCode {
        title: String,
        content: String,
        language: Option<String>,
        file_path: Option<String>,
    },

    // -- Timeline panel --
    /// Clear timeline panel.
    TimelineClear,

    // -- Messages --
    /// Append a message to the conversation.
    Message { role: String, content: String, css_class: String },

    // -- Sidebar --
    /// Mark a task as completed in the sidebar.
    SidebarCompleteTask(String),

    // -- Celebration --
    /// Show a small celebration toast.
    Celebrate(String),

    // -- Final state --
    /// Reset to idle after completion.
    ResetIdle,

    /// Suggest mode based on complexity (Step 4.7: mode auto-selection).
    SuggestMode(String),

    // -- Approval flow (Step 3.7) --
    /// Request user approval before proceeding. UI should render an ApprovalCard.
    AwaitApproval {
        /// Unique ID for this approval request (used to submit decision back)
        approval_id: Option<String>,
        risk_level: String,
        action: String,
        description: String,
        challenge_phrase: Option<String>,
    },

    // -- Natural language settings (Step 4.9) --
    /// A settings mutation was detected and applied.
    SettingsApplied { confirmation: String },

    // -- Sister visibility (Step 4.8) --
    /// Report which sisters were called for this query.
    SistersCalled { sisters: Vec<String> },

    // -- Token budget (Step 3.10) --
    /// Report token usage for budget tracking.
    TokenUsage { input_tokens: u64, output_tokens: u64 },

    // -- Streaming (Step 4.2) --
    /// Append a streaming token chunk (partial message).
    StreamChunk { content: String },
    /// Streaming complete — finalize message.
    StreamComplete,

    // -- Undo/Redo (Sprint 1, Task 5) --
    /// Undo stack status (can_undo, can_redo, last_description)
    UndoStatus { can_undo: bool, can_redo: bool, last_action: Option<String> },

    // -- Proactive notifications (Sprint 2, Task 10) --
    /// Proactive notification alert
    ProactiveAlert { title: String, message: String, priority: String },

    // -- Sprint 4 inventions --
    /// Sprint 4: Skill crystallized from repeated pattern
    SkillCrystallized { name: String, actions_count: usize },
    /// Sprint 4: Metacognition reflection insight
    ReflectionInsight { insight: String },
    /// Sprint 4: Token compression applied
    CompressionApplied { original_tokens: usize, compressed_tokens: usize, ratio: f64 },
    /// Dream insight surfaced from idle processing
    DreamInsight { category: String, description: String, confidence: f64 },
    /// Shadow validation result
    ShadowValidation { safe: bool, recommendation: String },
    /// Future echo prediction result
    PredictionResult { action: String, confidence: f64, recommendation: String },
    /// Pattern mutation/evolution completed
    PatternEvolved { summary: String },
    /// Temporal memory stored
    TemporalStored { category: String, content: String },

    // -- Ghost Cursor --
    /// Move the ghost cursor to screen coordinates.
    CursorMove { x: f64, y: f64, label: Option<String> },
    /// Ghost cursor click animation.
    CursorClick,
    /// Ghost cursor typing animation.
    CursorTyping { active: bool },
    /// Show/hide the ghost cursor.
    CursorVisibility { visible: bool },
    /// Set cursor mode (visible, fast, invisible, replay).
    CursorModeChange { mode: String },
    /// Cursor paused (user interaction detected).
    CursorPaused { paused: bool },

    // -- Belief system --
    /// Active beliefs loaded during PERCEIVE phase.
    BeliefsLoaded { count: usize, summary: String },
    /// A belief was updated or created during LEARN phase.
    BeliefUpdated { subject: String, content: String, confidence: f64, is_new: bool },

    // -- MCP Skill Discovery --
    /// MCP skills discovered and registered.
    McpSkillsDiscovered { server: String, tools: Vec<String>, count: usize },

    // -- Federation --
    /// Federation state synced.
    FederationSync { peers_online: usize, last_sync_version: i64 },
    /// Federation task delegated to a peer.
    FederationDelegated { peer_name: String, task_summary: String },

    // -- Self-Repair --
    /// Self-repair started for a spec.
    RepairStarted { spec: String, task: String },
    /// Self-repair check result.
    RepairCheckResult { name: String, passed: bool },
    /// Self-repair iteration progress.
    RepairIteration { iteration: u32, passed: usize, total: usize },
    /// Self-repair completed.
    RepairCompleted { task: String, status: String, iterations: u32 },

    // -- Omniscience Loop --
    /// Omniscience codebase analysis phase.
    OmniscienceAnalyzing { phase: String },
    /// Omniscience gap found.
    OmniscienceGapFound { description: String, severity: String, category: String },
    /// Omniscience spec generated via Forge.
    OmniscienceSpecGenerated { spec_name: String, task: String },
    /// Omniscience Aegis validation result.
    OmniscienceValidation { spec_name: String, safe: bool, recommendation: String },
    /// Omniscience scan complete.
    OmniscienceScanComplete { gaps_found: usize, specs_generated: usize, health_score: f64 },

    // -- Phase 3, C5: Phase-specific loading states --
    /// Phase-specific loading message with elapsed time for meaningful loading states.
    PhaseLoading { phase: String, elapsed_ms: u64 },

    // -- Phase 3, C4: Consolidation daemon --
    /// Consolidation cycle completed.
    ConsolidationCycleComplete { cycle: u64, strengthened: usize, decayed: usize, gc_cleaned: usize },

    // -- Obstacle resolution (Phase 5, Priority 2) --
    /// An obstacle was detected and is being diagnosed.
    ObstacleDetected { pattern: String, error_summary: String },
    /// Obstacle resolution attempt completed.
    ObstacleResolved { pattern: String, resolution: String, attempts: usize },

    // -- Autonomous project execution (Phase 5, Priority 6) --
    /// Project execution phase progress.
    ProjectExecPhase { repo: String, phase: String, detail: String },
}

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
    /// When set, this is preferred over anthropic_key for API calls.
    pub anthropic_oauth_token: Option<String>,
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
) {
    use crate::sisters::Sisters;

    let text = &config.text;
    eprintln!("[hydra:loop] INPUT: {:?}", safe_truncate(text, 120));

    // ═══════════════════════════════════════════════════════════
    // INTENT CLASSIFICATION — Micro-LLM classifier (~150 tokens)
    // Uses cheapest model (Haiku) to understand MEANING.
    // Works in any language, any phrasing, any slang.
    // ═══════════════════════════════════════════════════════════
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

    // ═══════════════════════════════════════════════════════════
    // PRE-PHASE DISPATCH — route by classified intent
    // Extracted to handlers/dispatch.rs and handlers/sister_ops.rs
    // ═══════════════════════════════════════════════════════════
    use super::handlers::dispatch;
    use super::handlers::sister_ops;

    // Crystallized skill shortcut — bypass LLM for learned patterns
    if dispatch::handle_crystallized_skill(text, &inventions, &tx).await { return; }

    // Greeting / Farewell / Thanks — instant response
    if dispatch::handle_greeting_farewell_thanks(&intent, &config, &tx) { return; }

    // Dependency/usage queries pre-check — before memory recall misroute
    if dispatch::handle_dep_query_precheck(text, &sisters_handle, &tx).await { return; }

    // Memory recall — natural conversational response
    if dispatch::handle_memory_recall(text, &intent, &config, &sisters_handle, &tx).await { return; }

    // Natural language settings detection
    if dispatch::handle_settings(text, &intent, &tx) { return; }

    // Self-repair — "fix yourself" intent
    if sister_ops::handle_self_repair(text, &intent, &tx).await { return; }

    // Omniscience scan — full semantic self-repair
    if sister_ops::handle_omniscience_scan(&intent, &sisters_handle, &tx).await { return; }

    // Self-implement — self-modification pipeline (Forge first, LLM fallback)
    if sister_ops::handle_self_implement(text, &intent, &config, &sisters_handle, &approval_manager, &tx).await { return; }

    if sister_ops::handle_sister_diagnose(text, &intent, &sisters_handle, &tx).await { return; }
    if sister_ops::handle_sister_repair(text, &intent, &sisters_handle, &tx).await { return; }
    if sister_ops::handle_sister_improve(text, &intent, &tx).await { return; }
    if sister_ops::handle_threat_query(text, &intent, &tx) { return; }
    if dispatch::handle_memory_store(text, &intent, &sisters_handle, &tx).await { return; }
    if dispatch::handle_project_exec_natural(text, &tx).await { return; }
    // Slash commands — /test, /files, /git, /build, /run, etc.
    if dispatch::handle_slash_command(text, &tx).await { return; }
    // Direct action fast-path — execute immediately, skip LLM
    if dispatch::handle_direct_action(text, &sisters_handle, &decide_engine, &tx).await { return; }

    // ═══════════════════════════════════════════════════════════
    // CLASSIFY — Determine complexity and risk BEFORE anything
    // ═══════════════════════════════════════════════════════════
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

    // ═══════════════════════════════════════════════════════════
    // PHASE 1: PERCEIVE — extracted to handlers/phase_perceive.rs
    // ═══════════════════════════════════════════════════════════
    use super::handlers::phase_perceive;
    use super::handlers::phase_think;

    let perceive = phase_perceive::run_perceive(
        text, is_simple, is_complex,
        &sisters_handle, &inventions, &proactive_notifier, &federation, &db, &tx,
    ).await;
    let perceive_ms = perceive.perceive_ms;

    // ═══════════════════════════════════════════════════════════
    // PHASE 2: THINK — extracted to handlers/phase_think.rs
    // ═══════════════════════════════════════════════════════════
    let think = phase_think::run_think(
        text, &config, &intent, &perceive,
        is_simple, is_complex, is_action_request, &complexity, risk_level,
        &sisters_handle, &decide_engine, &inventions, &spawner, &tx,
    ).await;
    let response_text = think.response_text;
    let active_model = think.active_model;
    let provider = &think.provider;
    let input_tokens = think.input_tokens;
    let output_tokens = think.output_tokens;
    let think_ms = think.think_ms;
    let llm_config = think.llm_config;
    let llm_ok = think.llm_ok;

    // ═══════════════════════════════════════════════════════════
    // PHASE 3: DECIDE — extracted to handlers/phase_decide.rs
    // ═══════════════════════════════════════════════════════════
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

    // ═══════════════════════════════════════════════════════════
    // PHASE 4: ACT — extracted to handlers/phase_act.rs
    // ═══════════════════════════════════════════════════════════
    let act = phase_act::run_act(
        text, &config, &response_text,
        is_simple, is_complex, llm_ok,
        &llm_config, provider, &active_model,
        risk_level, decide.gate_decision,
        &decide_engine, &sisters_handle, &undo_stack, &db,
        input_tokens, output_tokens,
        perceive_ms, think_ms, decide_ms,
        &tx,
    ).await;

    // ═══════════════════════════════════════════════════════════
    // PHASE 5: LEARN + DELIVER — extracted to handlers/phase_learn.rs
    // ═══════════════════════════════════════════════════════════
    phase_learn::run_learn(
        text, &config, &act.final_response,
        is_simple, is_complex, llm_ok,
        &llm_config, &active_model, &intent,
        &sisters_handle, &inventions, &db, &federation,
        &act.all_exec_results, is_action_request,
        perceive_ms, think_ms, decide_ms, act.act_ms,
        input_tokens, output_tokens,
        &tx,
    ).await;
}
