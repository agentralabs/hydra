//! Cognitive loop runner — 5-phase (Perceive→Think→Decide→Act→Learn) decoupled from UI.
//!
//! Sends `CognitiveUpdate` messages via `tokio::sync::mpsc` so the UI can
//! dispatch to Dioxus signals without the loop knowing about the rendering layer.

use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;

use crate::cognitive::decide::DecideEngine;
use crate::cognitive::inventions::InventionEngine;
use crate::cognitive::spawner::AgentSpawner;
use crate::sisters::SistersHandle;
use crate::state::hydra::{CognitivePhase, PhaseState, PhaseStatus};
use crate::utils::{detect_language, extract_json_plan, format_bytes, generate_deliverable_steps};
use hydra_db::HydraDb;
use hydra_runtime::approval::{ApprovalDecision, ApprovalManager};
use hydra_runtime::undo::{UndoStack, FileCreateAction};

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
}

/// Simple hash for receipt chain (non-cryptographic, for audit trail integrity)
fn md5_simple(input: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}

/// Run the 5-phase cognitive loop, sending updates via the channel.
pub async fn run_cognitive_loop(
    config: CognitiveLoopConfig,
    sisters_handle: Option<SistersHandle>,
    tx: mpsc::UnboundedSender<CognitiveUpdate>,
    decide_engine: Arc<DecideEngine>,
    undo_stack: Option<Arc<parking_lot::Mutex<UndoStack>>>,
    inventions: Option<Arc<InventionEngine>>,
    proactive_notifier: Option<Arc<parking_lot::Mutex<crate::proactive::ProactiveNotifier>>>,
    spawner: Option<Arc<AgentSpawner>>,
    approval_manager: Option<Arc<ApprovalManager>>,
    db: Option<Arc<HydraDb>>,
) {
    use crate::sisters::Sisters;

    let text = &config.text;

    // ═══════════════════════════════════════════════════════════
    // Step 4.9: Natural language settings detection
    // ═══════════════════════════════════════════════════════════
    if is_settings_intent(text) {
        let mut settings = crate::state::settings::SettingsStore::default();
        if let Some(confirmation) = settings.apply_natural_language(text) {
            let _ = tx.send(CognitiveUpdate::SettingsApplied { confirmation: confirmation.clone() });
            let _ = tx.send(CognitiveUpdate::Message {
                role: "hydra".into(),
                content: confirmation,
                css_class: "message hydra settings-applied".into(),
            });
            let _ = tx.send(CognitiveUpdate::ResetIdle);
            return;
        }
    }

    // ═══════════════════════════════════════════════════════════
    // CLASSIFY — Determine complexity and risk BEFORE anything
    // ═══════════════════════════════════════════════════════════
    let complexity = Sisters::classify_complexity(text);
    let risk_level = Sisters::assess_risk(text);
    // Action requests ("install it", "run it", "start it") should be treated as complex
    // so they go through the JSON plan execution path
    let is_action_request = is_action_intent(text);
    let is_simple = complexity == "simple" && !is_action_request;
    let is_complex = complexity == "complex" || is_action_request;

    // Step 4.7: Auto-suggest mode based on complexity
    let suggested_mode = if is_simple { "companion" } else { "workspace" };
    let _ = tx.send(CognitiveUpdate::SuggestMode(suggested_mode.into()));

    // ═══════════════════════════════════════════════════════════
    // PHASE 1: PERCEIVE — Query ALL sisters for context
    // ═══════════════════════════════════════════════════════════
    let _ = tx.send(CognitiveUpdate::Phase("Perceive".into()));
    let _ = tx.send(CognitiveUpdate::IconState("listening".into()));
    let _ = tx.send(CognitiveUpdate::PhaseStatuses(vec![
        PhaseStatus { phase: CognitivePhase::Perceive, state: PhaseState::Running, tokens_used: None, duration_ms: None },
    ]));
    let perceive_start = Instant::now();

    // Surface any dream insights from idle processing (inventions integration)
    if let Some(ref inv) = inventions {
        inv.reset_idle(); // User is active now
        if let Some(insights) = inv.surface_insights(0.6) {
            let _ = tx.send(CognitiveUpdate::EvidenceMemory {
                title: "Dream Insights".to_string(),
                content: insights,
            });
        }

        // Dream insights → send as DreamInsight update for UI tracking
        if let Some(dream_text) = inv.maybe_dream() {
            let _ = tx.send(CognitiveUpdate::DreamInsight {
                category: "idle_processing".to_string(),
                description: dream_text.clone(),
                confidence: 0.7,
            });
        }
    }

    // Setup workspace panels based on complexity
    if is_simple {
        let _ = tx.send(CognitiveUpdate::PlanClear);
        let _ = tx.send(CognitiveUpdate::TimelineClear);
        let _ = tx.send(CognitiveUpdate::EvidenceClear);
    } else {
        let steps = generate_deliverable_steps(text);
        let _ = tx.send(CognitiveUpdate::PlanInit {
            goal: text.clone(),
            steps: steps.clone(),
        });
        let _ = tx.send(CognitiveUpdate::PlanStepStart(0));
        let _ = tx.send(CognitiveUpdate::TimelineClear);
        let _ = tx.send(CognitiveUpdate::EvidenceClear);
    }

    // REAL PERCEIVE: Dispatch to ALL available sisters in parallel
    let perceived = if let Some(ref sh) = sisters_handle {
        sh.perceive(text).await
    } else {
        serde_json::json!({
            "input": text,
            "involves_code": false,
            "involves_vision": false,
        })
    };

    let perceive_ms = perceive_start.elapsed().as_millis() as u64;

    // Proactive: anticipate needs based on input
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

    // Add perceived context as evidence (complex tasks only)
    if !is_simple {
        if let Some(mem) = perceived["memory_context"].as_str() {
            let _ = tx.send(CognitiveUpdate::EvidenceMemory {
                title: "Relevant memories".into(),
                content: mem.into(),
            });
        }
        if let Some(code) = perceived["codebase_context"].as_str() {
            let _ = tx.send(CognitiveUpdate::EvidenceCode {
                title: "Codebase analysis".into(),
                content: code.into(),
                language: None,
                file_path: None,
            });
        }
        let _ = tx.send(CognitiveUpdate::PlanStepComplete { index: 0, duration_ms: Some(perceive_ms) });
        let _ = tx.send(CognitiveUpdate::PlanStepStart(1));
    }

    let _ = tx.send(CognitiveUpdate::PhaseStatuses(vec![
        PhaseStatus { phase: CognitivePhase::Perceive, state: PhaseState::Completed, tokens_used: Some(0), duration_ms: Some(perceive_ms) },
        PhaseStatus { phase: CognitivePhase::Think, state: PhaseState::Running, tokens_used: None, duration_ms: None },
    ]));

    // ═══════════════════════════════════════════════════════════
    // PHASE 2: THINK — Build cognitive prompt + call LLM
    // ═══════════════════════════════════════════════════════════
    let _ = tx.send(CognitiveUpdate::Phase("Think".into()));
    let _ = tx.send(CognitiveUpdate::IconState("working".into()));
    let think_start = Instant::now();

    // Sub-agent spawning for complex tasks (parallel decomposition)
    if let Some(ref spawner) = spawner {
        if spawner.should_spawn(text) {
            let subtasks = spawner.decompose(text);
            let session_id = spawner.create_session(text, &subtasks);
            let _ = tx.send(CognitiveUpdate::Phase(format!(
                "Spawning {} sub-agents for parallel execution",
                subtasks.len()
            )));
            // Log the decomposition — actual parallel execution comes in Sprint 4
            for st in &subtasks {
                let _ = tx.send(CognitiveUpdate::PlanStepStart(0));
                eprintln!("[hydra] Sub-agent {}: {}", st.module, st.description);
            }
            spawner.complete_session(&session_id);
        }
    }

    // ── Forge blueprinting: generate architecture before LLM for complex builds ──
    let forge_blueprint = if is_complex {
        if let Some(ref sh) = sisters_handle {
            let _ = tx.send(CognitiveUpdate::Phase("Think (Forge blueprint)".into()));
            sh.think_forge(text).await
        } else { None }
    } else { None };

    // ── Veritas intent compilation: structured intent parsing ──
    let veritas_intent = if let Some(ref sh) = sisters_handle {
        sh.think_veritas(text).await
    } else { None };

    // Build LLM config with provider auto-fallback
    let mut llm_config = hydra_model::LlmConfig::from_env();
    if !config.anthropic_key.is_empty() {
        llm_config.anthropic_api_key = Some(config.anthropic_key.clone());
    }
    if !config.openai_key.is_empty() {
        llm_config.openai_api_key = Some(config.openai_key.clone());
    }

    let mut active_model = config.model.clone();
    let mut provider = if active_model.contains("claude") {
        "anthropic"
    } else if active_model.contains("gpt") {
        "openai"
    } else if active_model.contains("gemini") {
        "google"
    } else if active_model == "ollama" {
        "ollama"
    } else {
        "anthropic"
    };

    // Auto-fallback: if selected provider has no key, switch to one that does
    let provider_has_key = match provider {
        "anthropic" => llm_config.anthropic_api_key.is_some(),
        "openai" => llm_config.openai_api_key.is_some(),
        "google" => !config.google_key.is_empty(),
        "ollama" => true,
        _ => false,
    };
    if !provider_has_key {
        if llm_config.openai_api_key.is_some() {
            provider = "openai";
            active_model = "gpt-4o".to_string();
        } else if llm_config.anthropic_api_key.is_some() {
            provider = "anthropic";
            active_model = "claude-sonnet-4-6".to_string();
        } else if !config.google_key.is_empty() {
            provider = "google";
            active_model = "gemini-2.0-flash".to_string();
        }
    }

    if provider == "google" && !config.google_key.is_empty() {
        llm_config.openai_api_key = Some(config.google_key.clone());
        llm_config.openai_base_url = "https://generativelanguage.googleapis.com".into();
    }

    let has_key = match provider {
        "anthropic" => llm_config.anthropic_api_key.is_some(),
        "openai" => llm_config.openai_api_key.is_some(),
        "google" => !config.google_key.is_empty(),
        "ollama" => true,
        _ => false,
    };

    // Build COGNITIVE system prompt from perceived sister context
    let system_prompt = if let Some(ref sh) = sisters_handle {
        let mut sp = sh.build_cognitive_prompt(&config.user_name, &perceived, is_complex);
        if let Some(ref blueprint) = forge_blueprint {
            sp.push_str(&format!("\n# Forge Blueprint (Pre-generated Architecture)\n{}\n\n", blueprint));
        }
        if let Some(ref intent) = veritas_intent {
            sp.push_str(&format!("\n# Compiled Intent\n{}\n\n", intent));
        }
        sp
    } else {
        format!(
            "You are Hydra, a cognitive AI orchestrator built by Agentra Labs. \
             You are NOT a chatbot — you are an agentic executor that DOES things. \
             You can execute commands, create projects, access APIs, deploy to cloud, \
             federate across systems, and integrate with any service the user provides credentials for. \
             {}When the user asks you to do something, DO IT — never say \"I can't\" for things you can do. \
             If you need credentials or access, ask for them specifically.",
            if config.user_name.is_empty() { String::new() } else { format!("The user's name is {}. ", config.user_name) }
        )
    };

    // Sprint 4: Token minimizer — compress context to reduce cost
    let system_prompt = if let Some(ref inv) = inventions {
        let (compressed, ratio) = inv.compress_context(&system_prompt);
        if ratio < 0.95 {
            let original_est = (system_prompt.len() + 3) / 4;
            let compressed_est = (compressed.len() + 3) / 4;
            let _ = tx.send(CognitiveUpdate::CompressionApplied {
                original_tokens: original_est,
                compressed_tokens: compressed_est,
                ratio,
            });
        }
        compressed
    } else {
        system_prompt
    };

    // Build messages with conversation history
    let mut api_messages: Vec<hydra_model::providers::Message> = Vec::new();
    for (role, content) in &config.history {
        api_messages.push(hydra_model::providers::Message {
            role: role.clone(),
            content: content.clone(),
        });
    }

    let llm_result = if has_key {
        let request = hydra_model::CompletionRequest {
            model: active_model.clone(),
            messages: api_messages,
            max_tokens: {
                // Use actual model max output limits — don't artificially cap
                let model_max = match active_model.as_str() {
                    m if m.contains("opus") => 32_768,
                    m if m.contains("sonnet") => 16_384,
                    m if m.contains("haiku") => 8_192,
                    m if m.contains("gpt-4o") => 16_384,
                    m if m.contains("gpt-4") => 8_192,
                    m if m.contains("gemini") => 8_192,
                    m if m.contains("deepseek") => 8_000,
                    m if m.contains("ollama") | m.contains("llama") | m.contains("phi") | m.contains("mistral") => 4_096,
                    _ => 16_384,
                };
                // Complex tasks use full model capacity; simple tasks use less
                if is_complex { model_max } else { std::cmp::min(4_096, model_max) }
            },
            temperature: Some(if is_complex { 0.3 } else { 0.7 }),
            system: Some(system_prompt),
        };

        match provider {
            "anthropic" => {
                match hydra_model::providers::anthropic::AnthropicClient::new(&llm_config) {
                    Ok(client) => client.complete(request).await
                        .map(|r| (r.content, r.model, r.input_tokens, r.output_tokens))
                        .map_err(|e| format!("{}", e)),
                    Err(e) => Err(format!("{}", e)),
                }
            }
            "openai" | "google" => {
                match hydra_model::providers::openai::OpenAiClient::new(&llm_config) {
                    Ok(client) => client.complete(request).await
                        .map(|r| (r.content, r.model, r.input_tokens, r.output_tokens))
                        .map_err(|e| format!("{}", e)),
                    Err(e) => Err(format!("{}", e)),
                }
            }
            "ollama" => {
                let mut ollama_config = llm_config.clone();
                ollama_config.openai_api_key = Some("ollama".into());
                ollama_config.openai_base_url = "http://localhost:11434".into();
                match hydra_model::providers::openai::OpenAiClient::new(&ollama_config) {
                    Ok(client) => client.complete(request).await
                        .map(|r| (r.content, r.model, r.input_tokens, r.output_tokens))
                        .map_err(|e| format!("{}", e)),
                    Err(e) => Err(format!("{}", e)),
                }
            }
            _ => Err("Unsupported provider".into()),
        }
    } else {
        Err("No API key configured. Add your key in Settings → API Key.".into())
    };

    let think_ms = think_start.elapsed().as_millis() as u64;
    let (response_text, _actual_model, input_tokens, output_tokens) = match &llm_result {
        Ok((content, model, inp, out)) => (content.clone(), model.clone(), *inp, *out),
        Err(err) => (format!("Error: {}", err), config.model.clone(), 0u64, 0u64),
    };

    // Step 3.10: Report token usage for budget tracking
    let _ = tx.send(CognitiveUpdate::TokenUsage { input_tokens, output_tokens });

    // Step 4.8: Report which sisters were called during perceive
    if let Some(ref sh) = sisters_handle {
        let called_sisters = sh.connected_sisters_list();
        let _ = tx.send(CognitiveUpdate::SistersCalled { sisters: called_sisters });
    }

    if !is_simple {
        let _ = tx.send(CognitiveUpdate::PlanStepComplete { index: 1, duration_ms: Some(think_ms) });
        let _ = tx.send(CognitiveUpdate::PlanStepStart(2));
    }
    let _ = tx.send(CognitiveUpdate::PhaseStatuses(vec![
        PhaseStatus { phase: CognitivePhase::Perceive, state: PhaseState::Completed, tokens_used: Some(0), duration_ms: Some(perceive_ms) },
        PhaseStatus { phase: CognitivePhase::Think, state: PhaseState::Completed, tokens_used: Some(input_tokens + output_tokens), duration_ms: Some(think_ms) },
        PhaseStatus { phase: CognitivePhase::Decide, state: PhaseState::Running, tokens_used: None, duration_ms: None },
    ]));

    // ═══════════════════════════════════════════════════════════
    // PHASE 3: DECIDE — Graduated autonomy + risk gating
    // ═══════════════════════════════════════════════════════════
    let _ = tx.send(CognitiveUpdate::Phase("Decide".into()));
    let _ = tx.send(CognitiveUpdate::IconState("needs-attention".into()));
    let decide_start = Instant::now();

    // Check graduated autonomy — trust level determines what proceeds automatically
    let decide_result = decide_engine.check(risk_level, "");

    // ── Contract policy check: does policy allow this action? ──
    let contract_verdict = if let Some(ref sh) = sisters_handle {
        sh.decide_contract(text, risk_level).await
    } else { None };

    // ── Veritas uncertainty check: how certain are we about the intent? ──
    let _veritas_uncertainty = if let Some(ref sh) = sisters_handle {
        sh.decide_veritas(text).await
    } else { None };

    // If contract says blocked, override gate decision
    let gate_decision = if let Some(ref verdict) = contract_verdict {
        if verdict.to_lowercase().contains("blocked") || verdict.to_lowercase().contains("denied") {
            "requires_approval"
        } else if decide_result.requires_approval && !decide_result.allowed {
            "requires_approval"
        } else if risk_level == "medium" {
            "shadow_first"
        } else {
            "approved"
        }
    } else if decide_result.requires_approval && !decide_result.allowed {
        "requires_approval"
    } else if risk_level == "medium" {
        "shadow_first"
    } else {
        "approved"
    };

    // Report trust-based decision context to the UI
    let _ = tx.send(CognitiveUpdate::Phase(format!(
        "Decide (trust: {:.0}%, {:?})",
        decide_result.trust_score * 100.0,
        decide_result.autonomy_level,
    )));

    // Future Echo: predict outcome before proceeding (inventions integration)
    if let Some(ref inv) = inventions {
        let risk_float: f32 = match risk_level {
            "high" | "critical" => 0.8,
            "medium" => 0.5,
            "low" => 0.2,
            _ => 0.1,
        };
        let (confidence, recommendation, prediction_desc) =
            inv.predict_outcome(text, risk_float);
        let _ = tx.send(CognitiveUpdate::Phase(format!(
            "Prediction: {} (confidence: {:.0}%, risk: {})",
            prediction_desc,
            confidence * 100.0,
            recommendation
        )));
        let _ = tx.send(CognitiveUpdate::PredictionResult {
            action: text.to_string(),
            confidence: confidence as f64,
            recommendation: recommendation.clone(),
        });

        // Shadow validation for medium+ risk actions
        if risk_level == "medium" || risk_level == "high" || risk_level == "critical" {
            let expected = std::collections::HashMap::new();
            let (safe, shadow_rec) = inv.shadow_validate(text, &expected);
            let _ = tx.send(CognitiveUpdate::EvidenceMemory {
                title: "Shadow Validation".to_string(),
                content: format!("Safe: {} | {}", safe, shadow_rec),
            });
            let _ = tx.send(CognitiveUpdate::ShadowValidation {
                safe,
                recommendation: shadow_rec.clone(),
            });
        }
    }

    // Step 3.7: Gate integration — if action requires approval, notify UI
    if gate_decision == "requires_approval" {
        let challenge = if risk_level == "critical" {
            // Generate challenge phrase for critical actions
            let words: Vec<&str> = text.split_whitespace().take(3).collect();
            Some(words.join(" ").to_lowercase())
        } else {
            None
        };
        let _ = tx.send(CognitiveUpdate::IconState("needs-attention".into()));

        // REAL APPROVAL BLOCKING: Use ApprovalManager to wait for user decision
        if let Some(ref mgr) = approval_manager {
            let (req, rx) = mgr.request_approval(
                &config.task_id,
                text,
                None,
                decide_result.trust_score,
                &format!("{} risk action", risk_level),
            );
            // Send the approval ID to UI so buttons can submit decision
            let _ = tx.send(CognitiveUpdate::AwaitApproval {
                approval_id: Some(req.id.clone()),
                risk_level: risk_level.to_string(),
                action: text.clone(),
                description: format!(
                    "This action is classified as {} risk. Trust: {:.0}%, level: {:?}",
                    risk_level,
                    decide_result.trust_score * 100.0,
                    decide_result.autonomy_level,
                ),
                challenge_phrase: challenge,
            });
            tracing::info!("[hydra] Approval requested: {} ({})", req.id, risk_level);

            match mgr.wait_for_approval(&req.id, rx).await {
                Ok(ApprovalDecision::Approved) => {
                    tracing::info!("[hydra] Approval GRANTED: {}", req.id);
                    let _ = tx.send(CognitiveUpdate::Phase("Approved — proceeding".into()));
                }
                Ok(ApprovalDecision::Denied { reason }) => {
                    tracing::warn!("[hydra] Approval DENIED: {} — {}", req.id, reason);
                    let _ = tx.send(CognitiveUpdate::Message {
                        role: "hydra".into(),
                        content: format!("Action denied: {}", reason),
                        css_class: "message hydra error".into(),
                    });
                    let _ = tx.send(CognitiveUpdate::ResetIdle);
                    return; // STOP — do not proceed to ACT phase
                }
                Ok(ApprovalDecision::Modified { new_action }) => {
                    tracing::info!("[hydra] Approval MODIFIED: {} → {}", req.id, new_action);
                    let _ = tx.send(CognitiveUpdate::Phase(format!("Modified: {}", new_action)));
                    // Continue with the modified action
                }
                Err(e) => {
                    tracing::warn!("[hydra] Approval timeout/cancelled: {} — {}", req.id, e);
                    let _ = tx.send(CognitiveUpdate::Message {
                        role: "hydra".into(),
                        content: format!("Approval timed out or was cancelled. Action not executed for safety."),
                        css_class: "message hydra error".into(),
                    });
                    let _ = tx.send(CognitiveUpdate::ResetIdle);
                    return; // STOP — timeout = deny by default
                }
            }
        } else {
            // No approval manager — send approval without ID and pause briefly (dev mode)
            let _ = tx.send(CognitiveUpdate::AwaitApproval {
                approval_id: None,
                risk_level: risk_level.to_string(),
                action: text.clone(),
                description: format!(
                    "This action is classified as {} risk. Trust: {:.0}%, level: {:?}",
                    risk_level, decide_result.trust_score * 100.0, decide_result.autonomy_level,
                ),
                challenge_phrase: challenge,
            });
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    } else if gate_decision == "shadow_first" {
        // Shadow simulation: run action in sandbox first via Aegis sister
        if let Some(ref sh) = sisters_handle {
            if let Some(aegis) = &sh.aegis {
                let _ = aegis.call_tool("shadow_simulate", serde_json::json!({
                    "action": text,
                    "risk_level": risk_level,
                })).await;
            }
        }
    }

    let decide_ms = decide_start.elapsed().as_millis() as u64;

    if !is_simple {
        let _ = tx.send(CognitiveUpdate::PlanStepComplete { index: 2, duration_ms: Some(decide_ms) });
        let _ = tx.send(CognitiveUpdate::PlanStepStart(3));
    }
    let _ = tx.send(CognitiveUpdate::PhaseStatuses(vec![
        PhaseStatus { phase: CognitivePhase::Perceive, state: PhaseState::Completed, tokens_used: Some(0), duration_ms: Some(perceive_ms) },
        PhaseStatus { phase: CognitivePhase::Think, state: PhaseState::Completed, tokens_used: Some(input_tokens + output_tokens), duration_ms: Some(think_ms) },
        PhaseStatus { phase: CognitivePhase::Decide, state: PhaseState::Completed, tokens_used: Some(0), duration_ms: Some(decide_ms) },
        PhaseStatus { phase: CognitivePhase::Act, state: PhaseState::Running, tokens_used: None, duration_ms: None },
    ]));

    // ═══════════════════════════════════════════════════════════
    // PHASE 4: ACT — Execute the plan through sisters
    // ═══════════════════════════════════════════════════════════
    let _ = tx.send(CognitiveUpdate::Phase("Act".into()));
    let _ = tx.send(CognitiveUpdate::IconState("working".into()));
    let act_start = Instant::now();

    let mut final_response = response_text.clone();
    if is_complex && llm_result.is_ok() {
        let json_plan = extract_json_plan(&response_text);
        if let Some(ref plan) = json_plan {
            final_response = execute_json_plan(plan, &tx, &undo_stack).await;

            // Multi-pass deepening: if generated files are shallow stubs, expand them
            let home = std::env::var("HOME").unwrap_or_default();
            let project_dir_name = plan["project_dir"].as_str().unwrap_or("hydra-project");
            let base_dir = format!("{}/projects/{}", home, project_dir_name);
            let summary = plan["summary"].as_str().unwrap_or("Project");
            if let Some(updated) = maybe_deepen_project(
                &base_dir,
                summary,
                &llm_config,
                provider,
                &active_model,
                &tx,
            ).await {
                // Append deepening metrics to the response
                final_response.push_str(&format!(
                    "\n\n### Multi-Pass Deepening\n\
                     | Metric | Value |\n\
                     |--------|-------|\n\
                     | Modules deepened | **{}** |\n\
                     | Files expanded | **{}** |\n\
                     | New total lines | **{}** |\n\
                     | New total size | **{}** |\n",
                    updated.modules_deepened,
                    updated.files_expanded,
                    updated.total_lines,
                    format_bytes(updated.total_bytes),
                ));
            }
        }
    }

    // ── Inline command execution ──
    // Two strategies:
    // 1. Parse <hydra-exec> tags if the LLM included them
    // 2. Detect action intent from the user's message and execute directly
    // EVERY command goes through the execution gate for risk evaluation.
    if llm_result.is_ok() {
        let mut exec_results = Vec::new();

        // Strategy 1: Parse <hydra-exec> tags
        let tagged_commands = extract_inline_commands(&final_response);

        // Strategy 2: Direct intent detection
        let direct_cmd = if tagged_commands.is_empty() {
            detect_direct_action_command(text).or_else(|| detect_system_control(text))
        } else { None };

        let all_commands: Vec<String> = tagged_commands.into_iter()
            .chain(direct_cmd.into_iter())
            .collect();

        for cmd in &all_commands {
            // ══════════════════════════════════════════════════════════
            // FULL SECURITY PIPELINE: Anomaly → Boundary → Risk → Gate
            // ══════════════════════════════════════════════════════════

            // Layer 0-3: evaluate_command does anomaly detection, boundary
            // enforcement, and risk assessment in one call
            let gate_result = decide_engine.evaluate_command(cmd);

            // Also check trust-based autonomy
            let cmd_decide = decide_engine.check(&gate_result.risk_level, cmd);

            // Create receipt BEFORE execution (audit trail)
            if let Some(ref sh) = sisters_handle {
                sh.act_receipt(cmd, &gate_result.risk_level, gate_result.allowed).await;
            }

            // ── BLOCKED: Anomaly detected (burst, exfiltration, destructive) ──
            if gate_result.anomaly_detected {
                let is_critical = gate_result.reason.contains("CRITICAL") || gate_result.reason.contains("exfiltration");
                // CRITICAL: Engage kill switch on destructive/exfiltration anomalies
                if is_critical {
                    decide_engine.kill_switch_engage(&gate_result.reason);
                    let _ = tx.send(CognitiveUpdate::Phase(
                        "🛑 KILL SWITCH ENGAGED — all execution halted".into()
                    ));
                }
                // Persist anomaly event to DB
                if let Some(ref db) = db {
                    let _ = db.create_anomaly_event(&hydra_db::AnomalyEventRow {
                        event_type: if is_critical { "critical".into() } else { "anomaly".into() },
                        command: cmd.clone(),
                        detail: Some(gate_result.reason.clone()),
                        severity: if is_critical { "critical".into() } else { "high".into() },
                        kill_switch_engaged: is_critical,
                    });
                }
                let _ = tx.send(CognitiveUpdate::ShadowValidation {
                    safe: false,
                    recommendation: gate_result.reason.clone(),
                });
                let _ = tx.send(CognitiveUpdate::Phase(format!(
                    "⚠ ANOMALY BLOCKED: {}", &gate_result.reason[..gate_result.reason.len().min(80)]
                )));
                exec_results.push((cmd.clone(), format!("BLOCKED — {}", gate_result.reason), false));
                continue;
            }

            // ── KILL SWITCH CHECK: If engaged, block everything ──
            if decide_engine.is_halted() {
                exec_results.push((cmd.clone(), "BLOCKED — Kill switch is active. All execution halted.".to_string(), false));
                continue;
            }

            // ── BLOCKED: Boundary violation (system paths, self-modification) ──
            if gate_result.boundary_blocked {
                let _ = tx.send(CognitiveUpdate::Phase(format!(
                    "⛔ BOUNDARY BLOCKED: {}", &gate_result.reason[..gate_result.reason.len().min(80)]
                )));
                exec_results.push((cmd.clone(), format!("BLOCKED — {}", gate_result.reason), false));
                continue;
            }

            // ── CRITICAL RISK: Score >= 0.9 — requires explicit approval with challenge ──
            if gate_result.risk_score >= 0.9 {
                if let Some(ref mgr) = approval_manager {
                    let challenge = cmd.split_whitespace().take(3).collect::<Vec<_>>().join(" ");
                    let (req, rx) = mgr.request_approval(
                        &config.task_id, cmd, None, gate_result.risk_score, &gate_result.reason,
                    );
                    let _ = tx.send(CognitiveUpdate::AwaitApproval {
                        approval_id: Some(req.id.clone()),
                        risk_level: "critical".to_string(),
                        action: cmd.clone(),
                        description: gate_result.reason.clone(),
                        challenge_phrase: Some(challenge),
                    });
                    match mgr.wait_for_approval(&req.id, rx).await {
                        Ok(ApprovalDecision::Approved) => {
                            let _ = tx.send(CognitiveUpdate::Phase(format!("Critical action approved: {}", cmd)));
                        }
                        _ => {
                            exec_results.push((cmd.clone(), format!("DENIED — Critical risk ({:.2})", gate_result.risk_score), false));
                            continue;
                        }
                    }
                } else {
                    let _ = tx.send(CognitiveUpdate::AwaitApproval {
                        approval_id: None,
                        risk_level: "critical".to_string(),
                        action: cmd.clone(),
                        description: gate_result.reason.clone(),
                        challenge_phrase: Some(cmd.split_whitespace().take(3).collect::<Vec<_>>().join(" ")),
                    });
                    exec_results.push((cmd.clone(), format!("CRITICAL RISK — {}", gate_result.reason), false));
                    continue;
                }
            }

            // ── REQUIRES APPROVAL: Risk score >= medium ──
            if gate_result.risk_score >= 0.5 || (cmd_decide.requires_approval && !cmd_decide.allowed) {
                // REAL BLOCKING: Wait for user approval via ApprovalManager
                if let Some(ref mgr) = approval_manager {
                    let (req, rx) = mgr.request_approval(
                        &config.task_id, cmd, None, gate_result.risk_score, &gate_result.reason,
                    );
                    // Send approval request to UI WITH the ID so buttons can submit back
                    let _ = tx.send(CognitiveUpdate::AwaitApproval {
                        approval_id: Some(req.id.clone()),
                        risk_level: gate_result.risk_level.clone(),
                        action: cmd.clone(),
                        description: gate_result.reason.clone(),
                        challenge_phrase: None,
                    });
                    match mgr.wait_for_approval(&req.id, rx).await {
                        Ok(ApprovalDecision::Approved) => {
                            let _ = tx.send(CognitiveUpdate::Phase(format!("Approved: {}", cmd)));
                            // Fall through to execute below
                        }
                        _ => {
                            exec_results.push((cmd.clone(), format!(
                                "DENIED — Command requires approval (risk: {:.2})", gate_result.risk_score
                            ), false));
                            continue;
                        }
                    }
                } else {
                    let _ = tx.send(CognitiveUpdate::AwaitApproval {
                        approval_id: None,
                        risk_level: gate_result.risk_level.clone(),
                        action: cmd.clone(),
                        description: gate_result.reason.clone(),
                        challenge_phrase: None,
                    });
                    exec_results.push((cmd.clone(), format!(
                        "Awaiting approval (risk: {:.2})...", gate_result.risk_score
                    ), false));
                    continue;
                }
            }

            // ── Aegis shadow validation for elevated risk (0.3+) ──
            if gate_result.risk_score >= 0.3 {
                if let Some(ref sh) = sisters_handle {
                    if let Some((safe, rec)) = sh.act_aegis_validate(cmd).await {
                        // Persist shadow validation to DB
                        if let Some(ref db) = db {
                            let _ = db.create_shadow_validation(&hydra_db::ShadowValidationRow {
                                action_description: cmd.clone(),
                                safe,
                                divergence_count: if safe { 0 } else { 1 },
                                critical_divergences: if safe { 0 } else { 1 },
                                recommendation: Some(rec.clone()),
                            });
                        }
                        if !safe {
                            let _ = tx.send(CognitiveUpdate::ShadowValidation {
                                safe: false,
                                recommendation: rec.clone(),
                            });
                            exec_results.push((cmd.clone(), format!("Blocked by Aegis: {}", rec), false));
                            continue;
                        }
                    }
                }
            }

            // ═══ ALL GATES PASSED — EXECUTE ═══
            let _ = tx.send(CognitiveUpdate::Phase(format!("Executing: {}", cmd)));

            // Ghost cursor: Show for visual actions (open, browse, UI interaction)
            let is_visual_cmd = cmd.contains("open -a") || cmd.contains("open http")
                || cmd.contains("xdg-open") || cmd.starts_with("open ")
                || cmd.contains("google-chrome") || cmd.contains("firefox");
            if is_visual_cmd {
                let _ = tx.send(CognitiveUpdate::CursorVisibility { visible: true });
                // Animate cursor to center-ish of screen with action label
                let label = if cmd.contains("open -a") || cmd.contains("open ") {
                    let app = cmd.split("open -a ").nth(1)
                        .or_else(|| cmd.split("open ").nth(1))
                        .unwrap_or(cmd)
                        .trim_matches('"');
                    format!("Opening {}", app)
                } else {
                    "Navigating...".into()
                };
                let _ = tx.send(CognitiveUpdate::CursorMove { x: 400.0, y: 300.0, label: Some(label) });
                let _ = tx.send(CognitiveUpdate::CursorClick);
            }

            match tokio::process::Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .output()
                .await
            {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    let combined = if stderr.is_empty() { stdout } else if stdout.is_empty() { stderr } else { format!("{}\n{}", stdout, stderr) };
                    let success = output.status.success();
                    exec_results.push((cmd.clone(), combined.clone(), success));

                    // Record trust outcome
                    if success {
                        decide_engine.record_success(&gate_result.risk_level, cmd);
                    } else {
                        decide_engine.record_failure(&gate_result.risk_level, cmd);
                    }

                    // Persist receipt to DB (hash-chained audit trail)
                    if let Some(ref db) = db {
                        let seq = db.next_receipt_sequence().unwrap_or(1);
                        let prev = db.last_receipt_hash().unwrap_or(None);
                        let hash_input = format!("{}:{}:{}:{}", seq, cmd, success, prev.as_deref().unwrap_or("genesis"));
                        let hash = format!("{:x}", md5_simple(&hash_input));
                        let _ = db.create_receipt(&hydra_db::ReceiptRow {
                            id: uuid::Uuid::new_v4().to_string(),
                            receipt_type: if success { "execution_success".into() } else { "execution_failure".into() },
                            action: cmd.clone(),
                            actor: "hydra".into(),
                            tokens_used: 0,
                            risk_level: Some(gate_result.risk_level.clone()),
                            hash,
                            prev_hash: prev,
                            sequence: seq,
                            created_at: chrono::Utc::now().to_rfc3339(),
                        });
                    }

                    // ── LEARN: Capture every command execution in memory ──
                    if let Some(ref sh) = sisters_handle {
                        sh.learn_capture_command(cmd, &combined, success).await;
                    }

                    // Ghost cursor: Hide after visual command completes
                    if is_visual_cmd {
                        let _ = tx.send(CognitiveUpdate::CursorVisibility { visible: false });
                    }

                    // Record cursor event to DB
                    if is_visual_cmd {
                        if let Some(ref db) = db {
                            let _ = db.record_cursor_event(
                                &config.task_id, 0, "execute",
                                400.0, 300.0,
                                Some(&serde_json::json!({
                                    "command": cmd,
                                    "success": success,
                                }).to_string()),
                            );
                        }
                    }
                }
                Err(e) => {
                    decide_engine.record_failure(&gate_result.risk_level, cmd);
                    exec_results.push((cmd.clone(), format!("Failed: {}", e), false));
                    // Ghost cursor: Hide on error too
                    if is_visual_cmd {
                        let _ = tx.send(CognitiveUpdate::CursorVisibility { visible: false });
                    }
                }
            }
        }

        if !exec_results.is_empty() {
            let cleaned = strip_hydra_exec_tags(&final_response);
            final_response = cleaned;
            for (cmd, output, success) in &exec_results {
                if !output.trim().is_empty() {
                    final_response.push_str(&format!(
                        "\n\n```\n$ {}\n{}\n```",
                        cmd,
                        output.trim()
                    ));
                }
                if !success {
                    final_response.push_str(&format!("\n*(Command `{}` failed)*", cmd));
                }
            }
        }

        // ── Vision: capture web page after URL navigation ──
        if let Some(ref sh) = sisters_handle {
            // Check if any executed command involved opening a URL
            for (cmd, _, success) in &exec_results {
                if *success && (cmd.contains("http://") || cmd.contains("https://") || cmd.contains("open -a")) {
                    // Extract URL if present
                    if let Some(url) = extract_url_from_command(cmd) {
                        if let Some(web_content) = sh.act_vision_capture(&url).await {
                            final_response.push_str(&format!(
                                "\n\n**Web page captured:**\n{}\n",
                                &web_content[..web_content.len().min(500)]
                            ));
                        }
                    }
                }
            }
        }
    }

    // Sign receipt via Identity sister
    if let Some(ref sh) = sisters_handle {
        if let Some(id) = &sh.identity {
            let _ = id.call_tool("receipt_create", serde_json::json!({
                "action": text,
                "risk_level": risk_level,
                "gate_decision": gate_decision,
                "tokens_used": input_tokens + output_tokens,
            })).await;
        }
    }

    // Record trust outcome — success earns trust, failure loses it
    if llm_result.is_ok() {
        decide_engine.record_success(risk_level, "");
    } else {
        decide_engine.record_failure(risk_level, "");
    }

    let act_ms = act_start.elapsed().as_millis() as u64;

    if !is_simple {
        let _ = tx.send(CognitiveUpdate::PlanStepComplete { index: 3, duration_ms: Some(act_ms) });
    }
    let _ = tx.send(CognitiveUpdate::PhaseStatuses(vec![
        PhaseStatus { phase: CognitivePhase::Perceive, state: PhaseState::Completed, tokens_used: Some(0), duration_ms: Some(perceive_ms) },
        PhaseStatus { phase: CognitivePhase::Think, state: PhaseState::Completed, tokens_used: Some(input_tokens + output_tokens), duration_ms: Some(think_ms) },
        PhaseStatus { phase: CognitivePhase::Decide, state: PhaseState::Completed, tokens_used: Some(0), duration_ms: Some(decide_ms) },
        PhaseStatus { phase: CognitivePhase::Act, state: PhaseState::Completed, tokens_used: Some(0), duration_ms: Some(act_ms) },
        PhaseStatus { phase: CognitivePhase::Learn, state: PhaseState::Running, tokens_used: None, duration_ms: None },
    ]));

    // ═══════════════════════════════════════════════════════════
    // PHASE 5: LEARN — Store, revise beliefs, crystallize
    // ═══════════════════════════════════════════════════════════
    let _ = tx.send(CognitiveUpdate::Phase("Learn".into()));
    let learn_start = Instant::now();

    let user_text = config.history.last().map(|(_, c)| c.clone()).unwrap_or_default();
    if let Some(ref sh) = sisters_handle {
        if llm_result.is_ok() {
            sh.learn(&user_text, &final_response).await;

            // Planning: update goal progress from this interaction
            sh.learn_planning(&user_text, &final_response[..final_response.len().min(200)]).await;

            // Comm: share significant learnings with peers
            sh.learn_comm_share(&format!("Completed: {}", &user_text[..user_text.len().min(100)])).await;
        }
    }

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

        // Sprint 4: Store interaction in temporal memory
        inv.store_temporal(text, "user_interaction", if success { 0.7 } else { 0.3 });

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
        content: final_response,
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

/// Detect whether user input is an action request that should be executed, not narrated.
/// E.g. "install it", "run it", "start the server", "deploy it", "do it", "go ahead"
fn is_action_intent(text: &str) -> bool {
    let lower = text.to_lowercase().trim().to_string();
    let action_phrases = [
        "install", "run it", "start it", "deploy it", "build it",
        "test it", "execute", "launch it", "compile it", "do it",
        "go ahead", "make it", "set it up", "run the", "start the",
        "install and", "npm install", "npm start", "npm run",
        "cargo run", "cargo build", "pip install", "yarn install",
        "now install", "now run", "now start", "now build",
        "please install", "please run", "please start",
        "can you install", "can you run", "can you start",
        "i want you to", "just do it", "just run", "just install",
    ];
    action_phrases.iter().any(|p| lower.contains(p))
}

/// Detect whether user input is a settings mutation intent (Step 4.9).
fn is_settings_intent(text: &str) -> bool {
    let lower = text.to_lowercase();
    let settings_patterns = [
        "be more creative",
        "be less creative",
        "use openai",
        "use anthropic",
        "use claude",
        "use gpt",
        "use gemini",
        "use ollama",
        "remember my",
        "set timeout",
        "set max tokens",
        "enable dream",
        "disable dream",
        "enable proactive",
        "disable proactive",
        "enable cache",
        "disable cache",
        "enable belief",
        "disable belief",
        "set compression",
        "set routing",
        "set cache ttl",
    ];
    settings_patterns.iter().any(|p| lower.contains(p))
}

/// Execute a JSON plan (create dirs, files, run commands) and return metrics summary.
async fn execute_json_plan(
    plan: &serde_json::Value,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
    undo_stack: &Option<Arc<parking_lot::Mutex<UndoStack>>>,
) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let project_dir_name = plan["project_dir"].as_str().unwrap_or("hydra-project");
    let base_dir = format!("{}/projects/{}", home, project_dir_name);
    let _ = tokio::fs::create_dir_all(&base_dir).await;

    let steps = plan["steps"].as_array();
    let total_steps = steps.map(|s| s.len()).unwrap_or(0);

    let mut files_created: Vec<(String, usize, u64)> = Vec::new();
    let mut dirs_created = 0u32;
    let mut commands_run: Vec<(String, bool)> = Vec::new();
    let mut total_lines = 0usize;
    let mut total_bytes = 0u64;
    let mut languages: std::collections::HashMap<String, (u32, usize)> = std::collections::HashMap::new();

    if let Some(steps) = steps {
        for (i, step) in steps.iter().enumerate() {
            let step_type = step["type"].as_str().unwrap_or("");

            match step_type {
                "create_dir" => {
                    let path = step["path"].as_str().unwrap_or("");
                    let full_path = format!("{}/{}", base_dir, path);
                    let _ = tokio::fs::create_dir_all(&full_path).await;
                    dirs_created += 1;
                }
                "create_file" | "modify_file" => {
                    let path = step["path"].as_str().unwrap_or("");
                    let content = step["content"].as_str().unwrap_or("");
                    let full_path = format!("{}/{}", base_dir, path);
                    if let Some(parent) = std::path::Path::new(&full_path).parent() {
                        let _ = tokio::fs::create_dir_all(parent).await;
                    }
                    let _ = tokio::fs::write(&full_path, content).await;

                    // Track file creation in undo stack
                    if let Some(undo) = undo_stack {
                        let action = FileCreateAction::new(&full_path, content.as_bytes().to_vec());
                        undo.lock().push(Box::new(action));
                        let stack = undo.lock();
                        let _ = tx.send(CognitiveUpdate::UndoStatus {
                            can_undo: stack.can_undo(),
                            can_redo: stack.can_redo(),
                            last_action: stack.last_action_description().map(String::from),
                        });
                    }

                    let line_count = content.lines().count();
                    let byte_count = content.len() as u64;
                    total_lines += line_count;
                    total_bytes += byte_count;
                    files_created.push((path.to_string(), line_count, byte_count));

                    let lang = detect_language(path);
                    let entry = languages.entry(lang.to_string()).or_insert((0, 0));
                    entry.0 += 1;
                    entry.1 += line_count;

                    let _ = tx.send(CognitiveUpdate::EvidenceCode {
                        title: format!("{} ({} lines, {})", path, line_count, format_bytes(byte_count)),
                        content: content[..content.len().min(500)].to_string(),
                        language: Some(lang.to_string()),
                        file_path: Some(path.to_string()),
                    });
                }
                "run_command" => {
                    let cmd = step["command"].as_str().unwrap_or("");
                    let cwd = step["cwd"].as_str().unwrap_or(".");
                    let work_dir = if cwd == "." { base_dir.clone() } else { format!("{}/{}", base_dir, cwd) };

                    let output = tokio::process::Command::new("sh")
                        .arg("-c")
                        .arg(cmd)
                        .current_dir(&work_dir)
                        .output()
                        .await;

                    let success = output.as_ref().map(|o| o.status.success()).unwrap_or(false);
                    commands_run.push((cmd.to_string(), success));

                    if let Ok(out) = output {
                        let stdout = String::from_utf8_lossy(&out.stdout);
                        let stderr = String::from_utf8_lossy(&out.stderr);
                        let display = if !stdout.is_empty() { stdout.to_string() } else { stderr.to_string() };
                        if !display.is_empty() {
                            let _ = tx.send(CognitiveUpdate::EvidenceCode {
                                title: format!("$ {} {}", cmd, if success { "✓" } else { "✗" }),
                                content: display[..display.len().min(300)].to_string(),
                                language: Some("bash".to_string()),
                                file_path: None,
                            });
                        }
                    }
                }
                _ => {}
            }

            // Report plan step progress
            if total_steps > 0 {
                let _ = tx.send(CognitiveUpdate::PlanStepComplete { index: i, duration_ms: None });
                if i + 1 < total_steps {
                    let _ = tx.send(CognitiveUpdate::PlanStepStart(i + 1));
                }
            }
        }
    }

    // Build rich metrics response
    let mut lang_list: Vec<_> = languages.iter().collect();
    lang_list.sort_by(|a, b| b.1 .1.cmp(&a.1 .1));

    let completion_msg = plan["completion_message"].as_str().unwrap_or("");
    let summary = plan["summary"].as_str().unwrap_or("Project created");
    let commands_ok = commands_run.iter().filter(|(_, s)| *s).count();

    let mut metrics = format!(
        "## {}\n\n\
         ### Project Metrics\n\
         | Metric | Value |\n\
         |--------|-------|\n\
         | Location | `~/projects/{}` |\n\
         | Files created | **{}** |\n\
         | Directories | **{}** |\n\
         | Total lines of code | **{}** |\n\
         | Total size | **{}** |\n\
         | Commands executed | **{}/{}** passed |\n\n",
        summary, project_dir_name,
        files_created.len(), dirs_created,
        total_lines, format_bytes(total_bytes),
        commands_ok, commands_run.len(),
    );

    if !lang_list.is_empty() {
        metrics.push_str("### Languages\n| Language | Files | Lines |\n|----------|-------|-------|\n");
        for (lang, (count, lines)) in &lang_list {
            metrics.push_str(&format!("| {} | {} | {} |\n", lang, count, lines));
        }
        metrics.push('\n');
    }

    metrics.push_str("### Files\n| File | Lines | Size |\n|------|-------|------|\n");
    for (path, lines, bytes) in &files_created {
        metrics.push_str(&format!("| `{}` | {} | {} |\n", path, lines, format_bytes(*bytes)));
    }
    metrics.push('\n');

    if !commands_run.is_empty() {
        metrics.push_str("### Commands\n");
        for (cmd, success) in &commands_run {
            metrics.push_str(&format!("- `{}` {}\n", cmd, if *success { "✓" } else { "✗" }));
        }
        metrics.push('\n');
    }

    if !completion_msg.is_empty() {
        metrics.push_str(&format!("### Getting Started\n{}\n", completion_msg));
    }

    metrics
}

