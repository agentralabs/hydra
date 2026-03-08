//! Cognitive loop runner — 5-phase (Perceive→Think→Decide→Act→Learn) decoupled from UI.
//!
//! Sends `CognitiveUpdate` messages via `tokio::sync::mpsc` so the UI can
//! dispatch to Dioxus signals without the loop knowing about the rendering layer.

use std::time::Instant;
use tokio::sync::mpsc;

use crate::sisters::SistersHandle;
use crate::state::hydra::{CognitivePhase, PhaseState, PhaseStatus};
use crate::utils::{detect_language, extract_json_plan, format_bytes, generate_deliverable_steps};

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

/// Run the 5-phase cognitive loop, sending updates via the channel.
pub async fn run_cognitive_loop(
    config: CognitiveLoopConfig,
    sisters_handle: Option<SistersHandle>,
    tx: mpsc::UnboundedSender<CognitiveUpdate>,
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
        sh.build_cognitive_prompt(&config.user_name, &perceived, is_complex)
    } else {
        format!(
            "You are Hydra, a cognitive AI orchestrator built by Agentra Labs. \
             {}Be helpful, concise, and conversational.",
            if config.user_name.is_empty() { String::new() } else { format!("The user's name is {}. ", config.user_name) }
        )
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
            max_tokens: if is_complex { 65536 } else { 4096 },
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
    if sisters_handle.is_some() {
        let mut called_sisters = vec!["Memory".to_string(), "Codebase".to_string()];
        if perceived["involves_vision"].as_bool().unwrap_or(false) {
            called_sisters.push("Vision".to_string());
        }
        if perceived["involves_code"].as_bool().unwrap_or(false) {
            called_sisters.push("Evolve".to_string());
        }
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
    // PHASE 3: DECIDE — Risk assessment and gating
    // ═══════════════════════════════════════════════════════════
    let _ = tx.send(CognitiveUpdate::Phase("Decide".into()));
    let _ = tx.send(CognitiveUpdate::IconState("needs-attention".into()));
    let decide_start = Instant::now();

    let gate_decision = match risk_level {
        "high" | "critical" => "requires_approval",
        "medium" => "shadow_first",
        _ => "approved",
    };

    // Step 3.7: Gate integration — if action requires approval, notify UI
    if gate_decision == "requires_approval" {
        let challenge = if risk_level == "critical" {
            // Generate challenge phrase for critical actions
            let words: Vec<&str> = text.split_whitespace().take(3).collect();
            Some(words.join(" ").to_lowercase())
        } else {
            None
        };
        let _ = tx.send(CognitiveUpdate::AwaitApproval {
            risk_level: risk_level.to_string(),
            action: text.clone(),
            description: format!("This action is classified as {} risk", risk_level),
            challenge_phrase: challenge,
        });
        let _ = tx.send(CognitiveUpdate::IconState("needs-attention".into()));
        // In production, this would await a response from the UI via a channel.
        // The approval manager in hydra-runtime handles the async request/response.
        // For now, we wait briefly to simulate the approval window.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
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
            final_response = execute_json_plan(plan, &tx).await;

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
        max_tokens: 65536,
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
