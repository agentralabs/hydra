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
use crate::sisters::{SistersHandle, connection::SisterConnection};
use crate::state::hydra::{CognitivePhase, PhaseState, PhaseStatus};
use crate::utils::{detect_language, extract_json_plan, format_bytes, generate_deliverable_steps};
use hydra_db::{HydraDb, BeliefRow, McpDiscoveredSkillRow, FederationStateRow};
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
    /// OAuth bearer token for Anthropic (from browser-based auth / Claude Max subscription).
    /// When set, this is preferred over anthropic_key for API calls.
    pub anthropic_oauth_token: Option<String>,
}

/// Extract cleaned facts from raw memory JSON.
///
/// Memory sister returns: `{"count": N, "nodes": [{"content": "...", "confidence": 0.95, ...}]}`
/// Returns a list of cleaned fact strings with common prefixes stripped.
fn extract_memory_facts(raw: &str) -> Vec<String> {
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(raw) {
        if let Some(nodes) = parsed.get("nodes").and_then(|n| n.as_array()) {
            return nodes.iter()
                .filter_map(|node| {
                    node.get("content").and_then(|c| c.as_str()).map(|s| {
                        s.strip_prefix("User preference: ")
                            .or_else(|| s.strip_prefix("User stated: "))
                            .or_else(|| s.strip_prefix("User fact: "))
                            .or_else(|| s.strip_prefix("Fact: "))
                            .unwrap_or(s)
                            .to_string()
                    })
                })
                .collect();
        }
    }
    // Not JSON — return as single item
    if !raw.is_empty() { vec![raw.to_string()] } else { vec![] }
}

/// Format memory recall through a micro-LLM call for natural, conversational response.
///
/// Instead of parroting raw facts ("My favorite database is PostgreSQL"),
/// Hydra responds as someone who KNOWS the user ("PostgreSQL — solid choice
/// for what you're building.").
async fn format_memory_recall_naturally(
    query: &str,
    facts: &[String],
    user_name: &str,
    llm_config: &hydra_model::LlmConfig,
    model: &str,
) -> String {
    if facts.is_empty() {
        return "I don't have anything stored about that.".into();
    }

    let facts_text = facts.join("\n");

    // Build a tiny prompt (~100 output tokens) to format the recall naturally
    let system = format!(
        "You are recalling facts you know about the user{}. \
         Respond naturally as someone who KNOWS them — like a trusted partner, not a database. \
         Rules:\n\
         - NEVER parrot the raw fact back. Don't say \"Your favorite X is Y\" robotically.\n\
         - Show you REMEMBER — weave the fact into a warm, brief response.\n\
         - The facts belong to THE USER, not to you. Never say \"My favorite...\".\n\
         - Match their vibe: if they're technical, be technical. If casual, be casual.\n\
         - Keep it to 1-2 sentences. Be warm, direct, personal.\n\
         - If relevant, offer to help with something related.\n\
         - If multiple facts, naturally weave them together.\n\n\
         Examples of GOOD responses:\n\
         Query: \"what's my favorite database\" | Fact: \"PostgreSQL\"\n\
         → \"PostgreSQL — you've been solid on that. Want me to set up a new one?\"\n\n\
         Query: \"what languages do I know\" | Fact: \"Rust, Python, TypeScript\"\n\
         → \"Rust is your main thing, plus Python and TypeScript. Need help with any of them?\"\n\n\
         Query: \"what am I working on\" | Fact: \"Building Hydra AI orchestrator\"\n\
         → \"Hydra — the AI orchestrator. What's the next piece you want to tackle?\"",
        if user_name.is_empty() { String::new() } else { format!(" ({})", user_name) }
    );

    let user_message = format!("User asked: \"{}\"\nFacts I know: {}", query, facts_text);

    let request = hydra_model::CompletionRequest {
        model: model.to_string(),
        messages: vec![hydra_model::providers::Message {
            role: "user".into(),
            content: user_message,
        }],
        max_tokens: 150,
        temperature: Some(0.7),
        system: Some(system),
    };

    // Use the cheapest available model for this tiny formatting call
    let result = if llm_config.anthropic_api_key.is_some() {
        match hydra_model::providers::anthropic::AnthropicClient::new(llm_config) {
            Ok(client) => client.complete(request).await.ok(),
            Err(_) => None,
        }
    } else if llm_config.openai_api_key.is_some() {
        match hydra_model::providers::openai::OpenAiClient::new(llm_config) {
            Ok(client) => client.complete(request).await.ok(),
            Err(_) => None,
        }
    } else {
        None
    };

    if let Some(resp) = result {
        if !resp.content.trim().is_empty() {
            return resp.content.trim().to_string();
        }
    }

    // Fallback: if LLM call fails, format locally (better than raw dump)
    format_memory_fallback(facts)
}

