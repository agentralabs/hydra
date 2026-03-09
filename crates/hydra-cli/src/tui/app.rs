use std::collections::VecDeque;
use std::sync::Arc;

use chrono::Local;
use tokio::sync::mpsc;

use crate::client::HydraClient;
use crate::tui::commands::CommandDropdown;
use hydra_native::cognitive::{
    AgentSpawner, CognitiveLoopConfig, CognitiveUpdate, DecideEngine, InventionEngine,
    run_cognitive_loop,
};
use hydra_native::federation::FederationManager;
use hydra_native::proactive::ProactiveNotifier;
use hydra_native::sisters::SistersHandle;
use hydra_runtime::approval::ApprovalManager;
use hydra_runtime::undo::UndoStack;

/// Sister connection info for sidebar display.
#[derive(Clone, Debug)]
pub struct SisterInfo {
    pub name: String,
    pub short_name: String,
    pub connected: bool,
    pub tool_count: usize,
}

/// A message in the conversation.
#[derive(Clone, Debug)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    pub timestamp: String,
    pub phase: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MessageRole {
    User,
    Hydra,
    System,
}

/// Input mode for the TUI.
#[derive(Clone, Debug, PartialEq)]
pub enum InputMode {
    Normal,
    Insert,
}

/// Focus area for Tab cycling.
#[derive(Clone, Debug, PartialEq)]
pub enum FocusArea {
    Conversation,
    Sidebar,
}

/// Recent task for sidebar display.
#[derive(Clone, Debug)]
pub struct RecentTask {
    pub summary: String,
    pub status: TaskStatus,
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub enum TaskStatus {
    Complete,
    Running,
    Failed,
}

/// Boot state — tracks initialization progress.
#[derive(Clone, Debug, PartialEq)]
pub enum BootState {
    /// Sisters are being spawned — show progress bar.
    Booting,
    /// Fully initialized and ready.
    Ready,
}

/// Boot progress for the progress bar (0.0 to 1.0).
/// Animates smoothly while waiting for sisters to spawn.
#[derive(Clone, Debug)]
pub struct BootProgress {
    pub fraction: f64,
    pub label: String,
}

/// Pending approval state for TUI y/n prompt.
#[derive(Clone, Debug)]
pub struct PendingApproval {
    pub approval_id: Option<String>,
    pub risk_level: String,
    pub action: String,
    pub description: String,
}

/// Main application state.
pub struct App {
    // Core state
    pub should_quit: bool,
    pub input_mode: InputMode,
    pub focus: FocusArea,
    pub sidebar_visible: bool,
    pub boot_state: BootState,

    // Input
    pub input: String,
    pub cursor_pos: usize,
    pub history: Vec<String>,
    pub history_index: Option<usize>,

    // Conversation
    pub messages: Vec<Message>,
    pub scroll_offset: usize,

    // System info
    pub sisters: Vec<SisterInfo>,
    pub connected_count: usize,
    pub total_sisters: usize,
    pub health_pct: u8,
    pub trust_level: String,
    pub memory_facts: u64,
    pub token_avg: u64,
    pub receipt_count: u64,
    pub current_phase: Option<String>,
    pub server_online: bool,
    pub user_name: String,
    pub working_dir: String,
    pub model_name: String,
    pub tool_count: u64,

    // Recent tasks
    pub recent_tasks: VecDeque<RecentTask>,

    // Progress
    pub progress: Option<(String, f64)>, // (label, 0.0..1.0)

    // Tick counter for animations
    pub tick_count: u64,

    // Sisters handle — live connection to all sister processes
    pub sisters_handle: Option<SistersHandle>,

    // HTTP client (fallback for server mode)
    client: HydraClient,

    // Tab completion
    pub completions: Vec<String>,
    pub completion_index: usize,

    // ── Cognitive loop infrastructure (same as Desktop) ──
    pub cognitive_rx: Option<mpsc::UnboundedReceiver<CognitiveUpdate>>,
    decide_engine: Arc<DecideEngine>,
    invention_engine: Arc<InventionEngine>,
    proactive_notifier: Arc<parking_lot::Mutex<ProactiveNotifier>>,
    agent_spawner: Arc<AgentSpawner>,
    undo_stack: Arc<parking_lot::Mutex<UndoStack>>,
    approval_manager: Arc<ApprovalManager>,
    federation_manager: Arc<FederationManager>,
    db: Option<Arc<hydra_db::HydraDb>>,

    // Conversation history for cognitive loop
    pub conversation_history: Vec<(String, String)>,

    // Approval flow — pending approval for y/n prompt
    pub pending_approval: Option<PendingApproval>,

    // Is the cognitive loop currently running?
    pub is_thinking: bool,