// ═══════════════════════════════════════════════════════════════════
// Multi-pass deepening system
// ═══════════════════════════════════════════════════════════════════

/// Result of a deepening pass.
struct DeepenResult {
    modules_deepened: usize,
    files_expanded: usize,
    total_lines: usize,
    total_bytes: u64,
}

/// Scan all files under `base_dir` and return (relative_path, line_count, byte_count).
async fn scan_project_files(base_dir: &str) -> Vec<(String, usize, u64)> {
    let mut files = Vec::new();
    let base = std::path::Path::new(base_dir);
    if !base.is_dir() {
        return files;
    }
    let mut stack = vec![base.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let mut entries = match tokio::fs::read_dir(&dir).await {
            Ok(e) => e,
            Err(_) => continue,
        };
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.is_dir() {
                // Skip hidden dirs and node_modules
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                if !name.starts_with('.') && name != "node_modules" && name != "target" {
                    stack.push(path);
                }
            } else if path.is_file() {
                let rel = path.strip_prefix(base)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                if rel.is_empty() {
                    continue;
                }
                if let Ok(content) = tokio::fs::read_to_string(&path).await {
                    let line_count = content.lines().count();
                    let byte_count = content.len() as u64;
                    files.push((rel, line_count, byte_count));
                }
            }
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

/// Check if a file is a source file that should be deepened (not config/data files).
fn is_deepenable_source(path: &str) -> bool {
    let ext = path.rsplit('.').next().unwrap_or("");
    matches!(ext,
        "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "java" | "kt" |
        "swift" | "c" | "cpp" | "h" | "hpp" | "cs" | "rb" | "php" | "vue" |
        "svelte" | "dart" | "zig" | "ex" | "exs" | "lua" | "scala"
    )
}

/// Group files by their first directory segment (module).
fn group_by_module(files: &[(String, usize, u64)]) -> std::collections::HashMap<String, Vec<(String, usize, u64)>> {
    let mut groups: std::collections::HashMap<String, Vec<(String, usize, u64)>> = std::collections::HashMap::new();
    for (path, lines, bytes) in files {
        if !is_deepenable_source(path) {
            continue;
        }
        let module = if let Some(idx) = path.find('/') {
            let first = &path[..idx];
            // Use two levels if first is "src" or "lib"
            if (first == "src" || first == "lib" || first == "app") && path[idx + 1..].contains('/') {
                let rest = &path[idx + 1..];
                if let Some(idx2) = rest.find('/') {
                    format!("{}/{}", first, &rest[..idx2])
                } else {
                    first.to_string()
                }
            } else {
                first.to_string()
            }
        } else {
            "root".to_string()
        };
        groups.entry(module).or_default().push((path.clone(), *lines, *bytes));
    }
    groups
}

/// Build a deepening prompt for a specific module group.
fn build_deepen_prompt(project_summary: &str, module: &str, files: &[(String, usize, u64)]) -> String {
    let mut file_listing = String::new();
    for (path, lines, _) in files {
        file_listing.push_str(&format!("- {} ({} lines)\n", path, lines));
    }
    format!(
        "You are expanding shallow stub files into full, production-quality implementations.\n\n\
         Project: {}\n\
         Module: {}\n\n\
         These files were generated as stubs and need to be fully implemented:\n{}\n\
         For EACH file listed above, output a complete, production-ready implementation.\n\
         Use real logic, proper error handling, documentation, and tests where appropriate.\n\
         Do NOT output placeholder comments like \"// TODO\" or \"// implement here\".\n\n\
         Output format — for each file, use exactly this format:\n\
         === FILE: <relative_path> ===\n\
         <full file content>\n\
         === END FILE ===\n\n\
         Expand ALL files listed above. Make them substantial and correct.",
        project_summary, module, file_listing
    )
}

/// Parse the LLM deepening response into file path -> content pairs.
fn parse_deepen_response(response: &str) -> Vec<(String, String)> {
    let mut results = Vec::new();
    let mut remaining = response;
    while let Some(start_marker) = remaining.find("=== FILE: ") {
        let after_marker = &remaining[start_marker + 10..];
        let line_end = after_marker.find(" ===").or_else(|| after_marker.find('\n'));
        if let Some(end) = line_end {
            let path = after_marker[..end].trim().to_string();
            let content_start = after_marker[end..].find('\n').map(|i| end + i + 1).unwrap_or(end);
            let after_path = &after_marker[content_start..];
            let content_end = after_path.find("=== END FILE ===").unwrap_or(after_path.len());
            let content = after_path[..content_end].trim_end().to_string();
            if !path.is_empty() && !content.is_empty() {
                results.push((path, content));
            }
            remaining = &after_path[content_end..];
        } else {
            break;
        }
    }
    results
}

/// Call the LLM provider and return the response content.
async fn call_llm_for_deepening(
    prompt: &str,
    llm_config: &hydra_model::LlmConfig,
    provider: &str,
    model: &str,
) -> Result<String, String> {
    let request = hydra_model::CompletionRequest {
        model: model.to_string(),
        messages: vec![hydra_model::providers::Message {
            role: "user".to_string(),
            content: prompt.to_string(),
        }],
        max_tokens: {
            // Use actual model limits for deepening calls
            match model {
                m if m.contains("opus") => 32_768,
                m if m.contains("sonnet") => 16_384,
                m if m.contains("haiku") => 8_192,
                m if m.contains("gpt-4o") => 16_384,
                m if m.contains("gpt-4") => 8_192,
                m if m.contains("ollama") | m.contains("llama") | m.contains("phi") | m.contains("mistral") => 4_096,
                _ => 16_384,
            }
        },
        temperature: Some(0.2),
        system: Some("You are a senior software engineer. Expand stub files into full implementations. Output ONLY the file contents in the specified format.".to_string()),
    };

    match provider {
        "anthropic" => {
            let client = hydra_model::providers::anthropic::AnthropicClient::new(llm_config)
                .map_err(|e| format!("{}", e))?;
            client.complete(request).await
                .map(|r| r.content)
                .map_err(|e| format!("{}", e))
        }
        "openai" | "google" => {
            let client = hydra_model::providers::openai::OpenAiClient::new(llm_config)
                .map_err(|e| format!("{}", e))?;
            client.complete(request).await
                .map(|r| r.content)
                .map_err(|e| format!("{}", e))
        }
        "ollama" => {
            let mut ollama_config = llm_config.clone();
            ollama_config.openai_api_key = Some("ollama".into());
            ollama_config.openai_base_url = "http://localhost:11434".into();
            let client = hydra_model::providers::openai::OpenAiClient::new(&ollama_config)
                .map_err(|e| format!("{}", e))?;
            client.complete(request).await
                .map(|r| r.content)
                .map_err(|e| format!("{}", e))
        }
        _ => Err("Unsupported provider".into()),
    }
}

/// Multi-pass deepening: if average lines per source file < 25, expand modules iteratively.
///
/// Scans the project on disk, groups shallow files by module, and makes targeted LLM calls
/// to replace stub files with full implementations.
async fn maybe_deepen_project(
    base_dir: &str,
    project_summary: &str,
    llm_config: &hydra_model::LlmConfig,
    provider: &str,
    model: &str,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> Option<DeepenResult> {
    let files = scan_project_files(base_dir).await;
    if files.is_empty() {
        return None;
    }

    // Only consider source files for shallowness check
    let source_files: Vec<_> = files.iter()
        .filter(|(p, _, _)| is_deepenable_source(p))
        .collect();

    if source_files.is_empty() {
        return None;
    }

    let total_source_lines: usize = source_files.iter().map(|(_, l, _)| l).sum();
    let avg_lines = total_source_lines / source_files.len();

    // Threshold: if average source file has >= 25 lines, no deepening needed
    if avg_lines >= 25 {
        return None;
    }

    let _ = tx.send(CognitiveUpdate::Phase("Deepening project...".into()));

    let modules = group_by_module(&files);
    let mut modules_deepened = 0usize;
    let mut files_expanded = 0usize;

    for (module_name, module_files) in &modules {
        // Only deepen modules where average is shallow
        let module_avg: usize = module_files.iter().map(|(_, l, _)| l).sum::<usize>()
            / module_files.len().max(1);
        if module_avg >= 25 {
            continue;
        }

        let display_module = if module_name == "root" {
            "root files".to_string()
        } else {
            format!("{} module", module_name)
        };
        let _ = tx.send(CognitiveUpdate::Phase(format!("Deepening {}...", display_module)));

        let prompt = build_deepen_prompt(project_summary, module_name, module_files);

        match call_llm_for_deepening(&prompt, llm_config, provider, model).await {
            Ok(response) => {
                let expanded = parse_deepen_response(&response);
                for (rel_path, content) in &expanded {
                    let full_path = format!("{}/{}", base_dir, rel_path);
                    if let Some(parent) = std::path::Path::new(&full_path).parent() {
                        let _ = tokio::fs::create_dir_all(parent).await;
                    }
                    let _ = tokio::fs::write(&full_path, content).await;
                    files_expanded += 1;

                    let line_count = content.lines().count();
                    let byte_count = content.len() as u64;
                    let lang = detect_language(rel_path);
                    let _ = tx.send(CognitiveUpdate::EvidenceCode {
                        title: format!("Deepened: {} ({} lines, {})", rel_path, line_count, format_bytes(byte_count)),
                        content: content[..content.len().min(500)].to_string(),
                        language: Some(lang.to_string()),
                        file_path: Some(rel_path.to_string()),
                    });
                }
                modules_deepened += 1;
            }
            Err(err) => {
                let _ = tx.send(CognitiveUpdate::EvidenceCode {
                    title: format!("Deepening {} failed", display_module),
                    content: err,
                    language: None,
                    file_path: None,
                });
            }
        }
    }

    if modules_deepened == 0 {
        return None;
    }

    // Re-scan to get final totals
    let final_files = scan_project_files(base_dir).await;
    let total_lines: usize = final_files.iter().map(|(_, l, _)| l).sum();
    let total_bytes: u64 = final_files.iter().map(|(_, _, b)| b).sum();

    Some(DeepenResult {
        modules_deepened,
        files_expanded,
        total_lines,
        total_bytes,
    })
}

// ═══════════════════════════════════════════════════════════
// Inline command execution — <hydra-exec> tag support
// ═══════════════════════════════════════════════════════════

/// Extract command strings from <hydra-exec>...</hydra-exec> tags (without executing).
fn extract_inline_commands(text: &str) -> Vec<String> {
    let mut commands = Vec::new();
    let mut remaining = text;
    while let Some(start) = remaining.find("<hydra-exec>") {
        let after = &remaining[start + 12..];
        if let Some(end) = after.find("</hydra-exec>") {
            let cmd = after[..end].trim().to_string();
            if !cmd.is_empty() {
                commands.push(cmd);
            }
            remaining = &after[end + 13..];
        } else {
            break;
        }
    }
    commands
}

/// Extract a URL from a command string (for Vision capture).
fn extract_url_from_command(cmd: &str) -> Option<String> {
    for word in cmd.split_whitespace() {
        if word.starts_with("http://") || word.starts_with("https://") {
            // Strip quotes
            let url = word.trim_matches(|c| c == '\'' || c == '"');
            return Some(url.to_string());
        }
    }
    None
}

/// Universal action executor — detects user intent and returns the appropriate shell command.
/// Works across macOS, Linux, and Windows. No hardcoded app list — resolves ANY app by name.
fn detect_direct_action_command(text: &str) -> Option<String> {
    let lower = text.to_lowercase();

    // ── Special case: Terminal (needs new window, not just focus) ──
    if (lower.contains("open") && lower.contains("terminal"))
        || lower.contains("new terminal")
        || lower.contains("fresh terminal")
        || (lower.contains("continue") && lower.contains("terminal"))
    {
        return Some(platform_new_terminal());
    }

    // ── Special case: New browser tab ──
    if lower.contains("new tab") || (lower.contains("open") && lower.contains("tab")) {
        let browser = extract_browser_name(&lower);
        return Some(platform_new_tab(&browser));
    }

    // ── URL detection: "open google.com" / "open https://..." / "go to example.com" ──
    if let Some(url) = extract_url_intent(&lower, text) {
        return Some(platform_open_url(&url));
    }

    // ── Scroll / navigate within an app ──
    if lower.contains("scroll") {
        let direction = if lower.contains("down") { "down" } else if lower.contains("up") { "up" } else { "down" };
        let amount = if lower.contains("bottom") || lower.contains("end") { "max" } else { "page" };
        return Some(platform_scroll(direction, amount));
    }

    // ── Type / input text into focused app ──
    if lower.starts_with("type ") || lower.starts_with("enter ") {
        let content = if lower.starts_with("type ") { &text[5..] } else { &text[6..] };
        return Some(platform_type_text(content.trim()));
    }

    // ── Screenshot ──
    if lower.contains("screenshot") || lower.contains("screen capture") || lower.contains("screen shot") {
        return Some(platform_screenshot());
    }

    // ── System info ──
    if lower.contains("system info") || lower.contains("system information")
        || lower.contains("what os") || lower.contains("what system")
    {
        return Some(platform_system_info());
    }

    // ── Kill / close / quit an app ──
    if (lower.contains("close") || lower.contains("quit") || lower.contains("kill"))
        && !lower.contains("close the door") && !lower.contains("kill the")
    {
        if let Some(app) = extract_app_name_from_intent(&lower, &["close", "quit", "kill"]) {
            return Some(platform_close_app(&app));
        }
    }

    // ── Minimize / hide ──
    if lower.contains("minimize") || lower.contains("hide") {
        if let Some(app) = extract_app_name_from_intent(&lower, &["minimize", "hide"]) {
            return Some(platform_minimize_app(&app));
        }
    }

    // ── Universal "open X" — resolves ANY app by name ──
    // This MUST be last since it's the most generic matcher
    if lower.starts_with("open ") || lower.starts_with("launch ") || lower.starts_with("start ") {
        let verb_len = if lower.starts_with("launch ") { 7 } else if lower.starts_with("start ") { 6 } else { 5 };
        let raw_target = text[verb_len..].trim();
        // Strip articles: "open the calculator" → "calculator"
        let target = strip_articles(raw_target);

        if !target.is_empty() {
            return Some(platform_open_app(&target));
        }
    }

    None
}

// ═══════════════════════════════════════════════════════════
// Platform abstraction layer — one function per action type
// ═══════════════════════════════════════════════════════════

fn platform_new_terminal() -> String {
    if cfg!(target_os = "macos") {
        "osascript -e 'tell application \"Terminal\" to do script \"\"' -e 'tell application \"Terminal\" to activate'".into()
    } else if cfg!(target_os = "windows") {
        "start cmd".into()
    } else {
        "gnome-terminal 2>/dev/null || konsole 2>/dev/null || xfce4-terminal 2>/dev/null || xterm 2>/dev/null".into()
    }
}

fn platform_new_tab(browser: &str) -> String {
    if cfg!(target_os = "macos") {
        match browser {
            "firefox" => "open -a Firefox 'about:blank'".into(),
            "safari" => "osascript -e 'tell application \"Safari\" to activate' -e 'tell application \"System Events\" to keystroke \"t\" using command down'".into(),
            _ => "open -a 'Google Chrome' 'about:blank'".into(),
        }
    } else if cfg!(target_os = "windows") {
        format!("start {} about:blank", if browser == "firefox" { "firefox" } else { "chrome" })
    } else {
        format!("{} 'about:blank' 2>/dev/null", if browser == "firefox" { "firefox" } else { "google-chrome" })
    }
}

fn platform_open_url(url: &str) -> String {
    if cfg!(target_os = "macos") {
        format!("open '{}'", url)
    } else if cfg!(target_os = "windows") {
        format!("start '{}'", url)
    } else {
        format!("xdg-open '{}' 2>/dev/null", url)
    }
}

fn platform_open_app(name: &str) -> String {
    // Resolve common aliases to their real app names
    let resolved = resolve_app_alias(name);

    if cfg!(target_os = "macos") {
        // macOS: `open -a "Name"` works for ANY installed .app
        // For CLI tools (code, docker), try the binary first
        if is_cli_tool(&resolved) {
            format!("{} 2>/dev/null || open -a '{}' 2>/dev/null", resolved, title_case(&resolved))
        } else {
            format!("open -a '{}' 2>/dev/null || open -a '{}' 2>/dev/null", title_case(&resolved), resolved)
        }
    } else if cfg!(target_os = "windows") {
        // Windows: `start` for known apps, or search Program Files
        format!("start \"\" \"{}\" 2>nul || where {} 2>nul && {} || echo App not found: {}", resolved, resolved, resolved, resolved)
    } else {
        // Linux: try lowercase binary name, then flatpak, then snap
        let bin = resolved.to_lowercase().replace(' ', "-");
        format!(
            "{bin} 2>/dev/null || flatpak run $(flatpak list --app | grep -i '{name}' | head -1 | awk '{{print $2}}') 2>/dev/null || snap run {bin} 2>/dev/null || echo 'App not found: {name}'",
            bin = bin,
            name = resolved,
        )
    }
}

fn platform_close_app(name: &str) -> String {
    let resolved = resolve_app_alias(name);
    if cfg!(target_os = "macos") {
        format!("osascript -e 'tell application \"{}\" to quit'", title_case(&resolved))
    } else if cfg!(target_os = "windows") {
        format!("taskkill /IM \"{}.exe\" /F 2>nul", resolved)
    } else {
        format!("pkill -f '{}' 2>/dev/null || killall '{}' 2>/dev/null", resolved, resolved)
    }
}

fn platform_minimize_app(name: &str) -> String {
    let resolved = resolve_app_alias(name);
    if cfg!(target_os = "macos") {
        format!("osascript -e 'tell application \"System Events\" to set visible of process \"{}\" to false'", title_case(&resolved))
    } else {
        format!("xdotool search --name '{}' windowminimize 2>/dev/null", resolved)
    }
}

fn platform_scroll(direction: &str, amount: &str) -> String {
    if cfg!(target_os = "macos") {
        let pixels = if amount == "max" { "9999" } else { "400" };
        let sign = if direction == "up" { "" } else { "-" };
        format!("osascript -e 'tell application \"System Events\" to scroll area 1 of (first process whose frontmost is true) by {{0, {}{}}}'", sign, pixels)
    } else {
        let button = if direction == "up" { "4" } else { "5" };
        let clicks = if amount == "max" { "50" } else { "5" };
        format!("xdotool click --repeat {} {} 2>/dev/null", clicks, button)
    }
}

fn platform_type_text(content: &str) -> String {
    let escaped = content.replace('\'', "'\\''");
    if cfg!(target_os = "macos") {
        format!("osascript -e 'tell application \"System Events\" to keystroke \"{}\"'", escaped)
    } else {
        format!("xdotool type '{}' 2>/dev/null", escaped)
    }
}

fn platform_screenshot() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let path = format!("{}/Desktop/screenshot_{}.png", home, timestamp);
    if cfg!(target_os = "macos") {
        format!("screencapture -x '{}'", path)
    } else if cfg!(target_os = "windows") {
        "snippingtool /clip".into()
    } else {
        format!("gnome-screenshot -f '{}' 2>/dev/null || scrot '{}' 2>/dev/null", path, path)
    }
}

fn platform_system_info() -> String {
    if cfg!(target_os = "macos") {
        "echo '=== System ===' && sw_vers && echo && echo '=== Hardware ===' && sysctl -n machdep.cpu.brand_string && echo && echo '=== Memory ===' && sysctl -n hw.memsize | awk '{printf \"%.0f GB\\n\", $1/1073741824}' && echo && echo '=== Disk ===' && df -h / | tail -1".into()
    } else if cfg!(target_os = "windows") {
        "systeminfo".into()
    } else {
        "echo '=== System ===' && uname -a && echo && cat /etc/os-release 2>/dev/null && echo && echo '=== CPU ===' && lscpu | head -5 && echo && echo '=== Memory ===' && free -h | head -2 && echo && echo '=== Disk ===' && df -h / | tail -1".into()
    }
}

// ═══════════════════════════════════════════════════════════
// Helper utilities
// ═══════════════════════════════════════════════════════════

/// Resolve common app aliases to their real names
fn resolve_app_alias(name: &str) -> String {
    let lower = name.to_lowercase();
    match lower.as_str() {
        "chrome" | "google chrome" => "Google Chrome".into(),
        "vscode" | "vs code" | "code" => "Visual Studio Code".into(),
        "iterm" | "iterm2" => "iTerm".into(),
        "postman" => "Postman".into(),
        "browser" => "Google Chrome".into(),
        "mail" | "email" => if cfg!(target_os = "macos") { "Mail".into() } else { "thunderbird".into() },
        "files" | "file manager" => if cfg!(target_os = "macos") { "Finder".into() } else { "nautilus".into() },
        "settings" | "preferences" | "system preferences" => {
            if cfg!(target_os = "macos") { "System Settings".into() } else { "gnome-control-center".into() }
        }
        "activity monitor" | "task manager" => {
            if cfg!(target_os = "macos") { "Activity Monitor".into() } else { "gnome-system-monitor".into() }
        }
        "word" => "Microsoft Word".into(),
        "excel" => "Microsoft Excel".into(),
        "powerpoint" | "ppt" => "Microsoft PowerPoint".into(),
        "teams" => "Microsoft Teams".into(),
        "figma" => "Figma".into(),
        "notion" => "Notion".into(),
        "obs" | "obs studio" => "OBS".into(),
        "whatsapp" => "WhatsApp".into(),
        _ => name.to_string(),
    }
}

/// Check if this is a CLI tool rather than a GUI app
fn is_cli_tool(name: &str) -> bool {
    let cli_tools = ["code", "docker", "npm", "node", "python", "pip", "cargo", "git",
                     "brew", "htop", "vim", "nvim", "tmux", "kubectl", "terraform"];
    cli_tools.iter().any(|t| name.to_lowercase() == *t)
}

/// Convert "google chrome" → "Google Chrome"
fn title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(c) => format!("{}{}", c.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Strip articles: "the calculator" → "calculator", "a terminal" → "terminal"
fn strip_articles(s: &str) -> String {
    let lower = s.to_lowercase();
    for prefix in &["the ", "a ", "an ", "my ", "that "] {
        if lower.starts_with(prefix) {
            return s[prefix.len()..].to_string();
        }
    }
    s.to_string()
}

/// Extract browser name from text
fn extract_browser_name(lower: &str) -> String {
    if lower.contains("firefox") { "firefox".into() }
    else if lower.contains("safari") { "safari".into() }
    else { "chrome".into() }
}

/// Extract URL from "open google.com" / "go to https://example.com"
fn extract_url_intent(lower: &str, original: &str) -> Option<String> {
    // Match "open X.com", "go to X.com", "visit X.com"
    for prefix in &["open ", "go to ", "visit ", "navigate to ", "browse "] {
        if lower.starts_with(prefix) {
            let rest = original[prefix.len()..].trim();
            let rest_lower = rest.to_lowercase();
            // Strip articles
            let target = strip_articles(&rest_lower);
            if target.starts_with("http://") || target.starts_with("https://")
                || target.contains(".com") || target.contains(".org") || target.contains(".io")
                || target.contains(".dev") || target.contains(".net") || target.contains(".co")
                || target.contains(".app") || target.contains(".me")
            {
                return if target.starts_with("http") {
                    Some(target)
                } else {
                    Some(format!("https://{}", target))
                };
            }
        }
    }
    None
}

/// Extract the app name from a verb+app intent like "close chrome" or "quit spotify"
fn extract_app_name_from_intent(lower: &str, verbs: &[&str]) -> Option<String> {
    for verb in verbs {
        if let Some(pos) = lower.find(verb) {
            let after = lower[pos + verb.len()..].trim();
            let app = strip_articles(after);
            if !app.is_empty() && app.len() > 1 {
                return Some(app);
            }
        }
    }
    None
}

// ═══════════════════════════════════════════════════════════
// Universal system control — volume, brightness, wifi, bluetooth, etc.
// ═══════════════════════════════════════════════════════════

/// Detect system-level control intents (volume, brightness, wifi, power, etc.)
fn detect_system_control(text: &str) -> Option<String> {
    let lower = text.to_lowercase();

    // ── Volume ──
    if lower.contains("volume") || lower.contains("sound") {
        if lower.contains("mute") || lower.contains("silent") {
            return Some(platform_volume("mute"));
        } else if lower.contains("up") || lower.contains("increase") || lower.contains("louder") {
            return Some(platform_volume("up"));
        } else if lower.contains("down") || lower.contains("decrease") || lower.contains("lower") || lower.contains("quieter") {
            return Some(platform_volume("down"));
        } else if lower.contains("max") || lower.contains("full") {
            return Some(platform_volume("max"));
        }
    }

    // ── Brightness ──
    if lower.contains("brightness") || lower.contains("screen bright") {
        if lower.contains("up") || lower.contains("increase") || lower.contains("brighter") {
            return Some(platform_brightness("up"));
        } else if lower.contains("down") || lower.contains("decrease") || lower.contains("dim") {
            return Some(platform_brightness("down"));
        }
    }

    // ── WiFi ──
    if lower.contains("wifi") || lower.contains("wi-fi") {
        if lower.contains("off") || lower.contains("disable") || lower.contains("disconnect") {
            return Some(platform_wifi(false));
        } else if lower.contains("on") || lower.contains("enable") || lower.contains("connect") {
            return Some(platform_wifi(true));
        } else if lower.contains("status") || lower.contains("check") {
            return Some(platform_wifi_status());
        }
    }

    // ── Bluetooth ──
    if lower.contains("bluetooth") {
        if lower.contains("off") || lower.contains("disable") {
            return Some(platform_bluetooth(false));
        } else if lower.contains("on") || lower.contains("enable") {
            return Some(platform_bluetooth(true));
        }
    }

    // ── Dark / Light mode ──
    if lower.contains("dark mode") {
        if lower.contains("on") || lower.contains("enable") || lower.contains("switch to") || lower.contains("turn on") {
            return Some(platform_dark_mode(true));
        } else if lower.contains("off") || lower.contains("disable") || lower.contains("turn off") {
            return Some(platform_dark_mode(false));
        }
    }
    if lower.contains("light mode") {
        return Some(platform_dark_mode(false));
    }

    // ── Sleep / Lock / Shutdown ──
    if lower.contains("lock") && (lower.contains("screen") || lower.contains("computer") || lower.contains("mac") || lower.contains("pc")) {
        return Some(platform_lock_screen());
    }
    if (lower.contains("sleep") || lower.contains("standby")) && (lower.contains("computer") || lower.contains("mac") || lower.contains("pc") || lower.contains("system")) {
        return Some(platform_sleep());
    }

    // ── Do Not Disturb ──
    if lower.contains("do not disturb") || lower.contains("dnd") || lower.contains("focus mode") {
        if lower.contains("off") || lower.contains("disable") {
            return Some(platform_dnd(false));
        } else {
            return Some(platform_dnd(true));
        }
    }

    // ── Empty trash ──
    if lower.contains("empty") && lower.contains("trash") {
        return Some(platform_empty_trash());
    }

    // ── Battery ──
    if lower.contains("battery") && (lower.contains("status") || lower.contains("level") || lower.contains("check") || lower.contains("how much")) {
        return Some(platform_battery_status());
    }

    // ── IP address / network ──
    if lower.contains("ip address") || lower.contains("my ip") || (lower.contains("what") && lower.contains("ip")) {
        return Some(platform_ip_address());
    }

    // ── Disk space ──
    if lower.contains("disk space") || lower.contains("storage") || lower.contains("how much space") {
        return Some(platform_disk_space());
    }

    // ── List running processes ──
    if lower.contains("running") && (lower.contains("process") || lower.contains("app")) {
        return Some(platform_running_processes());
    }

    // ── List installed apps ──
    if lower.contains("installed") && (lower.contains("app") || lower.contains("program") || lower.contains("software")) {
        return Some(platform_list_installed_apps());
    }

    None
}

fn platform_volume(action: &str) -> String {
    if cfg!(target_os = "macos") {
        match action {
            "mute" => "osascript -e 'set volume with output muted'".into(),
            "up" => "osascript -e 'set volume output volume ((output volume of (get volume settings)) + 15)'".into(),
            "down" => "osascript -e 'set volume output volume ((output volume of (get volume settings)) - 15)'".into(),
            "max" => "osascript -e 'set volume output volume 100'".into(),
            _ => "osascript -e 'get volume settings'".into(),
        }
    } else {
        match action {
            "mute" => "amixer sset Master toggle 2>/dev/null || pactl set-sink-mute @DEFAULT_SINK@ toggle 2>/dev/null".into(),
            "up" => "amixer sset Master 10%+ 2>/dev/null || pactl set-sink-volume @DEFAULT_SINK@ +10% 2>/dev/null".into(),
            "down" => "amixer sset Master 10%- 2>/dev/null || pactl set-sink-volume @DEFAULT_SINK@ -10% 2>/dev/null".into(),
            "max" => "amixer sset Master 100% 2>/dev/null || pactl set-sink-volume @DEFAULT_SINK@ 100% 2>/dev/null".into(),
            _ => "amixer sget Master 2>/dev/null".into(),
        }
    }
}

fn platform_brightness(action: &str) -> String {
    if cfg!(target_os = "macos") {
        match action {
            "up" => "osascript -e 'tell application \"System Events\" to key code 144'".into(), // Brightness Up key
            "down" => "osascript -e 'tell application \"System Events\" to key code 145'".into(), // Brightness Down key
            _ => "echo 'Brightness adjusted'".into(),
        }
    } else {
        match action {
            "up" => "xbacklight -inc 15 2>/dev/null || brightnessctl set +15% 2>/dev/null".into(),
            "down" => "xbacklight -dec 15 2>/dev/null || brightnessctl set 15%- 2>/dev/null".into(),
            _ => "xbacklight -get 2>/dev/null || brightnessctl get 2>/dev/null".into(),
        }
    }
}

fn platform_wifi(enable: bool) -> String {
    if cfg!(target_os = "macos") {
        if enable {
            "networksetup -setairportpower en0 on".into()
        } else {
            "networksetup -setairportpower en0 off".into()
        }
    } else {
        if enable { "nmcli radio wifi on".into() } else { "nmcli radio wifi off".into() }
    }
}

fn platform_wifi_status() -> String {
    if cfg!(target_os = "macos") {
        "networksetup -getairportnetwork en0 && echo && networksetup -getinfo Wi-Fi | head -5".into()
    } else {
        "nmcli general status && echo && nmcli connection show --active".into()
    }
}

fn platform_bluetooth(enable: bool) -> String {
    if cfg!(target_os = "macos") {
        // Requires blueutil: brew install blueutil
        if enable { "blueutil --power 1 2>/dev/null || echo 'Install blueutil: brew install blueutil'".into() }
        else { "blueutil --power 0 2>/dev/null || echo 'Install blueutil: brew install blueutil'".into() }
    } else {
        if enable { "bluetoothctl power on".into() } else { "bluetoothctl power off".into() }
    }
}

fn platform_dark_mode(enable: bool) -> String {
    if cfg!(target_os = "macos") {
        if enable {
            "osascript -e 'tell application \"System Events\" to tell appearance preferences to set dark mode to true'".into()
        } else {
            "osascript -e 'tell application \"System Events\" to tell appearance preferences to set dark mode to false'".into()
        }
    } else {
        if enable {
            "gsettings set org.gnome.desktop.interface color-scheme 'prefer-dark' 2>/dev/null".into()
        } else {
            "gsettings set org.gnome.desktop.interface color-scheme 'prefer-light' 2>/dev/null".into()
        }
    }
}

fn platform_lock_screen() -> String {
    if cfg!(target_os = "macos") {
        "osascript -e 'tell application \"System Events\" to keystroke \"q\" using {control down, command down}'".into()
    } else if cfg!(target_os = "windows") {
        "rundll32.exe user32.dll,LockWorkStation".into()
    } else {
        "loginctl lock-session 2>/dev/null || xdg-screensaver lock 2>/dev/null".into()
    }
}

fn platform_sleep() -> String {
    if cfg!(target_os = "macos") {
        "pmset sleepnow".into()
    } else if cfg!(target_os = "windows") {
        "rundll32.exe powrprof.dll,SetSuspendState 0,1,0".into()
    } else {
        "systemctl suspend 2>/dev/null".into()
    }
}

fn platform_dnd(enable: bool) -> String {
    if cfg!(target_os = "macos") {
        if enable {
            "shortcuts run 'Turn On Focus' 2>/dev/null || echo 'DND enabled (use System Settings to configure)'".into()
        } else {
            "shortcuts run 'Turn Off Focus' 2>/dev/null || echo 'DND disabled'".into()
        }
    } else {
        "echo 'Do Not Disturb toggled'".into()
    }
}

fn platform_empty_trash() -> String {
    if cfg!(target_os = "macos") {
        "osascript -e 'tell application \"Finder\" to empty the trash'".into()
    } else {
        "rm -rf ~/.local/share/Trash/files/* ~/.local/share/Trash/info/* 2>/dev/null && echo 'Trash emptied'".into()
    }
}

fn platform_battery_status() -> String {
    if cfg!(target_os = "macos") {
        "pmset -g batt".into()
    } else if cfg!(target_os = "windows") {
        "WMIC Path Win32_Battery Get EstimatedChargeRemaining".into()
    } else {
        "upower -i /org/freedesktop/UPower/devices/battery_BAT0 2>/dev/null || cat /sys/class/power_supply/BAT0/capacity 2>/dev/null".into()
    }
}

fn platform_ip_address() -> String {
    if cfg!(target_os = "macos") {
        "echo 'Local:' && ipconfig getifaddr en0 2>/dev/null; echo && echo 'Public:' && curl -s ifconfig.me".into()
    } else {
        "echo 'Local:' && hostname -I 2>/dev/null | awk '{print $1}'; echo && echo 'Public:' && curl -s ifconfig.me".into()
    }
}

fn platform_disk_space() -> String {
    if cfg!(target_os = "macos") {
        "df -h / && echo && echo '=== Largest folders ===' && du -sh ~/Desktop ~/Documents ~/Downloads ~/Library 2>/dev/null | sort -rh | head -10".into()
    } else {
        "df -h / && echo && echo '=== Largest folders ===' && du -sh ~/* 2>/dev/null | sort -rh | head -10".into()
    }
}

fn platform_running_processes() -> String {
    if cfg!(target_os = "macos") {
        "ps aux --sort=-%mem | head -15".into()
    } else {
        "ps aux --sort=-%mem | head -15".into()
    }
}

fn platform_list_installed_apps() -> String {
    if cfg!(target_os = "macos") {
        "ls /Applications/ | sed 's/.app$//' | sort".into()
    } else {
        "dpkg --list 2>/dev/null | tail -20 || rpm -qa 2>/dev/null | head -20 || pacman -Q 2>/dev/null | head -20".into()
    }
}

/// Strip <hydra-exec>...</hydra-exec> tags from the response text for clean display.
fn strip_hydra_exec_tags(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut search_from = 0;

    loop {
        let open_tag = "<hydra-exec>";
        let close_tag = "</hydra-exec>";

        match text[search_from..].find(open_tag) {
            Some(pos) => {
                result.push_str(&text[search_from..search_from + pos]);
                let after_open = search_from + pos + open_tag.len();
                match text[after_open..].find(close_tag) {
                    Some(end_pos) => {
                        search_from = after_open + end_pos + close_tag.len();
                    }
                    None => {
                        result.push_str(&text[search_from + pos..]);
                        break;
                    }
                }
            }
            None => {
                result.push_str(&text[search_from..]);
                break;
            }
        }
    }

    result.trim().to_string()
}