/// Local fallback formatting when LLM is unavailable — still conversational, not robotic.
fn format_memory_fallback(facts: &[String]) -> String {
    if facts.is_empty() {
        return "I don't have anything stored about that.".into();
    }
    if facts.len() == 1 {
        let fact = &facts[0];
        // Don't just capitalize and return — add a conversational wrapper
        format!("{} — that's what I've got. Need anything related?", fact)
    } else {
        let mut result = String::from("Here's what I know:\n\n");
        for fact in facts {
            result.push_str(&format!("• {}\n", fact));
        }
        result.push_str("\nAnything specific you want to dig into?");
        result
    }
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
    federation: Option<Arc<crate::federation::FederationManager>>,
) {
    use crate::sisters::Sisters;

    let text = &config.text;
    eprintln!("[hydra:loop] INPUT: {:?}", &text[..text.len().min(120)]);

    // ═══════════════════════════════════════════════════════════
    // INTENT CLASSIFICATION — Micro-LLM classifier (~150 tokens)
    // Uses cheapest model (Haiku) to understand MEANING.
    // Works in any language, any phrasing, any slang.
    // ═══════════════════════════════════════════════════════════
    let mut classify_llm_config = hydra_model::LlmConfig::from_env();
    if let Some(ref oauth_token) = config.anthropic_oauth_token {
        classify_llm_config.anthropic_api_key = Some(oauth_token.clone());
    } else if !config.anthropic_key.is_empty() {
        classify_llm_config.anthropic_api_key = Some(config.anthropic_key.clone());
    }
    if !config.openai_key.is_empty() {
        classify_llm_config.openai_api_key = Some(config.openai_key.clone());
    }
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
    // CRYSTALLIZED SKILL SHORTCUT — bypass LLM for learned patterns
    // If user input matches a crystallized skill (3+ successful executions),
    // execute it directly without LLM involvement.
    // ═══════════════════════════════════════════════════════════
    if let Some(ref inv) = inventions {
        if let Some((skill_name, skill_actions)) = inv.match_crystallized_skill(text) {
            eprintln!("[hydra:crystal] Matched crystallized skill '{}' — bypassing LLM", skill_name);
            let _ = tx.send(CognitiveUpdate::Phase("Act (crystallized)".into()));
            let _ = tx.send(CognitiveUpdate::IconState("working".into()));

            // Extract the shell command from the action chain (act:respond or act:execute_plan)
            let cmd = skill_actions.iter()
                .find(|a| a.starts_with("act:"))
                .map(|a| a.strip_prefix("act:").unwrap_or(a).to_string());

            // For slash-command skills, re-execute the original command
            if text.starts_with('/') {
                if let Some(slash_result) = handle_universal_slash_command(text) {
                    if !slash_result.starts_with("__TEXT__:") {
                        match tokio::process::Command::new("sh")
                            .arg("-c")
                            .arg(&slash_result)
                            .output()
                            .await
                        {
                            Ok(output) => {
                                let stdout = String::from_utf8_lossy(&output.stdout);
                                let stderr = String::from_utf8_lossy(&output.stderr);
                                let combined = if stderr.is_empty() { stdout.to_string() }
                                    else if stdout.is_empty() { stderr.to_string() }
                                    else { format!("{}\n{}", stdout, stderr) };
                                let _ = tx.send(CognitiveUpdate::Message {
                                    role: "hydra".into(),
                                    content: format!("⚡ *Crystallized skill `{}`*\n\n```\n{}\n```",
                                        skill_name, combined.trim()),
                                    css_class: "message hydra".into(),
                                });
                            }
                            Err(e) => {
                                let _ = tx.send(CognitiveUpdate::Message {
                                    role: "hydra".into(),
                                    content: format!("Crystallized skill `{}` failed: {}", skill_name, e),
                                    css_class: "message hydra error".into(),
                                });
                            }
                        }
                    } else {
                        let content = slash_result.strip_prefix("__TEXT__:").unwrap_or(&slash_result);
                        let _ = tx.send(CognitiveUpdate::Message {
                            role: "hydra".into(),
                            content: content.to_string(),
                            css_class: "message hydra".into(),
                        });
                    }
                    let _ = tx.send(CognitiveUpdate::SkillCrystallized {
                        name: skill_name,
                        actions_count: skill_actions.len(),
                    });
                    let _ = tx.send(CognitiveUpdate::ResetIdle);
                    return;
                }
            }

            // For non-slash crystallized skills, note it and fall through to normal processing
            // (the LLM still processes but the skill match is logged)
            eprintln!("[hydra:crystal] Non-slash skill '{}' — proceeding with LLM (cmd={:?})", skill_name, cmd);
        }
    }

    // ═══════════════════════════════════════════════════════════
    // DIRECT HANDLER DISPATCH — route by classified intent
    // If the intent has a direct handler and confidence >= 0.6,
    // handle it immediately without involving the LLM.
    // ═══════════════════════════════════════════════════════════

    // ── GREETING / FAREWELL / THANKS — instant response ──
    if intent.confidence >= 0.6 {
        match intent.category {
            super::intent_router::IntentCategory::Greeting => {
                let _ = tx.send(CognitiveUpdate::Message {
                    role: "hydra".into(),
                    content: format!("Hey{}! What can I do for you?",
                        if config.user_name.is_empty() { String::new() }
                        else { format!(", {}", config.user_name) }),
                    css_class: "message hydra".into(),
                });
                let _ = tx.send(CognitiveUpdate::ResetIdle);
                return;
            }
            super::intent_router::IntentCategory::Farewell => {
                let _ = tx.send(CognitiveUpdate::Message {
                    role: "hydra".into(),
                    content: "See you later! I'll be here when you need me.".into(),
                    css_class: "message hydra".into(),
                });
                let _ = tx.send(CognitiveUpdate::ResetIdle);
                return;
            }
            super::intent_router::IntentCategory::Thanks => {
                let _ = tx.send(CognitiveUpdate::Message {
                    role: "hydra".into(),
                    content: "You're welcome! Anything else?".into(),
                    css_class: "message hydra".into(),
                });
                let _ = tx.send(CognitiveUpdate::ResetIdle);
                return;
            }
            _ => {} // Fall through to existing handlers below
        }
    }

    // ── MEMORY RECALL — natural conversational response, not raw dump ──
    if intent.category == super::intent_router::IntentCategory::MemoryRecall && intent.confidence >= 0.6 {
        let _ = tx.send(CognitiveUpdate::Phase("Recall".into()));
        let _ = tx.send(CognitiveUpdate::IconState("working".into()));

        if let Some(ref sh) = sisters_handle {
            if let Some(ref mem) = sh.memory {
                // Build LLM config for the micro-formatting call
                let mut recall_llm_config = hydra_model::LlmConfig::from_env();
                if let Some(ref oauth_token) = config.anthropic_oauth_token {
                    recall_llm_config.anthropic_api_key = Some(oauth_token.clone());
                } else if !config.anthropic_key.is_empty() {
                    recall_llm_config.anthropic_api_key = Some(config.anthropic_key.clone());
                }
                if !config.openai_key.is_empty() {
                    recall_llm_config.openai_api_key = Some(config.openai_key.clone());
                }
                // Use cheapest model for formatting (Haiku-class)
                let recall_model = if recall_llm_config.anthropic_api_key.is_some() {
                    "claude-haiku-4-5-20251001"
                } else {
                    &config.model
                };

                // Query facts first (high-signal), then general
                let facts_result = mem.call_tool("memory_query", serde_json::json!({
                    "query": text,
                    "event_types": ["fact", "correction", "decision"],
                    "max_results": 5,
                    "sort_by": "highest_confidence"
                })).await;

                let fact_text = facts_result.ok()
                    .map(|v| crate::sisters::extract_text(&v))
                    .filter(|t| !t.is_empty() && !t.contains("No memories found"));

                if let Some(ref raw_facts) = fact_text {
                    eprintln!("[hydra:recall] Found facts: {}", &raw_facts[..raw_facts.len().min(200)]);
                    let facts = extract_memory_facts(raw_facts);
                    if !facts.is_empty() {
                        let formatted = format_memory_recall_naturally(
                            text, &facts, &config.user_name, &recall_llm_config, recall_model
                        ).await;
                        let _ = tx.send(CognitiveUpdate::Message {
                            role: "hydra".into(),
                            content: formatted,
                            css_class: "message hydra".into(),
                        });
                        let _ = tx.send(CognitiveUpdate::ResetIdle);
                        return;
                    }
                }

                // No facts found — try general memory
                let general_result = mem.call_tool("memory_query", serde_json::json!({
                    "query": text,
                    "max_results": 5
                })).await;

                let general_text = general_result.ok()
                    .map(|v| crate::sisters::extract_text(&v))
                    .filter(|t| !t.is_empty() && !t.contains("No memories found"));

                if let Some(ref raw_general) = general_text {
                    eprintln!("[hydra:recall] Found general memory: {}", &raw_general[..raw_general.len().min(200)]);
                    let facts = extract_memory_facts(raw_general);
                    if !facts.is_empty() {
                        let formatted = format_memory_recall_naturally(
                            text, &facts, &config.user_name, &recall_llm_config, recall_model
                        ).await;
                        let _ = tx.send(CognitiveUpdate::Message {
                            role: "hydra".into(),
                            content: formatted,
                            css_class: "message hydra".into(),
                        });
                        let _ = tx.send(CognitiveUpdate::ResetIdle);
                        return;
                    }
                }

                // Also check beliefs
                if let Some(ref cog) = sh.cognition {
                    let beliefs_result = cog.call_tool("cognition_belief_query", serde_json::json!({"query": text})).await;
                    let belief_text = beliefs_result.ok()
                        .map(|v| crate::sisters::extract_text(&v))
                        .filter(|t| !t.is_empty());
                    if let Some(ref raw_beliefs) = belief_text {
                        let facts = extract_memory_facts(raw_beliefs);
                        if !facts.is_empty() {
                            let formatted = format_memory_recall_naturally(
                                text, &facts, &config.user_name, &recall_llm_config, recall_model
                            ).await;
                            let _ = tx.send(CognitiveUpdate::Message {
                                role: "hydra".into(),
                                content: formatted,
                                css_class: "message hydra".into(),
                            });
                            let _ = tx.send(CognitiveUpdate::ResetIdle);
                            return;
                        }
                    }
                }

                // Nothing found — let it fall through to LLM
                eprintln!("[hydra:recall] No memories found, falling through to LLM");
            }
        }
    }

    // ── MEMORY STORE — handled by existing direct memory handler below ──
    // (Uses intent.category == MemoryStore, handled by extract_memory_intent path)

    // ═══════════════════════════════════════════════════════════
    // Step 4.9: Natural language settings detection
    // ═══════════════════════════════════════════════════════════
    if intent.category == super::intent_router::IntentCategory::Settings {
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
    // SELF-REPAIR: Detect "fix yourself" intent and run repair loop
    // ═══════════════════════════════════════════════════════════
    if intent.category == super::intent_router::IntentCategory::SelfRepair {
        let _ = tx.send(CognitiveUpdate::Phase("Self-Repair".into()));
        let _ = tx.send(CognitiveUpdate::IconState("working".into()));

        let repo_root = std::env::current_dir().unwrap_or_default();
        let engine = crate::cognitive::self_repair::SelfRepairEngine::new(&repo_root);

        // Find the best repair_spec for this complaint, or run diagnostics
        if let Some(spec_name) = crate::cognitive::self_repair::find_spec_for_complaint(text) {
            let spec_path = repo_root.join("repair-specs").join(spec_name);
            if spec_path.exists() {
                if let Ok(spec) = engine.load_spec(&spec_path) {
                    let _ = tx.send(CognitiveUpdate::RepairStarted {
                        spec: spec_name.to_string(),
                        task: spec.task.clone(),
                    });

                    // Run checks only (don't auto-invoke Claude from within the loop)
                    let (all_pass, checks) = engine.run_all_checks(&spec).await;
                    let passed = checks.iter().filter(|c| c.passed).count();

                    for c in &checks {
                        let _ = tx.send(CognitiveUpdate::RepairCheckResult {
                            name: c.name.clone(),
                            passed: c.passed,
                        });
                    }

                    let status = if all_pass { "passing" } else { "needs_repair" };
                    let _ = tx.send(CognitiveUpdate::RepairCompleted {
                        task: spec.task.clone(),
                        status: status.to_string(),
                        iterations: 0,
                    });

                    let msg = if all_pass {
                        format!("Self-diagnosis complete: **{}** — all {} checks passing. No repair needed.", spec.task, checks.len())
                    } else {
                        let failures: Vec<String> = checks.iter()
                            .filter(|c| !c.passed)
                            .map(|c| format!("- {} *({})*", c.name, &c.output[..c.output.len().min(80)]))
                            .collect();
                        format!(
                            "Self-diagnosis: **{}** — {}/{} checks passing.\n\nFailing checks:\n{}\n\nRun `./scripts/hydra-self-repair.sh repair-specs/{}` to auto-repair.",
                            spec.task, passed, checks.len(), failures.join("\n"), spec_name
                        )
                    };
                    let _ = tx.send(CognitiveUpdate::Message {
                        role: "hydra".into(),
                        content: msg,
                        css_class: "message hydra self-repair".into(),
                    });
                    let _ = tx.send(CognitiveUpdate::ResetIdle);
                    return;
                }
            }
        }

        // No specific spec found — run full diagnostics
        let status = engine.status().await;
        let total = status.len();
        let passing = status.iter().filter(|(_, _, p, t)| p == t).count();

        let summary: String = status.iter()
            .map(|(file, task, passed, total)| {
                let icon = if passed == total { "✅" } else { "⚠️" };
                format!("{} **{}** ({}/{} checks) — {}", icon, file, passed, total, task)
            })
            .collect::<Vec<_>>()
            .join("\n");

        let msg = format!(
            "Self-repair diagnostics: **{}/{}** specs fully passing.\n\n{}\n\nRun `./scripts/hydra-repair-all.sh` to repair all failing specs.",
            passing, total, summary
        );
        let _ = tx.send(CognitiveUpdate::Message {
            role: "hydra".into(),
            content: msg,
            css_class: "message hydra self-repair".into(),
        });
        let _ = tx.send(CognitiveUpdate::ResetIdle);
        return;
    }

    // ═══════════════════════════════════════════════════════════
    // OMNISCIENCE: Full semantic self-repair via Codebase + Forge + Aegis
    // ═══════════════════════════════════════════════════════════
    if intent.category == super::intent_router::IntentCategory::SelfScan {
        let _ = tx.send(CognitiveUpdate::Phase("Omniscience".into()));
        let _ = tx.send(CognitiveUpdate::IconState("working".into()));

        let repo_root = std::env::current_dir().unwrap_or_default();
        let omni = crate::cognitive::omniscience::OmniscienceEngine::new(&repo_root);

        // Create a channel for omniscience updates
        let (omni_tx, mut omni_rx) = mpsc::unbounded_channel();

        // Clone tx for the forwarding task
        let tx2 = tx.clone();
        let forward_task = tokio::spawn(async move {
            while let Some(update) = omni_rx.recv().await {
                match update {
                    crate::cognitive::omniscience::OmniscienceUpdate::CodebaseAnalyzing { phase } => {
                        let _ = tx2.send(CognitiveUpdate::OmniscienceAnalyzing { phase });
                    }
                    crate::cognitive::omniscience::OmniscienceUpdate::GapFound(gap) => {
                        let _ = tx2.send(CognitiveUpdate::OmniscienceGapFound {
                            description: gap.description,
                            severity: gap.severity,
                            category: gap.category,
                        });
                    }
                    crate::cognitive::omniscience::OmniscienceUpdate::SpecGenerated { spec_name, task } => {
                        let _ = tx2.send(CognitiveUpdate::OmniscienceSpecGenerated { spec_name, task });
                    }
                    crate::cognitive::omniscience::OmniscienceUpdate::AegisValidation { spec_name, safe, recommendation } => {
                        let _ = tx2.send(CognitiveUpdate::OmniscienceValidation { spec_name, safe, recommendation });
                    }
                    crate::cognitive::omniscience::OmniscienceUpdate::ScanComplete(scan) => {
                        let _ = tx2.send(CognitiveUpdate::OmniscienceScanComplete {
                            gaps_found: scan.gaps.len(),
                            specs_generated: scan.generated_specs.len(),
                            health_score: scan.code_health_score,
                        });
                    }
                }
            }
        });

        // Run the omniscience loop (needs Sisters)
        if let Some(ref sh) = sisters_handle {
            let scan = omni.run_omniscience_loop(sh, Some(&omni_tx)).await;
            drop(omni_tx);
            let _ = forward_task.await;

            let health_pct = (scan.code_health_score * 100.0) as u32;
            let repos_scanned = scan.repo_scans.len();
            let repos_healthy = scan.repo_scans.iter()
                .filter(|r| r.health_score >= 0.9)
                .count();

            // Per-repo summary
            let repo_summary: String = scan.repo_scans.iter()
                .map(|r| {
                    let h = (r.health_score * 100.0) as u32;
                    let icon = if r.health_score >= 0.9 { "✅" } else if r.health_score >= 0.7 { "⚠️" } else { "❌" };
                    format!("{} **{}** — {}% health ({} files, {} gaps, {} specs)",
                        icon, r.repo, h, r.files_analyzed, r.gaps.len(), r.generated_specs.len())
                })
                .collect::<Vec<_>>()
                .join("\n");

            let msg = format!(
                "**Omniscience Scan Complete** — {}/{} repos healthy\n\n\
                 | Metric | Value |\n\
                 |--------|-------|\n\
                 | Repos scanned | **{}** |\n\
                 | Total files | **{}** |\n\
                 | Overall health | **{}%** |\n\
                 | Total gaps | **{}** |\n\
                 | Specs generated | **{}** |\n\n\
                 ### Per-Repo Health\n{}\n\n\
                 {}\n\n\
                 Run `./scripts/hydra-repair-all.sh` to auto-repair generated specs.",
                repos_healthy, repos_scanned,
                repos_scanned,
                scan.total_files_analyzed,
                health_pct,
                scan.gaps.len(),
                scan.generated_specs.len(),
                repo_summary,
                if scan.gaps.is_empty() {
                    "No gaps detected — all codebases are healthy.".to_string()
                } else {
                    let gap_summary: String = scan.gaps.iter().take(15)
                        .map(|g| format!("- [{}|{}] {} — {}", g.repo, g.severity, g.category, g.description))
                        .collect::<Vec<_>>()
                        .join("\n");
                    format!("### Top Gaps\n{}", gap_summary)
                }
            );
            let _ = tx.send(CognitiveUpdate::Message {
                role: "hydra".into(),
                content: msg,
                css_class: "message hydra omniscience".into(),
            });
        } else {
            drop(omni_tx);
            let _ = forward_task.await;
            let _ = tx.send(CognitiveUpdate::Message {
                role: "hydra".into(),
                content: "Omniscience loop requires Sisters to be connected (Codebase + Forge + Aegis).".into(),
                css_class: "message hydra error".into(),
            });
        }

        let _ = tx.send(CognitiveUpdate::ResetIdle);
        return;
    }

    // ═══════════════════════════════════════════════════════════
    // SISTER DIAGNOSTICS: Direct sister health check (no LLM needed)
    // ═══════════════════════════════════════════════════════════
    if intent.category == super::intent_router::IntentCategory::SisterDiagnose {
        let _ = tx.send(CognitiveUpdate::Phase("Diagnostics".into()));
        let _ = tx.send(CognitiveUpdate::IconState("working".into()));

        if let Some(ref sh) = sisters_handle {
            let target_sister = intent.target.clone();
            let mut report = String::new();

            // Header
            report.push_str("## Sister Diagnostics\n\n");

            // Overall status
            let connected = sh.connected_count();
            report.push_str(&format!("**{}/14 sisters connected**\n\n", connected));

            // Per-sister detail
            report.push_str("| Sister | Status | Tools |\n|--------|--------|-------|\n");
            for (name, opt) in sh.all_sisters() {
                let (status, tools) = if let Some(conn) = opt {
                    ("ONLINE", conn.tools.len().to_string())
                } else {
                    ("OFFLINE", "-".to_string())
                };
                let icon = if opt.is_some() { "🟢" } else { "🔴" };
                report.push_str(&format!("| {} {} | {} | {} |\n", icon, name, status, tools));
            }

            // If user asked about a specific sister, do a deeper probe
            if let Some(ref target) = target_sister {
                report.push_str(&format!("\n### Deep Probe: {}\n\n", target));
                let probe_result = match target.to_lowercase().as_str() {
                    "memory" | "agenticmemory" => {
                        if let Some(mem) = &sh.memory {
                            let r = mem.call_tool("memory_longevity_stats", serde_json::json!({})).await;
                            match r {
                                Ok(v) => format!("Memory stats: {}", serde_json::to_string_pretty(&v).unwrap_or_default()),
                                Err(e) => format!("Memory probe FAILED: {}", e),
                            }
                        } else {
                            "Memory sister is NOT connected.".to_string()
                        }
                    }
                    "identity" | "agenticidentity" => {
                        if let Some(id) = &sh.identity {
                            let r = id.call_tool("identity_whoami", serde_json::json!({})).await;
                            match r {
                                Ok(v) => format!("Identity probe: {}", serde_json::to_string_pretty(&v).unwrap_or_default()),
                                Err(e) => format!("Identity probe FAILED: {}", e),
                            }
                        } else {
                            "Identity sister is NOT connected.".to_string()
                        }
                    }
                    "cognition" | "agenticcognition" => {
                        if let Some(cog) = &sh.cognition {
                            let r = cog.call_tool("cognition_model_query", serde_json::json!({"context": "diagnostic"})).await;
                            match r {
                                Ok(v) => format!("Cognition probe: {}", serde_json::to_string_pretty(&v).unwrap_or_default()),
                                Err(e) => format!("Cognition probe FAILED: {}", e),
                            }
                        } else {
                            "Cognition sister is NOT connected.".to_string()
                        }
                    }
                    _ => {
                        // Generic: check if the named sister is connected
                        let found = sh.all_sisters().iter()
                            .find(|(n, _)| n.to_lowercase() == target.to_lowercase())
                            .map(|(_, opt)| opt.is_some());
                        match found {
                            Some(true) => format!("{} sister is connected and responsive.", target),
                            Some(false) => format!("{} sister is NOT connected. It failed to spawn at startup.", target),
                            None => format!("Unknown sister: {}. Known sisters: Memory, Identity, Codebase, Vision, Comm, Contract, Time, Planning, Cognition, Reality, Forge, Aegis, Veritas, Evolve.", target),
                        }
                    }
                };
                report.push_str(&probe_result);
            }

            let _ = tx.send(CognitiveUpdate::Message {
                role: "hydra".into(),
                content: report,
                css_class: "message hydra diagnostics".into(),
            });
        } else {
            let _ = tx.send(CognitiveUpdate::Message {
                role: "hydra".into(),
                content: "No sisters available — running in offline mode.".into(),
                css_class: "message hydra error".into(),
            });
        }

        let _ = tx.send(CognitiveUpdate::ResetIdle);
        return;
    }

    // ═══════════════════════════════════════════════════════════
    // SISTER FIX: "fix broken sisters" / "fix contract sister" → diagnose & attempt repair
    // ═══════════════════════════════════════════════════════════
    if intent.category == super::intent_router::IntentCategory::SisterRepair {
        let _ = tx.send(CognitiveUpdate::Phase("Self-Repair".into()));
        let _ = tx.send(CognitiveUpdate::IconState("working".into()));

        if let Some(ref sh) = sisters_handle {
            let target = intent.target.clone();
            let mut report = String::from("## Sister Repair Report\n\n");

            // Get offline sisters (or just the targeted one)
            let offline: Vec<(&str, &str, &[&str])> = get_sister_bin_info()
                .into_iter()
                .filter(|(name, _, _)| {
                    // Check if this sister is offline
                    let is_offline = sh.all_sisters().iter()
                        .any(|(n, opt)| n.to_lowercase() == name.to_lowercase() && opt.is_none());
                    // If user targeted a specific sister, only fix that one
                    if let Some(ref t) = target {
                        is_offline && t.to_lowercase() == name.to_lowercase()
                    } else {
                        is_offline
                    }
                })
                .collect();

            if offline.is_empty() {
                if let Some(ref t) = target {
                    report.push_str(&format!("**{}** sister is already online! No fix needed.\n", t));
                } else {
                    report.push_str("All sisters are online! Nothing to fix.\n");
                }
            } else {
                report.push_str(&format!("Found **{}** offline sister(s). Repairing...\n\n", offline.len()));

                for (name, bin_name, args) in &offline {
                    report.push_str(&format!("### {} Sister\n\n", name));

                    let home = std::env::var("HOME").unwrap_or_default();
                    let bin_path = format!("{}/.local/bin/{}", home, bin_name);
                    let name_lower = name.to_lowercase();
                    let workspace_root = format!("{}/Documents/agentralabs-tech", home);
                    let sister_repo = format!("{}/agentic-{}", workspace_root, name_lower);
                    let mcp_crate = format!("{}/crates/agentic-{}-mcp", sister_repo, name_lower);
                    let has_repo = std::path::Path::new(&mcp_crate).exists();

                    // Track all attempts for final summary
                    let mut attempts: Vec<(String, String)> = Vec::new();
                    let mut fixed = false;

                    // ── Attempt 1: Direct respawn ──
                    if std::path::Path::new(&bin_path).exists() {
                        report.push_str("**Attempt 1:** Respawning process...\n");
                        let _ = tx.send(CognitiveUpdate::Message {
                            role: "hydra".into(),
                            content: report.clone(),
                            css_class: "message hydra diagnostics".into(),
                        });

                        match SisterConnection::spawn(name, &bin_path, args).await {
                            Ok(conn) => {
                                report.push_str(&format!("**{} is back online!** ({} tools)\n\n", name, conn.tools.len()));
                                fixed = true;
                            }
                            Err(e) => {
                                let err = e.to_string();
                                let short = err[..err.len().min(120)].to_string();
                                report.push_str(&format!("Failed: {}\n", short));
                                attempts.push(("Respawn".into(), short));
                            }
                        }
                    } else {
                        attempts.push(("Respawn".into(), "Binary not found".into()));
                    }

                    // ── Attempt 2: Fix corrupted data, then respawn ──
                    if !fixed {
                        let db_candidates = vec![
                            format!("{}/.hydra/{}.db", home, name_lower),
                            format!("{}/.hydra/{}.sqlite", home, name_lower),
                        ];
                        let mut db_fixed = false;
                        for db_path in &db_candidates {
                            if std::path::Path::new(db_path).exists() {
                                report.push_str(&format!("**Attempt 2:** Moving aside DB `{}`...\n", db_path));
                                let backup = format!("{}.bak.{}", db_path, chrono::Utc::now().timestamp());
                                if std::fs::rename(db_path, &backup).is_ok() {
                                    report.push_str("DB backed up. Respawning...\n");
                                    db_fixed = true;
                                    match SisterConnection::spawn(name, &bin_path, args).await {
                                        Ok(conn) => {
                                            report.push_str(&format!("**{} is back online!** ({} tools)\n\n", name, conn.tools.len()));
                                            fixed = true;
                                        }
                                        Err(e) => {
                                            let err = e.to_string();
                                            let short = err[..err.len().min(120)].to_string();
                                            report.push_str(&format!("Still failed: {}\n", short));
                                            attempts.push(("DB repair + respawn".into(), short));
                                        }
                                    }
                                    break;
                                }
                            }
                        }
                        if !db_fixed && !fixed {
                            attempts.push(("DB repair".into(), "No DB file found to repair".into()));
                        }
                    }

                    // ── Attempt 3: Rebuild from source, then respawn ──
                    if !fixed && has_repo {
                        report.push_str(&format!("**Attempt 3:** Rebuilding from source `{}`...\n", mcp_crate));
                        let _ = tx.send(CognitiveUpdate::Message {
                            role: "hydra".into(),
                            content: report.clone(),
                            css_class: "message hydra diagnostics".into(),
                        });

                        let build_result = tokio::process::Command::new("cargo")
                            .args(["install", "--path", &mcp_crate])
                            .stdout(std::process::Stdio::piped())
                            .stderr(std::process::Stdio::piped())
                            .output()
                            .await;

                        match build_result {
                            Ok(output) if output.status.success() => {
                                report.push_str("Rebuild succeeded. Respawning...\n");
                                match SisterConnection::spawn(name, &bin_path, args).await {
                                    Ok(conn) => {
                                        report.push_str(&format!("**{} is back online!** ({} tools)\n\n", name, conn.tools.len()));
                                        fixed = true;
                                    }
                                    Err(e) => {
                                        let err = e.to_string();
                                        let short = err[..err.len().min(120)].to_string();
                                        report.push_str(&format!("Rebuild OK but respawn failed: {}\n", short));
                                        attempts.push(("Rebuild + respawn".into(), short));
                                    }
                                }
                            }
                            Ok(output) => {
                                let stderr = String::from_utf8_lossy(&output.stderr);
                                let err_tail = if stderr.len() > 300 { &stderr[stderr.len() - 300..] } else { &stderr };
                                report.push_str(&format!("Rebuild failed: {}\n", err_tail.trim()));
                                attempts.push(("Rebuild".into(), err_tail.trim().to_string()));
                            }
                            Err(e) => {
                                report.push_str(&format!("Could not run cargo: {}\n", e));
                                attempts.push(("Rebuild".into(), e.to_string()));
                            }
                        }
                    } else if !fixed && !has_repo {
                        attempts.push(("Rebuild".into(), format!("Repo not found at {}", mcp_crate)));
                    }

                    // ── Attempt 4: Try alternative args (--stdio, serve, no args) ──
                    if !fixed && std::path::Path::new(&bin_path).exists() {
                        let alt_args_list: Vec<&[&str]> = vec![
                            &["--stdio"],
                            &["serve", "--stdio"],
                            &["serve"],
                            &[],
                        ];
                        // Only try args that differ from the original
                        for alt in &alt_args_list {
                            if *alt as &[&str] == *args { continue; }
                            report.push_str(&format!("**Attempt 4:** Trying args: `{}`...\n", alt.join(" ")));
                            match SisterConnection::spawn(name, &bin_path, alt).await {
                                Ok(conn) => {
                                    report.push_str(&format!("**{} is back online!** ({} tools) — with args: `{}`\n\n",
                                        name, conn.tools.len(), alt.join(" ")));
                                    fixed = true;
                                    break;
                                }
                                Err(e) => {
                                    let err = e.to_string();
                                    let short = err[..err.len().min(80)].to_string();
                                    report.push_str(&format!("Failed: {}\n", short));
                                    attempts.push((format!("Alt args `{}`", alt.join(" ")), short));
                                }
                            }
                        }
                    }

                    // ── Attempt 5: Clean rebuild (cargo clean first) ──
                    if !fixed && has_repo {
                        report.push_str("**Attempt 5:** Clean rebuild (cargo clean + install)...\n");
                        let _ = tx.send(CognitiveUpdate::Message {
                            role: "hydra".into(),
                            content: report.clone(),
                            css_class: "message hydra diagnostics".into(),
                        });

                        // Clean the sister's target dir
                        let _ = tokio::process::Command::new("cargo")
                            .args(["clean"])
                            .current_dir(&sister_repo)
                            .stdout(std::process::Stdio::null())
                            .stderr(std::process::Stdio::null())
                            .output()
                            .await;

                        let build_result = tokio::process::Command::new("cargo")
                            .args(["install", "--path", &mcp_crate, "--force"])
                            .stdout(std::process::Stdio::piped())
                            .stderr(std::process::Stdio::piped())
                            .output()
                            .await;

                        match build_result {
                            Ok(output) if output.status.success() => {
                                report.push_str("Clean rebuild succeeded. Respawning...\n");
                                match SisterConnection::spawn(name, &bin_path, args).await {
                                    Ok(conn) => {
                                        report.push_str(&format!("**{} is back online!** ({} tools)\n\n", name, conn.tools.len()));
                                        fixed = true;
                                    }
                                    Err(e) => {
                                        let err = e.to_string();
                                        let short = err[..err.len().min(120)].to_string();
                                        report.push_str(&format!("Clean rebuild OK but respawn still failed: {}\n", short));
                                        attempts.push(("Clean rebuild + respawn".into(), short));
                                    }
                                }
                            }
                            Ok(output) => {
                                let stderr = String::from_utf8_lossy(&output.stderr);
                                let err_tail = if stderr.len() > 300 { &stderr[stderr.len() - 300..] } else { &stderr };
                                attempts.push(("Clean rebuild".into(), err_tail.trim().to_string()));
                            }
                            Err(e) => {
                                attempts.push(("Clean rebuild".into(), e.to_string()));
                            }
                        }
                    }

                    // ── Attempt 6: Source code diagnosis (protocol mismatch) ──
                    if !fixed && has_repo {
                        let all_errors_so_far = attempts.iter().map(|(_, e)| e.as_str()).collect::<Vec<_>>().join(" ");
                        let is_protocol_issue = all_errors_so_far.contains("Content-Length")
                            || all_errors_so_far.contains("expected value at line 1 column 1");

                        if is_protocol_issue {
                            report.push_str("**Attempt 6:** Diagnosing protocol mismatch in source code...\n");
                            let _ = tx.send(CognitiveUpdate::Message {
                                role: "hydra".into(),
                                content: report.clone(),
                                css_class: "message hydra diagnostics".into(),
                            });

                            // Read the broken sister's main.rs
                            let main_rs = format!("{}/src/main.rs", mcp_crate);
                            let broken_src = tokio::fs::read_to_string(&main_rs).await.ok();

                            // Read a WORKING sister's main.rs for comparison (Memory sister works)
                            let working_main = format!("{}/agentic-memory/crates/agentic-memory-mcp/src/main.rs", workspace_root);
                            let working_src = tokio::fs::read_to_string(&working_main).await.ok();

                            // Also check Cargo.toml for MCP dependency versions
                            let broken_cargo = format!("{}/Cargo.toml", mcp_crate);
                            let broken_deps = tokio::fs::read_to_string(&broken_cargo).await.ok();
                            let working_cargo = format!("{}/agentic-memory/crates/agentic-memory-mcp/Cargo.toml", workspace_root);
                            let working_deps = tokio::fs::read_to_string(&working_cargo).await.ok();

                            report.push_str("\n**Source Code Diagnosis:**\n\n");

                            // Compare MCP transport setup
                            if let (Some(ref broken), Some(ref working)) = (&broken_src, &working_src) {
                                // Check for Content-Length / HTTP framing indicators
                                let broken_has_http = broken.contains("Content-Length")
                                    || broken.contains("content_length")
                                    || broken.contains("http_transport")
                                    || broken.contains("HttpTransport")
                                    || broken.contains("lsp_transport");
                                let working_has_http = working.contains("Content-Length")
                                    || working.contains("content_length")
                                    || working.contains("http_transport")
                                    || working.contains("HttpTransport");

                                if broken_has_http && !working_has_http {
                                    report.push_str(&format!(
                                        "`{}` uses HTTP/LSP framing (Content-Length headers).\n\
                                         Working sister (Memory) uses raw JSON-RPC over stdio.\n\
                                         **Fix needed:** Change transport in `{}`\n\n",
                                        bin_name, main_rs
                                    ));
                                }

                                // Check for stdio setup differences
                                let broken_stdio = broken.contains("StdioTransport")
                                    || broken.contains("stdio_transport")
                                    || broken.contains("stdin") && broken.contains("stdout");
                                let working_stdio = working.contains("StdioTransport")
                                    || working.contains("stdio_transport")
                                    || working.contains("stdin") && working.contains("stdout");

                                if working_stdio && !broken_stdio {
                                    report.push_str(&format!(
                                        "Working sister uses stdio transport. `{}` does NOT.\n\
                                         The sister's MCP server needs to be configured for stdio transport.\n\n",
                                        bin_name
                                    ));
                                }

                                // Show key differences in transport setup (first 20 lines with "transport" or "serve")
                                let broken_transport_lines: Vec<&str> = broken.lines()
                                    .filter(|l| {
                                        let lower = l.to_lowercase();
                                        lower.contains("transport") || lower.contains("serve")
                                            || lower.contains("stdin") || lower.contains("stdout")
                                            || lower.contains("content_length") || lower.contains("content-length")
                                    })
                                    .take(10)
                                    .collect();
                                if !broken_transport_lines.is_empty() {
                                    report.push_str(&format!("Relevant lines in `{}`:\n```rust\n", main_rs));
                                    for line in &broken_transport_lines {
                                        report.push_str(&format!("{}\n", line.trim()));
                                    }
                                    report.push_str("```\n\n");
                                }

                                let working_transport_lines: Vec<&str> = working.lines()
                                    .filter(|l| {
                                        let lower = l.to_lowercase();
                                        lower.contains("transport") || lower.contains("serve")
                                            || lower.contains("stdin") || lower.contains("stdout")
                                    })
                                    .take(10)
                                    .collect();
                                if !working_transport_lines.is_empty() {
                                    report.push_str(&format!("Working sister (Memory) uses:\n```rust\n"));
                                    for line in &working_transport_lines {
                                        report.push_str(&format!("{}\n", line.trim()));
                                    }
                                    report.push_str("```\n\n");
                                }
                            } else {
                                if broken_src.is_none() {
                                    report.push_str(&format!("Could not read `{}`\n", main_rs));
                                }
                            }

                            // Compare MCP dependency versions
                            if let (Some(ref broken_d), Some(ref working_d)) = (&broken_deps, &working_deps) {
                                // Extract MCP-related deps
                                let extract_mcp_deps = |toml: &str| -> Vec<String> {
                                    toml.lines()
                                        .filter(|l| l.contains("mcp") || l.contains("transport") || l.contains("jsonrpc"))
                                        .map(|l| l.trim().to_string())
                                        .collect()
                                };
                                let broken_mcp = extract_mcp_deps(broken_d);
                                let working_mcp = extract_mcp_deps(working_d);

                                if broken_mcp != working_mcp {
                                    report.push_str("**Dependency differences:**\n");
                                    if !broken_mcp.is_empty() {
                                        report.push_str(&format!("  {} uses: {}\n", bin_name, broken_mcp.join(", ")));
                                    }
                                    if !working_mcp.is_empty() {
                                        report.push_str(&format!("  Memory uses: {}\n", working_mcp.join(", ")));
                                    }
                                    report.push('\n');
                                }
                            }

                            attempts.push(("Source code diagnosis".into(), "Protocol mismatch identified".into()));
                        }
                    }

                    // ── Final report if not fixed ──
                    if !fixed {
                        report.push_str(&format!("\n**Tried {} approaches for {} — all failed.**\n\n", attempts.len(), name));
                        for (i, (approach, error)) in attempts.iter().enumerate() {
                            let short_err = if error.len() > 100 { &error[..100] } else { error.as_str() };
                            report.push_str(&format!("{}. **{}** — {}\n", i + 1, approach, short_err));
                        }

                        // Root cause summary
                        let all_errors = attempts.iter().map(|(_, e)| e.as_str()).collect::<Vec<_>>().join(" ");
                        if all_errors.contains("Content-Length") || all_errors.contains("expected value at line 1 column 1")
                            || all_errors.contains("Protocol mismatch") {
                            report.push_str(&format!(
                                "\n**Root blocker:** `{}` outputs HTTP-framed protocol (Content-Length headers) \
                                 but Hydra expects raw JSON-RPC over stdio. The fix requires changing the MCP transport \
                                 configuration in `agentic-{}/crates/agentic-{}-mcp/src/main.rs` to match the working sisters.\n\n",
                                bin_name, name_lower, name_lower
                            ));
                        } else if all_errors.contains("File format error") || all_errors.contains("Unknown entity") {
                            report.push_str("\n**Root blocker:** Persistent database corruption that survived backup+recreate.\n\n");
                        } else if all_errors.contains("No such file") {
                            report.push_str("\n**Root blocker:** Binary not installed and repo not available for rebuild.\n\n");
                        } else {
                            report.push_str(&format!("\n**Root blocker:** Binary crashes on startup. \
                                The error is not a known pattern.\n\n"));
                        }
                    }
                }
            }

            let _ = tx.send(CognitiveUpdate::Message {
                role: "hydra".into(),
                content: report,
                css_class: "message hydra diagnostics".into(),
            });
        } else {
            let _ = tx.send(CognitiveUpdate::Message {
                role: "hydra".into(),
                content: "No sisters available — running in offline mode.".into(),
                css_class: "message hydra error".into(),
            });
        }

        let _ = tx.send(CognitiveUpdate::ResetIdle);
        return;
    }

    // ═══════════════════════════════════════════════════════════
    // DIRECT MEMORY: "remember X" / "note that X" → save directly, skip LLM
    // ═══════════════════════════════════════════════════════════
    let memory_payload = if intent.category == super::intent_router::IntentCategory::MemoryStore {
        intent.payload.clone().or_else(|| Some(text.to_string()))
    } else {
        None
    };
    if let Some(fact) = memory_payload {
        let _ = tx.send(CognitiveUpdate::Phase("Learn (direct)".into()));
        let _ = tx.send(CognitiveUpdate::IconState("working".into()));
        eprintln!("[hydra:memory] Saving directly: {}", &fact[..fact.len().min(80)]);

        let mut saved = false;
        if let Some(ref sh) = sisters_handle {
            if let Some(ref mem) = sh.memory {
                // memory_add requires event_type (fact|decision|inference|correction|skill|episode)
                let payload = serde_json::json!({
                    "event_type": "fact",
                    "content": format!("User preference: {}", fact),
                    "confidence": 0.95
                });
                match mem.call_tool("memory_add", payload).await {
                    Ok(v) => {
                        saved = true;
                        eprintln!("[hydra:memory] memory_add OK: {}", serde_json::to_string(&v).unwrap_or_default());
                    }
                    Err(e) => { eprintln!("[hydra:memory] memory_add FAILED: {}", e); }
                }
            }
            // Also store as a belief via cognition
            if let Some(ref cog) = sh.cognition {
                let _ = cog.call_tool("cognition_belief_add", serde_json::json!({
                    "subject": "user_preference",
                    "content": fact,
                    "confidence": 1.0,
                    "source": "explicit_user_statement"
                })).await;
            }
        }

        let msg = if saved {
            format!("Got it! I'll remember that: **{}**", fact)
        } else {
            format!("I'll remember: **{}** (note: memory sister may be offline)", fact)
        };
        let _ = tx.send(CognitiveUpdate::Message {
            role: "hydra".into(),
            content: msg,
            css_class: "message hydra".into(),
        });
        let _ = tx.send(CognitiveUpdate::ResetIdle);
        return;
    }

    // ═══════════════════════════════════════════════════════════
    // SLASH COMMAND HANDLER — works universally (Desktop + TUI)
    // Detects /test, /files, /git, /build, /run, etc. and routes
    // to direct shell execution. TUI also handles these locally,
    // but this ensures Desktop gets the same capability.
    // ═══════════════════════════════════════════════════════════
    if text.starts_with('/') {
        if let Some(slash_result) = handle_universal_slash_command(text) {
            let _ = tx.send(CognitiveUpdate::Phase("Act (command)".into()));
            let _ = tx.send(CognitiveUpdate::IconState("working".into()));

            // Some slash commands return static text (no shell execution needed)
            if slash_result.starts_with("__TEXT__:") {
                let content = slash_result.strip_prefix("__TEXT__:").unwrap_or(&slash_result);
                let _ = tx.send(CognitiveUpdate::Message {
                    role: "hydra".into(),
                    content: content.to_string(),
                    css_class: "message hydra".into(),
                });
                let _ = tx.send(CognitiveUpdate::ResetIdle);
                return;
            }

            // Execute the shell command
            eprintln!("[hydra:slash] Executing: {}", &slash_result[..slash_result.len().min(100)]);
            match tokio::process::Command::new("sh")
                .arg("-c")
                .arg(&slash_result)
                .output()
                .await
            {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let combined = if stderr.is_empty() {
                        stdout.to_string()
                    } else if stdout.is_empty() {
                        stderr.to_string()
                    } else {
                        format!("{}\n{}", stdout, stderr)
                    };
                    let display = if combined.trim().is_empty() {
                        "Done.".to_string()
                    } else {
                        format!("```\n{}\n```", combined.trim())
                    };
                    let _ = tx.send(CognitiveUpdate::Message {
                        role: "hydra".into(),
                        content: display,
                        css_class: "message hydra".into(),
                    });
                }
                Err(e) => {
                    let _ = tx.send(CognitiveUpdate::Message {
                        role: "hydra".into(),
                        content: format!("Command failed: {}", e),
                        css_class: "message hydra error".into(),
                    });
                }
            }
            let _ = tx.send(CognitiveUpdate::ResetIdle);
            return;
        }
    }

    // ═══════════════════════════════════════════════════════════
    // DIRECT ACTION FAST-PATH: Execute immediately, skip LLM
    // If we can detect a concrete shell command from the user's
    // intent, run it NOW instead of wasting tokens on an LLM call.
    // ═══════════════════════════════════════════════════════════
    let direct_result = detect_direct_action_command(text)
        .or_else(|| detect_system_control(text));
    eprintln!("[hydra:loop] direct_action_check: {:?}", direct_result.as_ref().map(|c| &c[..c.len().min(80)]));
    if let Some(direct_cmd) = direct_result {
        let _ = tx.send(CognitiveUpdate::Phase("Act (direct)".into()));
        let _ = tx.send(CognitiveUpdate::IconState("working".into()));

        // Quick risk check — only block truly dangerous commands
        let gate_result = decide_engine.evaluate_command(&direct_cmd);
        if gate_result.risk_score >= 0.9 || gate_result.anomaly_detected || gate_result.boundary_blocked {
            let _ = tx.send(CognitiveUpdate::Message {
                role: "hydra".into(),
                content: format!("Blocked: {}", gate_result.reason),
                css_class: "message hydra error".into(),
            });
            let _ = tx.send(CognitiveUpdate::ResetIdle);
            return;
        }

        // Execute the command directly
        let _ = tx.send(CognitiveUpdate::Typing(false));
        eprintln!("[hydra:direct] Executing: {}", &direct_cmd[..direct_cmd.len().min(100)]);
        match tokio::process::Command::new("sh")
            .arg("-c")
            .arg(&direct_cmd)
            .output()
            .await
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let combined = if stderr.is_empty() {
                    stdout.to_string()
                } else if stdout.is_empty() {
                    stderr.to_string()
                } else {
                    format!("{}\n{}", stdout, stderr)
                };
                let display = if combined.trim().is_empty() {
                    "Done.".to_string()
                } else {
                    format!("```\n{}\n```", combined.trim())
                };
                let _ = tx.send(CognitiveUpdate::Message {
                    role: "hydra".into(),
                    content: display,
                    css_class: "message hydra".into(),
                });

                // LEARN: capture this in memory
                if let Some(ref sh) = sisters_handle {
                    sh.learn(text, &combined[..combined.len().min(500)]).await;
                }
            }
            Err(e) => {
                let _ = tx.send(CognitiveUpdate::Message {
                    role: "hydra".into(),
                    content: format!("Command failed: {}", e),
                    css_class: "message hydra error".into(),
                });
            }
        }

        let _ = tx.send(CognitiveUpdate::ResetIdle);
        return;
    }

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
    // PHASE 1: PERCEIVE — Query ALL sisters for context
    // ═══════════════════════════════════════════════════════════
    let _ = tx.send(CognitiveUpdate::Phase("Perceive".into()));
    let _ = tx.send(CognitiveUpdate::IconState("listening".into()));
    let _ = tx.send(CognitiveUpdate::PhaseStatuses(vec![
        PhaseStatus { phase: CognitivePhase::Perceive, state: PhaseState::Running, tokens_used: None, duration_ms: None },
    ]));
    let perceive_start = Instant::now();

    // Surface any dream insights from idle processing (inventions integration).
    // tick_idle advances the dream state machine: idle → dreaming → surfacing.
    // When the user returns, we harvest any insights generated during idle time.
    if let Some(ref inv) = inventions {
        inv.tick_idle(0); // Advance idle/dream state before resetting
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

    // REAL PERCEIVE: Simple queries only query memory + cognition.
    // Complex queries dispatch to ALL sisters in parallel.
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

    // ── BELIEF LOADING: Load active beliefs from DB for context injection ──
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

    // ── MCP SKILL DISCOVERY: Discover tools from connected sisters (complex only) ──
    // Simple queries don't need 500+ tool names — saves ~2000 tokens.
    let _mcp_context = if !is_complex {
        None
    } else if let Some(ref sh) = sisters_handle {
        let tools = sh.discover_mcp_tools();
        if !tools.is_empty() {
            // Persist discovered tools to DB
            if let Some(ref db) = db {
                let now = chrono::Utc::now().to_rfc3339();
                let mut servers_seen: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
                for (server, tool_name) in &tools {
                    servers_seen.entry(server.clone()).or_default().push(tool_name.clone());
                    let skill_id = format!("mcp-{}-{}", server.to_lowercase(), tool_name);
                    let _ = db.upsert_mcp_skill(&McpDiscoveredSkillRow {
                        id: skill_id,
                        server_name: server.clone(),
                        tool_name: tool_name.clone(),
                        description: None,
                        input_schema: None,
                        discovered_at: now.clone(),
                        last_used_at: None,
                        use_count: 0,
                        active: true,
                    });
                }
                // Report discovery per server
                for (server, tool_names) in &servers_seen {
                    let _ = tx.send(CognitiveUpdate::McpSkillsDiscovered {
                        server: server.clone(),
                        tools: tool_names.clone(),
                        count: tool_names.len(),
                    });
                }
            }
            // Build context summary for prompt injection
            let mut by_server: std::collections::HashMap<&str, Vec<&str>> = std::collections::HashMap::new();
            for (server, tool_name) in &tools {
                by_server.entry(server).or_default().push(tool_name);
            }
            let summary: String = by_server.iter()
                .map(|(server, tls)| format!("- {} ({} tools): {}", server, tls.len(), tls.join(", ")))
                .collect::<Vec<_>>()
                .join("\n");
            Some(summary)
        } else { None }
    } else { None };

    // ── FEDERATION CONTEXT: Load peer status via SyncProtocol (complex only) ──
    // The federation_state is loaded from the SyncProtocol CRDT and peer registry,
    // providing the PERCEIVE phase with awareness of available federated peers.
    let federation_context = if !is_complex {
        None
    } else if let Some(ref fed) = federation {
        if fed.is_enabled() {
            let peer_count = fed.peer_count();
            let available = fed.registry.available_peers().len();
            let federation_state = fed.sync.version();
            if let Some(ref db) = db {
                // Persist federation state
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

    // ── Veritas intent compilation: structured intent parsing (complex only) ──
    let veritas_intent = if is_complex {
        if let Some(ref sh) = sisters_handle {
            sh.think_veritas(text).await
        } else { None }
    } else { None };

    // Build LLM config with provider auto-fallback
    let mut llm_config = hydra_model::LlmConfig::from_env();
    // OAuth token takes priority over API key for Anthropic (uses subscription credits)
    if let Some(ref oauth_token) = config.anthropic_oauth_token {
        llm_config.anthropic_api_key = Some(oauth_token.clone());
    } else if !config.anthropic_key.is_empty() {
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
        // Inject active beliefs
        if let Some(ref beliefs) = beliefs_context {
            sp.push_str(&format!("\n# Active Beliefs\nThese are known facts and preferences about this user. Use them naturally:\n{}\n\n", beliefs));
        }
        // Inject trust level so LLM can answer trust queries
        let trust = decide_engine.current_trust();
        let autonomy = decide_engine.current_level();
        sp.push_str(&format!(
            "\n# Trust & Autonomy\nCurrent trust score: {:.0}%\nAutonomy level: {:?}\nThis reflects how much the user trusts Hydra based on interaction history.\n\n",
            trust * 100.0, autonomy,
        ));
        // TOOL ROUTER: Send only relevant tools based on intent.
        // 522 tools × ~60 tokens = ~31K tokens. Route to 0-30 tools = 0-1.8K tokens.
        let routed_tools = route_tools_for_prompt(&intent, &complexity, is_action_request, sh, text);
        if !routed_tools.is_empty() {
            sp.push_str(&format!("\n# Available Tools\n{}\n\n", routed_tools));
        }
        // Inject federation status
        if let Some(ref fed_ctx) = federation_context {
            sp.push_str(&format!("\n# Federation Status\n{}\n\n", fed_ctx));
        }
        sp
    } else {
        format!(
            "You are Hydra, a cognitive AI orchestrator built by Agentra Labs. \
             You are NOT a chatbot — you are an agentic executor that DOES things. \
             You can execute commands, create projects, access APIs, deploy to cloud, \
             federate across systems, and integrate with any service the user provides credentials for. \
             {}When the user asks you to do something, DO IT — never say \"I can't\" for things you can do. \
             If you need credentials or access, ask for them specifically.\n\n\
             ## How to Execute Commands:\n\
             When the user asks you to DO something (open an app, run a command, check something, read a file, \
             browse the web, access a directory), you MUST wrap the shell command in <hydra-exec> tags. \
             This is how you actually execute actions on the user's machine.\n\n\
             Examples:\n\
             - User: \"what's in this folder?\" → <hydra-exec>ls -la</hydra-exec>\n\
             - User: \"read this file\" → <hydra-exec>cat ~/path/to/file.md</hydra-exec>\n\
             - User: \"open my terminal\" → <hydra-exec>open -a Terminal</hydra-exec>\n\
             - User: \"browse the internet for top stories\" → <hydra-exec>curl -s 'https://hacker-news.firebaseio.com/v0/topstories.json?print=pretty' | head -20</hydra-exec>\n\
             - User: \"access this directory\" → <hydra-exec>ls -la ~/Documents/path</hydra-exec>\n\n\
             CRITICAL: Without <hydra-exec> tags, you are ONLY talking. The tags make things HAPPEN. \
             Always use them when the user wants you to DO something, not just talk about it. \
             Never say \"Let me do X\" without including the actual <hydra-exec> command. \
             You can include multiple <hydra-exec> tags in one response. Each will be executed in order.\n\
             The command output will be captured and shown to the user.",
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

    // Build messages with conversation history.
    // Simple queries: last 6 messages. Complex queries: last 20.
    let history_limit = if is_simple { 6 } else { 20 };
    let history_start = config.history.len().saturating_sub(history_limit);
    let max_msg_chars = if is_simple { 500 } else { 2000 };
    let mut api_messages: Vec<hydra_model::providers::Message> = Vec::new();
    for (role, content) in &config.history[history_start..] {
        let trimmed = if content.len() > max_msg_chars {
            format!("{}...", &content[..max_msg_chars])
        } else {
            content.clone()
        };
        api_messages.push(hydra_model::providers::Message {
            role: role.clone(),
            content: trimmed,
        });
    }

    // Log estimated token usage before LLM call
    let prompt_est = (system_prompt.len() + 3) / 4;
    let history_est: usize = api_messages.iter().map(|m| (m.content.len() + 3) / 4).sum();
    eprintln!("[hydra:tokens] prompt=~{} history=~{} total=~{} mode={}", prompt_est, history_est, prompt_est + history_est, if is_simple { "simple" } else { "complex" });

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

    // Future Echo: predict outcome before proceeding (inventions integration).
    // The future_echo (aka predict_outcome) step uses pattern history + risk to forecast likely outcomes,
    // allowing the DECIDE phase to gate dangerous actions before they happen.
    if let Some(ref inv) = inventions {
        let risk_float: f32 = match risk_level {
            "high" | "critical" => 0.8,
            "medium" => 0.5,
            "low" => 0.2,
            _ => 0.1,
        };
        let (confidence, recommendation, prediction_desc) =
            inv.future_echo(text, risk_float);
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
            let _cmd_decide = decide_engine.check(&gate_result.risk_level, cmd);

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

            // ── RISK SCORE LOGGING (no blocking) ──
            // Security is enforced by anomaly detection and boundary enforcement above.
            // Risk score is logged for audit trail but does NOT block execution.
            // This prevents approval timeouts that made Hydra unusable for normal commands.
            if gate_result.risk_score >= 0.5 {
                eprintln!("[hydra:security] Elevated risk {:.2} for: {}", gate_result.risk_score, &cmd[..cmd.len().min(80)]);
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

    // ── BELIEF UPDATE: Extract and persist beliefs from this interaction ──
    if let Some(ref db) = db {
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
                // "actually, we switched to FastAPI instead of Express" should find
                // beliefs mentioning "Express" or "FastAPI"
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
                    // Supersede old belief
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

    // ── FEDERATION SYNC: Sync learnings with federated peers ──
    if let Some(ref fed) = federation {
        if fed.is_enabled() && llm_result.is_ok() {
            // Sync the interaction as a state entry
            let entry = hydra_federation::sync::SyncEntry {
                key: format!("interaction:{}", config.task_id),
                value: serde_json::json!({
                    "input": &text[..text.len().min(200)],
                    "response_summary": &final_response[..final_response.len().min(200)],
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

/// Extract the subject from a belief-triggering sentence.
/// E.g., "I prefer PostgreSQL" → "PostgreSQL", "We use Express" → "Express"
fn extract_belief_subject(text: &str, trigger: &str) -> String {
    let lower = text.to_lowercase();
    if let Some(pos) = lower.find(trigger) {
        let remainder = &text[pos + trigger.len()..];
        let trimmed = remainder.trim();
        // Take the first meaningful phrase (up to 60 chars or end of sentence)
        let end = trimmed.find(|c: char| c == '.' || c == ',' || c == '!' || c == '?')
            .unwrap_or(trimmed.len().min(60));
        trimmed[..end].trim().to_string()
    } else {
        // Fallback: use first 3 words
        text.split_whitespace().take(3).collect::<Vec<_>>().join(" ")
    }
}

// DELETED: is_sister_fix_intent — replaced by Veritas intent router (IntentCategory::SisterRepair)

/// Get sister binary info: (display_name, binary_name, spawn_args)
fn get_sister_bin_info() -> Vec<(&'static str, &'static str, &'static [&'static str])> {
    vec![
        ("Memory", "agentic-memory-mcp", &["serve"] as &[&str]),
        ("Identity", "agentic-identity-mcp", &["serve"]),
        ("Codebase", "agentic-codebase-mcp", &["serve"]),
        ("Vision", "agentic-vision-mcp", &["serve"]),
        ("Comm", "agentic-comm-mcp", &["serve"]),
        ("Contract", "agentic-contract-mcp", &[]),
        ("Time", "agentic-time-mcp", &["serve"]),
        ("Planning", "agentic-planning-mcp", &["serve"]),
        ("Cognition", "agentic-cognition-mcp", &[]),
        ("Reality", "agentic-reality-mcp", &[]),
        ("Forge", "agentic-forge-mcp", &[]),
        ("Aegis", "agentic-aegis-mcp", &[]),
        ("Veritas", "agentic-veritas-mcp", &[]),
        ("Evolve", "agentic-evolve-mcp", &[]),
    ]
}

// DELETED: is_action_intent, is_sister_diagnostic_intent, extract_sister_name, is_settings_intent
// All replaced by Veritas intent router — IntentCategory::SisterRepair, SisterDiagnose, Settings, etc.
// "Words don't define what Hydra does. MEANING does."

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

    // ── Top stories / latest news / headlines → fetch via HackerNews API ──
    if lower.contains("top stories") || lower.contains("latest news")
        || lower.contains("headlines") || lower.contains("trending news")
        || lower.contains("what's happening") || lower.contains("news today")
    {
        return Some(
            "echo '=== Top Stories ===' && \
             curl -s 'https://hacker-news.firebaseio.com/v0/topstories.json' | \
             python3 -c \"import sys,json; ids=json.load(sys.stdin)[:10]; \
             [print(json.loads(__import__('urllib.request').urlopen(\
             f'https://hacker-news.firebaseio.com/v0/item/{i}.json').read())['title']) \
             for i in ids]\" 2>/dev/null || echo 'Could not fetch stories — try: open https://news.ycombinator.com'"
            .to_string()
        );
    }

    // ── "Browse the internet for X" / "search for X" → open a web search ──
    if (lower.contains("browse") && lower.contains("internet"))
        || lower.starts_with("search for ")
        || lower.starts_with("google ")
        || lower.starts_with("look up ")
    {
        // Extract the search query
        let query = if let Some(pos) = lower.find("for ") {
            &text[pos + 4..]
        } else if lower.starts_with("google ") {
            &text[7..]
        } else if lower.starts_with("look up ") {
            &text[8..]
        } else {
            text
        };
        let encoded = query.trim().replace(' ', "+");
        return Some(platform_open_url(&format!("https://www.google.com/search?q={}", encoded)));
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

    // ── Read file / access directory — direct filesystem commands ──
    if (lower.contains("read") || lower.contains("show me") || lower.contains("cat "))
        && !lower.contains("read my mind")
    {
        // Extract path-like tokens from the original text
        if let Some(path) = extract_path_from_text(text) {
            return Some(format!("cat {}", shell_escape(&path)));
        }
    }
    if (lower.contains("access") || lower.contains("what's in") || lower.contains("whats in")
        || lower.contains("list"))
        && !lower.contains("access denied") && !lower.contains("access control")
    {
        if let Some(path) = extract_path_from_text(text) {
            return Some(format!("ls -la {}", shell_escape(&path)));
        }
        // "what's in this folder" / "what's in here" / "list this directory" → current dir
        if lower.contains("this folder") || lower.contains("this directory")
            || lower.contains("this dir") || lower.contains("in here")
            || lower.contains("current folder") || lower.contains("current directory")
            || (lower.contains("what") && lower.contains("in") && !lower.contains("in my"))
        {
            return Some("ls -la .".to_string());
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

/// Extract a file/directory path from user text.
/// Looks for ~ paths, / paths, and common file extensions.
fn extract_path_from_text(text: &str) -> Option<String> {
    for word in text.split_whitespace() {
        let clean = word.trim_matches(|c: char| c == '?' || c == ',' || c == '"' || c == '\'');
        if clean.starts_with('~') || clean.starts_with('/') {
            return Some(clean.to_string());
        }
        // Match words ending in common file extensions
        if clean.contains('.') && !clean.starts_with("http") {
            let exts = [".md", ".rs", ".json", ".toml", ".yaml", ".yml", ".txt", ".sh", ".py", ".js", ".ts"];
            if exts.iter().any(|e| clean.ends_with(e)) {
                return Some(clean.to_string());
            }
        }
    }
    None
}

/// Shell-escape a path for safe command interpolation.
fn shell_escape(s: &str) -> String {
    if s.contains(' ') || s.contains('(') || s.contains(')') || s.contains('&') {
        format!("'{}'", s.replace('\'', "'\\''"))
    } else {
        s.to_string()
    }
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

// DELETED: extract_memory_intent — memory payload extraction now in intent_router.rs
// Veritas extracts the payload via entity recognition. No keyword parsing needed.

// ═══════════════════════════════════════════════════════════════════
// TOOL ROUTER — The most important optimization in Hydra.
//
// 522 tools × ~60 tokens each = ~31,000 tokens per request.
// With routing: 0-30 tools × ~60 tokens = 0-1,800 tokens.
// That's a 95% reduction for most queries.
// ═══════════════════════════════════════════════════════════════════

/// Select which MCP tools to include in the LLM prompt based on intent.
/// Returns a formatted string of tool names grouped by sister, or empty string
/// if no tools are needed (Tier 0).
fn route_tools_for_prompt(
    intent: &super::intent_router::ClassifiedIntent,
    complexity: &str,
    is_action: bool,
    sisters: &super::super::sisters::cognitive::Sisters,
    user_text: &str,
) -> String {
    use super::intent_router::IntentCategory;

    // Direct-handled intents don't need LLM tools
    if intent.category.has_direct_handler() && intent.confidence >= 0.6 {
        return String::new();
    }

    let mut tools: Vec<String> = Vec::new();

    match intent.category {
        // Memory recall → memory + cognition tools
        IntentCategory::MemoryRecall => {
            tools.extend(sisters.tools_for_sister("memory", &[
                "memory_query", "memory_similar", "memory_temporal",
                "memory_context", "memory_search",
            ]));
            tools.extend(sisters.tools_for_sister("cognition", &[
                "cognition_belief_query", "cognition_belief_list",
            ]));
        }
        // Code tasks → forge + codebase + memory
        IntentCategory::CodeBuild | IntentCategory::CodeFix | IntentCategory::CodeExplain => {
            tools.extend(sisters.tools_for_sister("forge", &[
                "forge_blueprint", "forge_skeleton", "forge_structure",
            ]));
            tools.extend(sisters.tools_for_sister("codebase", &[
                "symbol_lookup", "impact_analysis", "graph_stats",
                "search_semantic", "search_code",
            ]));
            tools.extend(sisters.tools_for_sister("memory", &[
                "memory_query", "memory_context",
            ]));
        }
        // Planning → planning + time + memory
        IntentCategory::PlanningQuery => {
            tools.extend(sisters.tools_for_sister("planning", &[
                "planning_goal", "planning_progress", "planning_decision",
            ]));
            tools.extend(sisters.tools_for_sister("time", &[
                "time_deadline_check", "time_deadline_add", "time_schedule_query",
            ]));
            tools.extend(sisters.tools_for_sister("memory", &[
                "memory_query", "memory_temporal",
            ]));
        }
        // Web/browse → vision tools
        IntentCategory::WebBrowse => {
            tools.extend(sisters.tools_for_sister("vision", &[
                "vision_capture", "vision_query", "vision_ocr",
                "vision_compare", "vision_ground",
            ]));
        }
        // Communication → comm tools
        IntentCategory::Communicate => {
            tools.extend(sisters.tools_for_sister("comm", &[
                "comm_message", "comm_channel", "comm_federation",
                "comm_send", "comm_notify",
            ]));
        }
        // Unknown/Question → route by complexity, with smart detection
        IntentCategory::Unknown | IntentCategory::Question => {
            let lower_input = user_text.to_lowercase();

            // Even simple queries need tools if they mention specific sisters/capabilities
            let needs_identity = lower_input.contains("receipt") || lower_input.contains("prove")
                || lower_input.contains("trust") || lower_input.contains("what did you")
                || lower_input.contains("what have you") || lower_input.contains("last action");
            let needs_time = lower_input.contains("deadline") || lower_input.contains("schedule")
                || lower_input.contains("when") || lower_input.contains("how long");
            let needs_planning = lower_input.contains("goal") || lower_input.contains("plan")
                || lower_input.contains("what should") || lower_input.contains("next step");

            if needs_identity {
                tools.extend(sisters.tools_for_sister("identity", &[
                    "identity_show", "receipt_list",
                ]));
            }
            if needs_time {
                tools.extend(sisters.tools_for_sister("time", &[
                    "time_schedule", "time_deadline", "time_deadline_check",
                ]));
            }
            if needs_planning {
                tools.extend(sisters.tools_for_sister("planning", &[
                    "planning_goal", "planning_progress",
                ]));
            }

            if complexity == "complex" || is_action {
                // Broad tool set for complex unknown intents
                tools.extend(sisters.tools_for_sister("memory", &[
                    "memory_query", "memory_context", "memory_similar",
                ]));
                tools.extend(sisters.tools_for_sister("codebase", &[
                    "symbol_lookup", "impact_analysis", "search_semantic",
                ]));
                tools.extend(sisters.tools_for_sister("forge", &[
                    "forge_blueprint", "forge_skeleton",
                ]));
                tools.extend(sisters.tools_for_sister("vision", &[
                    "vision_capture", "vision_query",
                ]));
                if !needs_identity {
                    tools.extend(sisters.tools_for_sister("identity", &[
                        "identity_show", "receipt_list",
                    ]));
                }
                if !needs_planning {
                    tools.extend(sisters.tools_for_sister("planning", &[
                        "planning_goal", "planning_progress",
                    ]));
                }
                tools.extend(sisters.tools_for_sister("cognition", &[
                    "cognition_model", "cognition_predict",
                ]));
                tools.extend(sisters.tools_for_sister("reality", &[
                    "reality_deployment", "reality_environment",
                ]));
                tools.extend(sisters.tools_for_sister("veritas", &[
                    "veritas_compile", "veritas_verify",
                ]));
                tools.extend(sisters.tools_for_sister("aegis", &[
                    "shadow_simulate", "aegis_validate",
                ]));
                tools.extend(sisters.tools_for_sister("comm", &[
                    "comm_send", "comm_message",
                ]));
                if !needs_time {
                    tools.extend(sisters.tools_for_sister("time", &[
                        "time_schedule", "time_deadline",
                    ]));
                }
                tools.truncate(30);
            }
        }
        // All other categories are direct-handled (no LLM tools needed)
        _ => {}
    }

    format_tool_list(&tools)
}

/// Format a list of tool names into a concise prompt section.
fn format_tool_list(tools: &[String]) -> String {
    if tools.is_empty() {
        return String::new();
    }
    let mut by_prefix: std::collections::BTreeMap<String, Vec<&str>> = std::collections::BTreeMap::new();
    for tool in tools {
        let prefix = tool.split('_').next().unwrap_or("other").to_string();
        by_prefix.entry(prefix).or_default().push(tool);
    }
    let mut out = String::new();
    out.push_str("You can call these MCP tools using <hydra-tool> tags:\n");
    for (prefix, names) in &by_prefix {
        out.push_str(&format!("- {}: {}\n", prefix, names.join(", ")));
    }
    out.push_str(&format!("({} tools available)\n", tools.len()));
    out
}

// ═══════════════════════════════════════════════════════════════════
// Tool router intent detection helpers (lightweight string matching)
// Prefixed with is_tool_ to avoid name conflicts with other detectors.
// ═══════════════════════════════════════════════════════════════════

// DELETED: All is_tool_* keyword functions.
// Tool routing is now driven by intent.category from the Veritas-powered intent router.
// See route_tools_for_prompt() above and cognitive/intent_router.rs.

// ═══════════════════════════════════════════════════════════════════
// Universal slash command handler — works in both Desktop and TUI.
// Returns the shell command to execute, or "__TEXT__:content" for
// static text responses. Returns None if unrecognized.
// ═══════════════════════════════════════════════════════════════════

fn handle_universal_slash_command(input: &str) -> Option<String> {
    let trimmed = input.trim();
    let (cmd, args) = match trimmed.find(' ') {
        Some(pos) => (&trimmed[..pos], trimmed[pos + 1..].trim()),
        None => (trimmed, ""),
    };

    match cmd {
        // ── Developer commands ──
        "/test" => {
            let extra = if args.is_empty() { String::new() } else { format!(" {}", args) };
            Some(detect_project_command("test", &extra))
        }
        "/build" => {
            let extra = if args.is_empty() { String::new() } else { format!(" {}", args) };
            Some(detect_project_command("build", &extra))
        }
        "/run" => {
            let extra = if args.is_empty() { String::new() } else { format!(" {}", args) };
            Some(detect_project_command("run", &extra))
        }
        "/lint" => Some(detect_project_command("lint", "")),
        "/fmt" => Some(detect_project_command("fmt", "")),
        "/bench" => Some(detect_project_command("bench", "")),
        "/doc" => Some(detect_project_command("doc", "")),
        "/deps" => Some(detect_project_command("deps", "")),

        "/files" => {
            // Show project tree (depth 3, max 200 entries)
            Some("find . -maxdepth 3 -not -path '*/target/*' -not -path '*/.git/*' -not -path '*/node_modules/*' -not -path '*/.next/*' | head -200 | sort".to_string())
        }

        "/git" => {
            if args.is_empty() || args == "status" {
                Some("git status && echo '---' && git log --oneline -5".to_string())
            } else if args.starts_with("log") {
                let n = args.strip_prefix("log").unwrap_or("").trim();
                let count = n.parse::<u32>().unwrap_or(10);
                Some(format!("git log --oneline -{}", count))
            } else if args.starts_with("diff") {
                Some(format!("git {}", args))
            } else if args.starts_with("branch") {
                Some("git branch -a".to_string())
            } else {
                Some(format!("git {}", args))
            }
        }

        "/search" => {
            if args.is_empty() {
                Some("__TEXT__:Usage: `/search <pattern>` — searches code for a pattern".to_string())
            } else {
                Some(format!(
                    "grep -rn --include='*.rs' --include='*.ts' --include='*.tsx' --include='*.js' \
                     --include='*.py' --include='*.go' --include='*.toml' --include='*.json' \
                     '{}' . 2>/dev/null | head -50",
                    args.replace('\'', "'\\''")
                ))
            }
        }

        "/symbols" => {
            if args.is_empty() {
                Some("__TEXT__:Usage: `/symbols <file>` — extracts functions and types from a file".to_string())
            } else {
                // Rust-aware symbol extraction
                Some(format!(
                    "grep -n '^\\s*\\(pub\\s\\+\\)\\?\\(fn\\|struct\\|enum\\|trait\\|impl\\|type\\|mod\\|const\\|static\\)\\s' {} 2>/dev/null || \
                     grep -n '\\(function\\|class\\|interface\\|type\\|export\\)' {} 2>/dev/null || \
                     grep -n '\\(def\\|class\\)' {} 2>/dev/null || \
                     echo 'No symbols found in {}'",
                    args, args, args, args
                ))
            }
        }

        // ── System commands ──
        "/sisters" | "/status" => {
            // This will be handled by the sister diagnostic path in the cognitive loop
            // Return None to let it fall through to the normal path
            None
        }

        "/health" => {
            Some("echo '=== System Health ===' && uptime && echo '---' && df -h . && echo '---' && free -h 2>/dev/null || vm_stat 2>/dev/null".to_string())
        }

        "/clear" | "/compact" | "/history" => {
            // UI-only commands — can't handle here, return text hint
            Some("__TEXT__:This command is handled by the UI layer. Use the Desktop or TUI interface directly.".to_string())
        }

        "/model" => {
            if args.is_empty() {
                Some("__TEXT__:Current model is set in Settings. Use `/model <name>` to change.".to_string())
            } else {
                Some(format!("__TEXT__:Model preference noted: **{}**. Change it in Settings to apply.", args))
            }
        }

        "/help" => {
            Some("__TEXT__:## Slash Commands\n\n\
                **Developer:** /test, /build, /run, /files, /git, /search, /symbols, /lint, /fmt, /deps, /bench, /doc\n\
                **System:** /sisters, /health, /status\n\
                **Conversation:** /clear, /compact, /history\n\
                **Settings:** /model, /theme, /voice\n\
                **Control:** /approve, /deny, /kill\n\
                **Debug:** /help, /tokens, /log\n\n\
                Type `/` to see autocomplete suggestions.".to_string())
        }

        _ => None,
    }
}

/// Detect project type and return the appropriate shell command.
/// Uses shell conditionals so it works regardless of cwd at compile time.
fn detect_project_command(action: &str, extra_args: &str) -> String {
    // Use shell conditionals to detect project type at runtime
    match action {
        "test" => format!("if [ -f Cargo.toml ]; then cargo test{}; elif [ -f package.json ]; then npm test; elif [ -f pyproject.toml ]; then python -m pytest; elif [ -f go.mod ]; then go test ./...; else echo 'No project detected'; fi", extra_args),
        "build" => format!("if [ -f Cargo.toml ]; then cargo build{}; elif [ -f package.json ]; then npm run build; elif [ -f go.mod ]; then go build ./...; else echo 'No project detected'; fi", extra_args),
        "run" => format!("if [ -f Cargo.toml ]; then cargo run{}; elif [ -f package.json ]; then npm start; elif [ -f go.mod ]; then go run .; else echo 'No project detected'; fi", extra_args),
        "lint" => "if [ -f Cargo.toml ]; then cargo clippy 2>&1; elif [ -f package.json ]; then npx eslint .; elif [ -f pyproject.toml ]; then python -m ruff check .; else echo 'No project detected'; fi".to_string(),
        "fmt" => "if [ -f Cargo.toml ]; then cargo fmt; elif [ -f package.json ]; then npx prettier --write .; elif [ -f pyproject.toml ]; then python -m black .; else echo 'No project detected'; fi".to_string(),
        "bench" => "if [ -f Cargo.toml ]; then cargo bench; elif [ -f package.json ]; then npm run bench 2>/dev/null || echo 'No bench script'; else echo 'No project detected'; fi".to_string(),
        "doc" => "if [ -f Cargo.toml ]; then cargo doc --open; elif [ -f package.json ]; then npm run docs 2>/dev/null || echo 'No docs script'; else echo 'No project detected'; fi".to_string(),
        "deps" => "if [ -f Cargo.toml ]; then cargo tree --depth 1; elif [ -f package.json ]; then cat package.json | python3 -c \"import sys,json; d=json.load(sys.stdin); [print(f'{k}: {v}') for k,v in {**d.get('dependencies',{}),**d.get('devDependencies',{})}.items()]\"; else echo 'No project detected'; fi".to_string(),
        _ => format!("echo 'Unknown action: {}'", action),
    }
}