    // Slash command dropdown
    pub command_dropdown: CommandDropdown,
}

impl App {
    pub fn new() -> Self {
        let sister_names = vec![
            "Memory", "Identity", "Codebase", "Vision", "Comm", "Contract", "Time",
            "Planning", "Cognition", "Reality",
            "Forge", "Aegis", "Veritas", "Evolve",
        ];

        let sisters: Vec<SisterInfo> = sister_names
            .iter()
            .map(|short| SisterInfo {
                name: format!("Agentic{}", short),
                short_name: short.to_string(),
                connected: false,
                tool_count: 0,
            })
            .collect();

        let working_dir = {
            let prof = hydra_native::profile::load_profile();
            prof.as_ref()
                .and_then(|p| p.working_directory.clone())
                .or_else(|| std::env::current_dir().ok().map(|p| p.display().to_string()))
                .unwrap_or_else(|| "~".to_string())
        };

        // Load user name from shared profile first, fall back to env
        let profile = hydra_native::profile::load_profile();
        let user_name = profile.as_ref()
            .and_then(|p| p.user_name.clone())
            .or_else(|| std::env::var("USER").ok())
            .or_else(|| std::env::var("USERNAME").ok())
            .unwrap_or_else(|| "user".to_string());

        // Initialize DB (same path as Desktop)
        let db = {
            let db_path = std::env::var("HOME")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            let db_path = db_path.join(".hydra").join("hydra.db");
            hydra_db::HydraDb::init(&db_path).ok().map(Arc::new)
        };

        Self {
            should_quit: false,
            input_mode: InputMode::Insert,
            focus: FocusArea::Conversation,
            sidebar_visible: true,
            boot_state: BootState::Booting,

            input: String::new(),
            cursor_pos: 0,
            history: Vec::new(),
            history_index: None,

            messages: Vec::new(),
            scroll_offset: 0,

            sisters,
            connected_count: 0,
            total_sisters: 14,
            health_pct: 0,
            trust_level: "Unknown".to_string(),
            memory_facts: 0,
            token_avg: 0,
            receipt_count: 0,
            current_phase: None,
            server_online: false,
            user_name,
            working_dir,
            model_name: resolve_model_name(),
            tool_count: 740,

            recent_tasks: VecDeque::with_capacity(10),
            progress: None,

            tick_count: 0,
            sisters_handle: None,
            client: HydraClient::new(),

            completions: Vec::new(),
            completion_index: 0,

            // Cognitive loop infrastructure — mirrors Desktop's init
            cognitive_rx: None,
            decide_engine: Arc::new(DecideEngine::new()),
            invention_engine: Arc::new(InventionEngine::new()),
            proactive_notifier: Arc::new(parking_lot::Mutex::new(ProactiveNotifier::new())),
            agent_spawner: Arc::new(AgentSpawner::new(100)),
            undo_stack: Arc::new(parking_lot::Mutex::new(UndoStack::new(100))),
            approval_manager: Arc::new(ApprovalManager::with_default_timeout()),
            federation_manager: Arc::new(FederationManager::new()),
            db,

            conversation_history: Vec::new(),
            pending_approval: None,
            is_thinking: false,

            command_dropdown: CommandDropdown::default(),
        }
    }

    /// Called once after sisters have been spawned. Updates all sidebar state.
    pub fn on_sisters_ready(&mut self, handle: SistersHandle) {
        let all = handle.all_sisters();
        let mut connected = 0usize;
        let mut total_tools = 0u64;

        for (i, (_name, opt)) in all.iter().enumerate() {
            if i < self.sisters.len() {
                let is_connected = opt.is_some();
                let tools = opt.as_ref().map(|c| c.tools.len()).unwrap_or(0);
                self.sisters[i].connected = is_connected;
                self.sisters[i].tool_count = tools;
                if is_connected {
                    connected += 1;
                    total_tools += tools as u64;
                }
            }
        }

        self.connected_count = connected;
        self.tool_count = total_tools;
        self.sisters_handle = Some(handle);
        self.boot_state = BootState::Ready;

        self.health_pct = if self.total_sisters > 0 {
            ((connected as f64 / self.total_sisters as f64) * 100.0) as u8
        } else {
            0
        };

        self.server_online = self.client.health_check();

        let timestamp = Local::now().format("%H:%M").to_string();
        self.messages.push(Message {
            role: MessageRole::System,
            content: format!(
                "{}/{} sisters connected · {} tools available",
                connected, self.total_sisters, total_tools
            ),
            timestamp,
            phase: None,
        });
    }

    /// Periodic tick — refresh animations and drain cognitive updates.
    pub fn tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);
        self.process_cognitive_updates();
    }

    /// Drain all pending CognitiveUpdate events from the channel.
    /// Called every tick (250ms) to keep the TUI responsive.
    fn process_cognitive_updates(&mut self) {
        // Drain into a Vec to avoid borrow issues with self
        let (updates, disconnected) = {
            match self.cognitive_rx.as_mut() {
                Some(rx) => {
                    let mut buf = Vec::new();
                    let mut disc = false;
                    loop {
                        match rx.try_recv() {
                            Ok(update) => buf.push(update),
                            Err(mpsc::error::TryRecvError::Empty) => break,
                            Err(mpsc::error::TryRecvError::Disconnected) => {
                                disc = true;
                                break;
                            }
                        }
                    }
                    (buf, disc)
                }
                None => return,
            }
        };

        // If sender dropped (loop finished/panicked), clean up
        if disconnected && updates.is_empty() {
            self.is_thinking = false;
            self.cognitive_rx = None;
        }

        for update in updates {
            let timestamp = Local::now().format("%H:%M").to_string();

            match update {
                CognitiveUpdate::Phase(p) => {
                    self.current_phase = Some(p);
                }
                CognitiveUpdate::Typing(t) => {
                    self.is_thinking = t;
                }
                CognitiveUpdate::Message { role, content, .. } => {
                    let msg_role = match role.as_str() {
                        "user" => MessageRole::User,
                        "hydra" | "assistant" => MessageRole::Hydra,
                        _ => MessageRole::System,
                    };
                    // Track conversation history for future cognitive loop calls
                    let api_role = if msg_role == MessageRole::User { "user" } else { "assistant" };
                    self.conversation_history.push((api_role.to_string(), content.clone()));

                    self.messages.push(Message {
                        role: msg_role,
                        content,
                        timestamp: timestamp.clone(),
                        phase: self.current_phase.clone(),
                    });
                    self.scroll_to_bottom();
                }
                CognitiveUpdate::ResetIdle => {
                    self.current_phase = None;
                    self.is_thinking = false;
                    self.progress = None;
                }
                CognitiveUpdate::AwaitApproval { approval_id, risk_level, action, description, .. } => {
                    match risk_level.as_str() {
                        "critical" | "high" | "medium" => {
                            // Show approval prompt in TUI
                            self.pending_approval = Some(PendingApproval {
                                approval_id,
                                risk_level: risk_level.clone(),
                                action: action.clone(),
                                description: description.clone(),
                            });
                            self.messages.push(Message {
                                role: MessageRole::System,
                                content: format!(
                                    "[{} RISK] {}\n{}\n\nApprove? (y/n)",
                                    risk_level.to_uppercase(), action, description
                                ),
                                timestamp: timestamp.clone(),
                                phase: Some("Decide".to_string()),
                            });
                            self.scroll_to_bottom();
                        }
                        _ => {
                            // Low/none risk: auto-approve silently
                        }
                    }
                }

                // -- Repair events --
                CognitiveUpdate::RepairStarted { spec, task } => {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: format!("Self-repair started: {} ({})", task, spec),
                        timestamp: timestamp.clone(),
                        phase: Some("Repair".to_string()),
                    });
                    self.scroll_to_bottom();
                }
                CognitiveUpdate::RepairIteration { iteration, passed, total } => {
                    self.progress = Some((
                        format!("Repair iteration {}", iteration),
                        passed as f64 / total.max(1) as f64,
                    ));
                }
                CognitiveUpdate::RepairCompleted { task, status, iterations } => {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: format!("Repair complete: {} — {} ({} iterations)", task, status, iterations),
                        timestamp: timestamp.clone(),
                        phase: Some("Repair".to_string()),
                    });
                    self.progress = None;
                    self.scroll_to_bottom();
                }

                // -- Omniscience events --
                CognitiveUpdate::OmniscienceAnalyzing { phase } => {
                    self.current_phase = Some(format!("Omniscience: {}", phase));
                }
                CognitiveUpdate::OmniscienceGapFound { description, severity, category } => {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: format!("Gap [{}|{}]: {}", severity, category, description),
                        timestamp: timestamp.clone(),
                        phase: Some("Omniscience".to_string()),
                    });
                    self.scroll_to_bottom();
                }
                CognitiveUpdate::OmniscienceScanComplete { gaps_found, specs_generated, health_score } => {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: format!(
                            "Omniscience scan complete: {} gaps, {} specs, {:.0}% health",
                            gaps_found, specs_generated, health_score * 100.0
                        ),
                        timestamp: timestamp.clone(),
                        phase: Some("Omniscience".to_string()),
                    });
                    self.scroll_to_bottom();
                }

                // -- Plan events --
                CognitiveUpdate::PlanInit { goal, steps } => {
                    let mut plan_msg = format!("Plan: {}\n", goal);
                    for (i, step) in steps.iter().enumerate() {
                        plan_msg.push_str(&format!("  {}. {}\n", i + 1, step));
                    }
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: plan_msg,
                        timestamp: timestamp.clone(),
                        phase: Some("Think".to_string()),
                    });
                    self.scroll_to_bottom();
                }
                CognitiveUpdate::PlanStepComplete { index, duration_ms } => {
                    let dur = duration_ms.map(|ms| format!(" ({}ms)", ms)).unwrap_or_default();
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: format!("Step {} complete{}", index + 1, dur),
                        timestamp: timestamp.clone(),
                        phase: Some("Act".to_string()),
                    });
                }

                // -- Belief events --
                CognitiveUpdate::BeliefsLoaded { count, summary } => {
                    if count > 0 {
                        self.messages.push(Message {
                            role: MessageRole::System,
                            content: format!("{} beliefs loaded: {}", count, summary),
                            timestamp: timestamp.clone(),
                            phase: Some("Perceive".to_string()),
                        });
                    }
                }

                // -- Celebration --
                CognitiveUpdate::Celebrate(msg) => {
                    self.messages.push(Message {
                        role: MessageRole::Hydra,
                        content: msg,
                        timestamp: timestamp.clone(),
                        phase: None,
                    });
                    self.scroll_to_bottom();
                }

                // -- Sisters called (show in sidebar-style) --
                CognitiveUpdate::SistersCalled { sisters } => {
                    if !sisters.is_empty() {
                        self.messages.push(Message {
                            role: MessageRole::System,
                            content: format!("Sisters: {}", sisters.join(", ")),
                            timestamp: timestamp.clone(),
                            phase: self.current_phase.clone(),
                        });
                    }
                }

                // -- Proactive alerts --
                CognitiveUpdate::ProactiveAlert { title, message, priority } => {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: format!("[{}] {} — {}", priority, title, message),
                        timestamp: timestamp.clone(),
                        phase: None,
                    });
                    self.scroll_to_bottom();
                }

                // -- Ghost Cursor events → text-based progress in TUI --
                CognitiveUpdate::CursorMove { label, .. } => {
                    if let Some(label) = label {
                        self.progress = Some((label, 0.5));
                    }
                }
                CognitiveUpdate::CursorTyping { active } => {
                    if active {
                        self.progress = Some(("Typing...".to_string(), 0.5));
                    } else {
                        self.progress = None;
                    }
                }
                CognitiveUpdate::CursorVisibility { visible } => {
                    if !visible {
                        self.progress = None;
                    }
                }

                // -- Silently handled / no TUI equivalent --
                CognitiveUpdate::IconState(_)
                | CognitiveUpdate::PhaseStatuses(_)
                | CognitiveUpdate::PlanClear
                | CognitiveUpdate::PlanStepStart(_)
                | CognitiveUpdate::EvidenceClear
                | CognitiveUpdate::EvidenceMemory { .. }
                | CognitiveUpdate::EvidenceCode { .. }
                | CognitiveUpdate::TimelineClear
                | CognitiveUpdate::SidebarCompleteTask(_)
                | CognitiveUpdate::SuggestMode(_)
                | CognitiveUpdate::SettingsApplied { .. }
                | CognitiveUpdate::TokenUsage { .. }
                | CognitiveUpdate::StreamChunk { .. }
                | CognitiveUpdate::StreamComplete
                | CognitiveUpdate::UndoStatus { .. }
                | CognitiveUpdate::SkillCrystallized { .. }
                | CognitiveUpdate::ReflectionInsight { .. }
                | CognitiveUpdate::CompressionApplied { .. }
                | CognitiveUpdate::DreamInsight { .. }
                | CognitiveUpdate::ShadowValidation { .. }
                | CognitiveUpdate::PredictionResult { .. }
                | CognitiveUpdate::PatternEvolved { .. }
                | CognitiveUpdate::TemporalStored { .. }
                | CognitiveUpdate::CursorClick
                | CognitiveUpdate::CursorModeChange { .. }
                | CognitiveUpdate::CursorPaused { .. }
                | CognitiveUpdate::McpSkillsDiscovered { .. }
                | CognitiveUpdate::FederationSync { .. }
                | CognitiveUpdate::FederationDelegated { .. }
                | CognitiveUpdate::RepairCheckResult { .. }
                | CognitiveUpdate::OmniscienceSpecGenerated { .. }
                | CognitiveUpdate::OmniscienceValidation { .. }
                | CognitiveUpdate::BeliefUpdated { .. }
                => {}
            }
        }
    }

    /// Refresh sister status from live handle.
    pub fn refresh_status(&mut self) {
        if let Some(ref handle) = self.sisters_handle {
            let all = handle.all_sisters();
            let mut connected = 0usize;
            for (i, (_name, opt)) in all.iter().enumerate() {
                if i < self.sisters.len() {
                    self.sisters[i].connected = opt.is_some();
                    if opt.is_some() {
                        connected += 1;
                    }
                }
            }
            self.connected_count = connected;
            self.health_pct = if self.total_sisters > 0 {
                ((connected as f64 / self.total_sisters as f64) * 100.0) as u8
            } else {
                0
            };
        }
        self.server_online = self.client.health_check();
    }

    /// Submit user input.
    pub fn submit_input(&mut self, input: &str) {
        // Allow escape commands even during approval prompt
        if input.starts_with('/') {
            match input {
                "/kill" | "/quit" | "/exit" | "/q" | "/deny" | "/n" | "/approve" | "/y" | "/clear" => {
                    // These commands work during approval
                    let timestamp = Local::now().format("%H:%M").to_string();
                    if !input.is_empty() {
                        self.history.push(input.to_string());
                    }
                    self.history_index = None;
                    self.handle_slash_command(input, &timestamp);
                    return;
                }
                _ => {}
            }
        }

        // Handle approval response
        if self.pending_approval.is_some() {
            let lower = input.trim().to_lowercase();
            if lower == "y" || lower == "yes" {
                self.handle_approval(true);
            } else if lower == "n" || lower == "no" {
                self.handle_approval(false);
            } else {
                let timestamp = Local::now().format("%H:%M").to_string();
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: "Type 'y' to approve, 'n' to deny, or /kill to cancel.".to_string(),
                    timestamp,
                    phase: None,
                });
                self.scroll_to_bottom();
            }
            return;
        }

        if !input.is_empty() {
            self.history.push(input.to_string());
            // Cap history to prevent unbounded growth
            if self.history.len() > 500 {
                self.history.drain(0..self.history.len() - 500);
            }
        }
        self.history_index = None;

        let timestamp = Local::now().format("%H:%M").to_string();

        if input.starts_with('/') {
            self.handle_slash_command(input, &timestamp);
            return;
        }

        self.messages.push(Message {
            role: MessageRole::User,
            content: input.to_string(),
            timestamp: timestamp.clone(),
            phase: None,
        });

        // Track in conversation history
        self.conversation_history.push(("user".to_string(), input.to_string()));

        self.execute_intent(input, &timestamp);
        self.scroll_to_bottom();
    }

    /// Handle approval decision (y/n).
    fn handle_approval(&mut self, approved: bool) {
        let timestamp = Local::now().format("%H:%M").to_string();
        if let Some(approval) = self.pending_approval.take() {
            if approved {
                if let Some(id) = &approval.approval_id {
                    let _ = self.approval_manager.submit_decision(
                        id,
                        hydra_runtime::approval::ApprovalDecision::Approved,
                    );
                }
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: format!("Approved: {}", approval.action),
                    timestamp,
                    phase: Some("Decide".to_string()),
                });
            } else {
                if let Some(id) = &approval.approval_id {
                    let _ = self.approval_manager.submit_decision(
                        id,
                        hydra_runtime::approval::ApprovalDecision::Denied {
                            reason: "User denied".into(),
                        },
                    );
                }
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: format!("Denied: {}", approval.action),
                    timestamp,
                    phase: Some("Decide".to_string()),
                });
            }
        }
        self.scroll_to_bottom();
    }

    fn handle_slash_command(&mut self, input: &str, timestamp: &str) {
        let parts: Vec<&str> = input.splitn(2, ' ').collect();
        let cmd = parts[0];
        let _args = parts.get(1).copied().unwrap_or("");

        match cmd {
            // ── System ──
            "/sisters" => {
                self.refresh_status();
                let mut lines = format!(
                    "Sisters: {}/{}\n",
                    self.connected_count, self.total_sisters
                );
                lines.push_str("┌──────────────┬────────────┬───────┐\n");
                lines.push_str("│ Sister       │ Status     │ Tools │\n");
                lines.push_str("├──────────────┼────────────┼───────┤\n");
                for s in &self.sisters {
                    let dot = if s.connected { "●" } else { "○" };
                    let status = if s.connected { "connected" } else { "offline" };
                    lines.push_str(&format!(
                        "│ {} {:<10} │ {:<10} │ {:>5} │\n",
                        dot, s.short_name, status, s.tool_count
                    ));
                }
                lines.push_str("└──────────────┴────────────┴───────┘");
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: lines,
                    timestamp: timestamp.to_string(),
                    phase: None,
                });
            }
            "/fix" => {
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: "Running sister repair...".to_string(),
                    timestamp: timestamp.to_string(),
                    phase: Some("Repair".to_string()),
                });
                // Route through cognitive loop same as "fix sisters"
                self.execute_intent("repair offline sisters", timestamp);
            }
            "/scan" => {
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: "Starting Omniscience scan...".to_string(),
                    timestamp: timestamp.to_string(),
                    phase: Some("Omniscience".to_string()),
                });
                self.execute_intent("scan all repos for gaps", timestamp);
            }
            "/repair" => {
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: "Running self-repair specs...".to_string(),
                    timestamp: timestamp.to_string(),
                    phase: Some("Repair".to_string()),
                });
                self.execute_intent("run self repair", timestamp);
            }
            "/memory" => {
                let msg = format!(
                    "Memory Stats:\n  Facts: {}\n  Tokens avg: {}\n  Receipts: {}",
                    self.memory_facts, self.token_avg, self.receipt_count
                );
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: msg,
                    timestamp: timestamp.to_string(),
                    phase: None,
                });
            }
            "/goals" => {
                self.execute_intent("show active planning goals", timestamp);
            }
            "/beliefs" => {
                self.execute_intent("show current belief store", timestamp);
            }
            "/receipts" => {
                let msg = format!("Recent receipts: {}", self.receipt_count);
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: msg,
                    timestamp: timestamp.to_string(),
                    phase: None,
                });
            }
            "/health" => {
                self.refresh_status();
                let mode = if self.sisters_handle.is_some() {
                    "Local (embedded)"
                } else if self.server_online {
                    "Server"
                } else {
                    "Offline"
                };
                let msg = format!(
                    "System Health Dashboard:\n\
                     ├─ Mode: {}\n\
                     ├─ Sisters: {}/{} ({:.0}%)\n\
                     ├─ Tools: {}\n\
                     ├─ Trust: {}\n\
                     ├─ Memory: {} facts\n\
                     ├─ Tokens: {} avg\n\
                     ├─ Receipts: {}\n\
                     └─ Model: {}",
                    mode,
                    self.connected_count, self.total_sisters, self.health_pct,
                    self.tool_count,
                    self.trust_level,
                    self.memory_facts,
                    self.token_avg,
                    self.receipt_count,
                    self.model_name,
                );
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: msg,
                    timestamp: timestamp.to_string(),
                    phase: None,
                });
            }
            "/status" => {
                self.refresh_status();
                let mode = if self.sisters_handle.is_some() {
                    "Local (embedded)"
                } else if self.server_online {
                    "Server"
                } else {
                    "Offline"
                };
                let msg = format!(
                    "Status: {} · Sisters {}/{} · {}% · {} tools · Trust: {}",
                    mode,
                    self.connected_count,
                    self.total_sisters,
                    self.health_pct,
                    self.tool_count,
                    self.trust_level
                );
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: msg,
                    timestamp: timestamp.to_string(),
                    phase: None,
                });
            }

            // ── Conversation ──
            "/clear" => {
                self.messages.clear();
                self.scroll_offset = 0;
                self.conversation_history.clear();
            }
            "/compact" => {
                // Keep only last 20 messages + conversation history
                if self.messages.len() > 20 {
                    let drain_count = self.messages.len() - 20;
                    self.messages.drain(0..drain_count);
                    self.messages.insert(0, Message {
                        role: MessageRole::System,
                        content: format!("Compacted {} messages.", drain_count),
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                }
                // Also compact conversation history (keep last 20 turns)
                if self.conversation_history.len() > 20 {
                    let drain_count = self.conversation_history.len() - 20;
                    self.conversation_history.drain(0..drain_count);
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: format!("Compacted. {} history turns retained.", self.conversation_history.len()),
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                } else if self.messages.len() <= 20 {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: "Nothing to compact.".to_string(),
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                }
            }
            "/history" => {
                if self.history.is_empty() {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: "No command history.".to_string(),
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                } else {
                    let mut lines = String::from("Command History:\n");
                    for (i, h) in self.history.iter().rev().take(20).enumerate() {
                        lines.push_str(&format!("  {}. {}\n", i + 1, h));
                    }
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: lines,
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                }
            }

            // ── Settings ──
            "/model" => {
                let msg = format!(
                    "Current model: {}\n\
                     Available: Opus 4.6, Sonnet 4.6, Haiku 4.5\n\
                     Set with: /model <name> or HYDRA_MODEL env var",
                    self.model_name
                );
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: msg,
                    timestamp: timestamp.to_string(),
                    phase: None,
                });
            }
            "/voice" => {
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: "Voice input: not yet available in TUI mode.".to_string(),
                    timestamp: timestamp.to_string(),
                    phase: None,
                });
            }
            "/sidebar" => {
                self.sidebar_visible = !self.sidebar_visible;
            }
            "/theme" => {
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: "Theme: Hydra Dark (default). More themes coming soon.".to_string(),
                    timestamp: timestamp.to_string(),
                    phase: None,
                });
            }
            "/config" => {
                let msg = format!(
                    "Configuration:\n\
                     ├─ Model: {}\n\
                     ├─ Sidebar: {}\n\
                     ├─ Trust: {}\n\
                     ├─ Working dir: {}\n\
                     └─ User: {}",
                    self.model_name,
                    if self.sidebar_visible { "visible" } else { "hidden" },
                    self.trust_level,
                    self.working_dir,
                    self.user_name,
                );
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: msg,
                    timestamp: timestamp.to_string(),
                    phase: None,
                });
            }

            // ── Control ──
            "/trust" => {
                let msg = format!("Trust level: {}", self.trust_level);
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: msg,
                    timestamp: timestamp.to_string(),
                    phase: None,
                });
            }
            "/approve" | "/y" => {
                if self.pending_approval.is_some() {
                    self.handle_approval(true);
                } else {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: "No pending approval.".to_string(),
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                }
            }
            "/deny" | "/n" => {
                if self.pending_approval.is_some() {
                    self.handle_approval(false);
                } else {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: "No pending approval.".to_string(),
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                }
            }
            "/kill" => {
                self.kill_current();
            }

            // ── Debug ──
            "/log" => {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
                let log_path = format!("{}/.hydra/hydra-tui.log", home);
                match std::fs::read_to_string(&log_path) {
                    Ok(content) => {
                        let tail: String = content.lines().rev().take(30).collect::<Vec<_>>()
                            .into_iter().rev().collect::<Vec<_>>().join("\n");
                        self.messages.push(Message {
                            role: MessageRole::System,
                            content: format!("Last 30 log lines:\n{}", tail),
                            timestamp: timestamp.to_string(),
                            phase: None,
                        });
                    }
                    Err(_) => {
                        self.messages.push(Message {
                            role: MessageRole::System,
                            content: format!("No log file at {}", log_path),
                            timestamp: timestamp.to_string(),
                            phase: None,
                        });
                    }
                }
            }
            "/debug" => {
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: "Debug mode: not yet implemented.".to_string(),
                    timestamp: timestamp.to_string(),
                    phase: None,
                });
            }
            "/tokens" => {
                let msg = format!("Token usage: {} avg per turn", self.token_avg);
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: msg,
                    timestamp: timestamp.to_string(),
                    phase: None,
                });
            }

            // ── Help ──
            "/help" | "/?" => {
                use crate::tui::commands::COMMANDS;
                let mut help = String::from("Hydra Commands:\n\n");
                let mut current_cat = String::new();
                for cmd in COMMANDS {
                    let cat = format!("{:?}", cmd.category);
                    if cat != current_cat {
                        if !current_cat.is_empty() {
                            help.push('\n');
                        }
                        current_cat = cat;
                    }
                    help.push_str(&format!("  {:<12} {}\n", cmd.name, cmd.description));
                }
                help.push_str("\nShortcuts:\n");
                help.push_str("  Ctrl+B  Toggle sidebar\n");
                help.push_str("  Ctrl+K  Kill execution\n");
                help.push_str("  Ctrl+L  Refresh status\n");
                help.push_str("  Ctrl+C  Exit\n");
                help.push_str("  Esc     Normal mode (j/k scroll)\n");
                help.push_str("  Tab     Autocomplete command\n");
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: help,
                    timestamp: timestamp.to_string(),
                    phase: None,
                });
            }

            // ── Exit ──
            "/quit" | "/exit" | "/q" => {
                self.should_quit = true;
            }

            _ => {
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: format!("Unknown command: {}. Type /help for commands.", cmd),
                    timestamp: timestamp.to_string(),
                    phase: None,
                });
            }
        }
        self.scroll_to_bottom();
    }

    fn execute_intent(&mut self, input: &str, timestamp: &str) {
        // If we have a server, route through it
        if self.server_online {
            let body = serde_json::json!({ "intent": input });
            if let Ok(resp) = self.client.post("/api/conversations", &body) {
                let content = resp
                    .get("response")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Intent received. Processing...");
                self.messages.push(Message {
                    role: MessageRole::Hydra,
                    content: content.to_string(),
                    timestamp: timestamp.to_string(),
                    phase: Some("Act".to_string()),
                });
                return;
            }
        }

        // Sisters connected locally → spawn cognitive loop (same as Desktop)
        if let Some(ref sisters) = self.sisters_handle {
            if self.connected_count > 0 {
                // Drop previous channel to signal old loop to stop
                self.cognitive_rx = None;

                let (tx, rx) = mpsc::unbounded_channel::<CognitiveUpdate>();
                self.cognitive_rx = Some(rx);
                self.is_thinking = true;

                let task_id = uuid::Uuid::new_v4().to_string();
                let loop_config = CognitiveLoopConfig {
                    text: input.to_string(),
                    anthropic_key: std::env::var("ANTHROPIC_API_KEY").unwrap_or_default(),
                    openai_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
                    google_key: std::env::var("GOOGLE_API_KEY").unwrap_or_default(),
                    model: std::env::var("HYDRA_MODEL").unwrap_or_else(|_| "claude-sonnet-4-20250514".to_string()),
                    user_name: self.user_name.clone(),
                    task_id,
                    history: self.conversation_history.clone(),
                    anthropic_oauth_token: None,
                };

                let sisters_handle = Some(sisters.clone());
                let decide = self.decide_engine.clone();
                let undo = Some(self.undo_stack.clone());
                let inv = Some(self.invention_engine.clone());
                let notifier = Some(self.proactive_notifier.clone());
                let spawner = Some(self.agent_spawner.clone());
                let approval = Some(self.approval_manager.clone());
                let db = self.db.clone();
                let fed = Some(self.federation_manager.clone());

                tokio::spawn(async move {
                    run_cognitive_loop(
                        loop_config,
                        sisters_handle,
                        tx,
                        decide,
                        undo,
                        inv,
                        notifier,
                        spawner,
                        approval,
                        db,
                        fed,
                    )
                    .await;
                });

                return;
            }
        }

        // No sisters, no server
        self.messages.push(Message {
            role: MessageRole::Hydra,
            content: format!(
                "Received: \"{}\"\n\n\
                 [No sisters connected]\n\
                 Sister binaries not found in ~/.local/bin/\n\
                 Install sisters or run `hydra serve` for server mode.",
                input
            ),
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    /// Kill current running execution.
    pub fn kill_current(&mut self) {
        let timestamp = Local::now().format("%H:%M").to_string();
        if self.server_online {
            let _ = self.client.post("/api/system/kill", &serde_json::json!({}));
        }
        self.progress = None;
        self.current_phase = None;
        self.is_thinking = false;
        self.cognitive_rx = None; // Drop the channel — loop will stop when tx fails
        self.messages.push(Message {
            role: MessageRole::System,
            content: "Execution killed.".to_string(),
            timestamp,
            phase: None,
        });
        self.scroll_to_bottom();
    }

    // Scroll
    pub fn scroll_down(&mut self) {
        if self.scroll_offset < self.messages.len().saturating_sub(1) {
            self.scroll_offset += 1;
        }
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.messages.len().saturating_sub(1);
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    // History navigation
    pub fn history_prev(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let idx = match self.history_index {
            Some(i) => if i > 0 { i - 1 } else { 0 },
            None => self.history.len() - 1,
        };
        self.history_index = Some(idx);
        self.input = self.history[idx].clone();
        self.cursor_pos = self.input.len();
    }

    pub fn history_next(&mut self) {
        if let Some(idx) = self.history_index {
            if idx + 1 < self.history.len() {
                self.history_index = Some(idx + 1);
                self.input = self.history[idx + 1].clone();
                self.cursor_pos = self.input.len();
            } else {
                self.history_index = None;
                self.input.clear();
                self.cursor_pos = 0;
            }
        }
    }

    pub fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            FocusArea::Conversation => FocusArea::Sidebar,
            FocusArea::Sidebar => FocusArea::Conversation,
        };
    }

    /// Update the command dropdown filter based on current input.
    pub fn update_dropdown(&mut self) {
        self.command_dropdown.update_filter(&self.input);
    }

    /// Tab completion — if dropdown is visible, select the highlighted command.
    /// Otherwise fall back to cycling through matches.
    pub fn tab_complete(&mut self) {
        // If dropdown is showing, accept the selected command
        if self.command_dropdown.visible {
            if let Some(name) = self.command_dropdown.selected_command() {
                self.input = name.to_string();
                self.cursor_pos = self.input.len();
                self.command_dropdown.close();
            }
            return;
        }

        if self.input.is_empty() {
            return;
        }

        // Legacy cycle-through for non-dropdown cases
        if self.completions.is_empty() {
            use crate::tui::commands::COMMANDS;
            self.completions = COMMANDS
                .iter()
                .filter(|c| c.name.starts_with(&self.input))
                .map(|c| c.name.to_string())
                .collect();
            self.completion_index = 0;
        }

        if !self.completions.is_empty() {
            let completion = self.completions[self.completion_index].clone();
            self.input = completion;
            self.cursor_pos = self.input.len();
            self.completion_index = (self.completion_index + 1) % self.completions.len();
        }
    }
}

/// Resolve model display name from HYDRA_MODEL env var.
pub fn resolve_model_name() -> String {
    let raw = std::env::var("HYDRA_MODEL").unwrap_or_default();
    // Map model IDs to friendly names
    match raw.as_str() {
        s if s.contains("opus") => "Opus 4.6".to_string(),
        s if s.contains("sonnet") => "Sonnet 4.6".to_string(),
        s if s.contains("haiku") => "Haiku 4.5".to_string(),
        s if s.contains("gpt-4") => format!("GPT-4 ({})", s),
        s if s.contains("gemini") => format!("Gemini ({})", s),
        s if !s.is_empty() => s.to_string(),
        _ => "Sonnet 4.6".to_string(), // default model
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_creation() {
        let app = App::new();
        assert_eq!(app.sisters.len(), 14);
        assert_eq!(app.total_sisters, 14);
        assert!(!app.should_quit);
        assert_eq!(app.input_mode, InputMode::Insert);
        assert_eq!(app.boot_state, BootState::Booting);
        // Cognitive loop infrastructure initialized
        assert!(app.cognitive_rx.is_none());
        assert!(app.pending_approval.is_none());
        assert!(!app.is_thinking);
    }

    #[test]
    fn history_navigation() {
        let mut app = App::new();
        app.history.push("first".to_string());
        app.history.push("second".to_string());

        app.history_prev();
        assert_eq!(app.input, "second");

        app.history_prev();
        assert_eq!(app.input, "first");

        app.history_next();
        assert_eq!(app.input, "second");
    }

    #[test]
    fn scroll_bounds() {
        let mut app = App::new();
        app.scroll_up();
        assert_eq!(app.scroll_offset, 0);
        app.scroll_down();
        assert_eq!(app.scroll_offset, 0);
    }

    #[test]
    fn sidebar_toggle() {
        let mut app = App::new();
        assert!(app.sidebar_visible);
        app.sidebar_visible = !app.sidebar_visible;
        assert!(!app.sidebar_visible);
    }

    #[test]
    fn approval_flow() {
        let mut app = App::new();
        app.pending_approval = Some(PendingApproval {
            approval_id: Some("test-123".to_string()),
            risk_level: "high".to_string(),
            action: "rm -rf /tmp/test".to_string(),
            description: "Delete test directory".to_string(),
        });

        // Submitting "y" should clear approval
        app.submit_input("y");
        assert!(app.pending_approval.is_none());

        // Verify approval message was added
        assert!(app.messages.iter().any(|m| m.content.contains("Approved")));
    }
}
