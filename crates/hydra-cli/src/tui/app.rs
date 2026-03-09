use std::collections::VecDeque;
use std::sync::Arc;

use chrono::Local;
use tokio::sync::mpsc;

use crate::client::HydraClient;
use crate::tui::commands::CommandDropdown;
use crate::tui::project::{self, ProjectInfo};
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

    // Project awareness
    pub project_info: Option<ProjectInfo>,

    // Async command execution — child process handle for /test, /build, etc.
    pub running_cmd: Option<RunningCommand>,
}

/// State for a running shell command (async, streamed output).
pub struct RunningCommand {
    pub label: String,
    pub lines: Vec<String>,
    pub rx: mpsc::UnboundedReceiver<CommandEvent>,
    pub start: std::time::Instant,
    pub exit_code: Option<i32>,
}

/// Events from an async command process.
pub enum CommandEvent {
    Line(String),
    Done(Option<i32>),
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

        // Detect project type from working directory
        let detected_project = project::detect_project(std::path::Path::new(&working_dir));

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
            project_info: detected_project,
            running_cmd: None,
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

    /// Periodic tick — refresh animations, drain cognitive updates, advance idle timer.
    pub fn tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);
        self.process_cognitive_updates();
        self.process_running_command();

        // Idle timer — tick_idle every ~1 second (4 ticks × 250ms).
        // Mirrors Desktop's 10s idle timer but at finer granularity.
        if self.tick_count % 4 == 0 {
            self.invention_engine.tick_idle(1);
            if self.tick_count % 40 == 0 {
                // Every ~10 seconds, check for dream insights
                if let Some(dream_text) = self.invention_engine.maybe_dream() {
                    eprintln!("[hydra:tui:dream] {}", &dream_text[..dream_text.len().min(200)]);
                }
            }
        }
    }

    /// Drain output from a running shell command.
    fn process_running_command(&mut self) {
        let (finished, new_lines) = {
            match self.running_cmd.as_mut() {
                Some(cmd) => {
                    let mut lines = Vec::new();
                    let mut done = false;
                    loop {
                        match cmd.rx.try_recv() {
                            Ok(CommandEvent::Line(line)) => {
                                cmd.lines.push(line.clone());
                                lines.push(line);
                            }
                            Ok(CommandEvent::Done(code)) => {
                                cmd.exit_code = code;
                                done = true;
                                break;
                            }
                            Err(mpsc::error::TryRecvError::Empty) => break,
                            Err(mpsc::error::TryRecvError::Disconnected) => {
                                done = true;
                                break;
                            }
                        }
                    }
                    (done, lines)
                }
                None => return,
            }
        };

        // Update the last message with new lines
        if !new_lines.is_empty() || finished {
            if let Some(cmd) = &self.running_cmd {
                let elapsed = cmd.start.elapsed().as_secs_f64();
                let status = if finished {
                    let code = cmd.exit_code.unwrap_or(-1);
                    if code == 0 {
                        format!("completed successfully ({:.1}s)", elapsed)
                    } else {
                        format!("failed with exit code {} ({:.1}s)", code, elapsed)
                    }
                } else {
                    format!("running ({:.1}s)...", elapsed)
                };

                // Build display: show last 40 lines max
                let display_lines: Vec<&str> = cmd.lines.iter()
                    .rev().take(40).collect::<Vec<_>>()
                    .into_iter().rev()
                    .map(|s| s.as_str())
                    .collect();

                let mut content = format!("$ {}\n", cmd.label);
                for line in &display_lines {
                    content.push_str(line);
                    content.push('\n');
                }
                content.push_str(&format!("\n{}", status));

                let timestamp = Local::now().format("%H:%M").to_string();
                // Replace the last system message if it's the command output
                if let Some(last) = self.messages.last_mut() {
                    if last.role == MessageRole::System && last.content.starts_with(&format!("$ {}", cmd.label)) {
                        last.content = content;
                        last.timestamp = timestamp;
                    } else {
                        self.messages.push(Message {
                            role: MessageRole::System,
                            content,
                            timestamp,
                            phase: Some("Act".to_string()),
                        });
                    }
                }
                self.scroll_to_bottom();
            }
        }

        if finished {
            self.running_cmd = None;
        }
    }

    /// Spawn a shell command asynchronously and stream its output.
    fn spawn_command(&mut self, label: &str, program: &str, args: &[&str]) {
        let timestamp = Local::now().format("%H:%M").to_string();

        // Show initial message
        self.messages.push(Message {
            role: MessageRole::System,
            content: format!("$ {}\nStarting...", label),
            timestamp,
            phase: Some("Act".to_string()),
        });
        self.scroll_to_bottom();

        let (tx, rx) = mpsc::unbounded_channel::<CommandEvent>();
        let program = program.to_string();
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let work_dir = self.working_dir.clone();

        tokio::spawn(async move {
            use tokio::io::{AsyncBufReadExt, BufReader};
            use tokio::process::Command;

            let child = Command::new(&program)
                .args(&args)
                .current_dir(&work_dir)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn();

            match child {
                Ok(mut child) => {
                    // Read stdout
                    let stdout = child.stdout.take();
                    let stderr = child.stderr.take();
                    let tx2 = tx.clone();

                    let stdout_task = tokio::spawn(async move {
                        if let Some(stdout) = stdout {
                            let reader = BufReader::new(stdout);
                            let mut lines = reader.lines();
                            while let Ok(Some(line)) = lines.next_line().await {
                                if tx2.send(CommandEvent::Line(line)).is_err() {
                                    break;
                                }
                            }
                        }
                    });

                    let tx3 = tx.clone();
                    let stderr_task = tokio::spawn(async move {
                        if let Some(stderr) = stderr {
                            let reader = BufReader::new(stderr);
                            let mut lines = reader.lines();
                            while let Ok(Some(line)) = lines.next_line().await {
                                if tx3.send(CommandEvent::Line(line)).is_err() {
                                    break;
                                }
                            }
                        }
                    });

                    let _ = stdout_task.await;
                    let _ = stderr_task.await;

                    let status = child.wait().await;
                    let code = status.ok().and_then(|s| s.code());
                    let _ = tx.send(CommandEvent::Done(code));
                }
                Err(e) => {
                    let _ = tx.send(CommandEvent::Line(format!("Failed to start: {}", e)));
                    let _ = tx.send(CommandEvent::Done(Some(1)));
                }
            }
        });

        self.running_cmd = Some(RunningCommand {
            label: label.to_string(),
            lines: Vec::new(),
            rx,
            start: std::time::Instant::now(),
            exit_code: None,
        });
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
                    self.invention_engine.reset_idle();
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
        let args = parts.get(1).copied().unwrap_or("");

        match cmd {
            // ── Developer commands ──
            "/files" => {
                let dir = std::path::Path::new(&self.working_dir);
                let depth: usize = args.parse().unwrap_or(2);
                let entries = project::list_files(dir, depth);
                let mut content = format!("Project files (depth {}):\n\n", depth);
                if entries.is_empty() {
                    content.push_str("  (no files found)");
                } else {
                    for entry in entries.iter().take(200) {
                        content.push_str(entry);
                        content.push('\n');
                    }
                    if entries.len() > 200 {
                        content.push_str(&format!("  ... and {} more", entries.len() - 200));
                    }
                }
                self.messages.push(Message {
                    role: MessageRole::System,
                    content,
                    timestamp: timestamp.to_string(),
                    phase: None,
                });
            }
            "/open" => {
                if args.is_empty() {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: "Usage: /open <file_path>".to_string(),
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                } else {
                    let file_path = if args.starts_with('/') {
                        std::path::PathBuf::from(args)
                    } else {
                        std::path::Path::new(&self.working_dir).join(args)
                    };
                    match project::read_file_with_lines(&file_path) {
                        Ok((content, language)) => {
                            let line_count = content.lines().count();
                            let display: String = content.lines()
                                .take(100)
                                .enumerate()
                                .map(|(i, l)| format!("{:>4} | {}", i + 1, l))
                                .collect::<Vec<_>>()
                                .join("\n");
                            let mut msg = format!(
                                "--- {} ({}, {} lines) ---\n{}",
                                file_path.display(), language, line_count, display
                            );
                            if line_count > 100 {
                                msg.push_str(&format!("\n\n... {} more lines (use /open {} <offset>)", line_count - 100, args));
                            }
                            self.messages.push(Message {
                                role: MessageRole::System,
                                content: msg,
                                timestamp: timestamp.to_string(),
                                phase: None,
                            });
                        }
                        Err(e) => {
                            self.messages.push(Message {
                                role: MessageRole::System,
                                content: e,
                                timestamp: timestamp.to_string(),
                                phase: None,
                            });
                        }
                    }
                }
            }
            "/edit" => {
                if args.is_empty() {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: "Usage: /edit <file_path>".to_string(),
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                } else {
                    let editor = std::env::var("EDITOR")
                        .or_else(|_| std::env::var("VISUAL"))
                        .unwrap_or_else(|_| "vim".to_string());
                    let file_path = if args.starts_with('/') {
                        args.to_string()
                    } else {
                        format!("{}/{}", self.working_dir, args)
                    };
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: format!("Opening {} in {}...", file_path, editor),
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                    // Spawn the editor outside the TUI context
                    // The TUI will need to suspend — for now just show instruction
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: format!("Run: {} {}", editor, file_path),
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                }
            }
            "/search" => {
                if args.is_empty() {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: "Usage: /search <term>".to_string(),
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                } else {
                    // Fast grep search
                    let dir = &self.working_dir;
                    let output = std::process::Command::new("grep")
                        .args(["-rn", "--include=*.rs", "--include=*.ts", "--include=*.tsx",
                               "--include=*.js", "--include=*.py", "--include=*.go",
                               "--include=*.toml", "--include=*.json",
                               "-I", args, dir])
                        .output();
                    match output {
                        Ok(o) if o.status.success() => {
                            let results = String::from_utf8_lossy(&o.stdout);
                            let lines: Vec<&str> = results.lines().take(50).collect();
                            let mut content = format!("Search results for \"{}\":\n\n", args);
                            for line in &lines {
                                // Strip the working dir prefix for cleaner display
                                let display = line.strip_prefix(dir).unwrap_or(line);
                                let display = display.strip_prefix('/').unwrap_or(display);
                                content.push_str(&format!("  {}\n", display));
                            }
                            let total: usize = results.lines().count();
                            if total > 50 {
                                content.push_str(&format!("\n  ... and {} more matches", total - 50));
                            } else {
                                content.push_str(&format!("\n  {} match{}", total, if total == 1 { "" } else { "es" }));
                            }
                            self.messages.push(Message {
                                role: MessageRole::System,
                                content,
                                timestamp: timestamp.to_string(),
                                phase: None,
                            });
                        }
                        Ok(_) => {
                            self.messages.push(Message {
                                role: MessageRole::System,
                                content: format!("No results for \"{}\"", args),
                                timestamp: timestamp.to_string(),
                                phase: None,
                            });
                        }
                        Err(e) => {
                            self.messages.push(Message {
                                role: MessageRole::System,
                                content: format!("Search failed: {}", e),
                                timestamp: timestamp.to_string(),
                                phase: None,
                            });
                        }
                    }
                }
            }
            "/symbols" => {
                if args.is_empty() {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: "Usage: /symbols <file_path>".to_string(),
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                } else {
                    let file_path = if args.starts_with('/') {
                        std::path::PathBuf::from(args)
                    } else {
                        std::path::Path::new(&self.working_dir).join(args)
                    };
                    match std::fs::read_to_string(&file_path) {
                        Ok(content) => {
                            let lang = project::detect_language(&file_path);
                            let mut symbols = Vec::new();
                            for (i, line) in content.lines().enumerate() {
                                let trimmed = line.trim();
                                let is_symbol = match lang.as_str() {
                                    "Rust" => trimmed.starts_with("pub fn ")
                                        || trimmed.starts_with("fn ")
                                        || trimmed.starts_with("pub struct ")
                                        || trimmed.starts_with("struct ")
                                        || trimmed.starts_with("pub enum ")
                                        || trimmed.starts_with("enum ")
                                        || trimmed.starts_with("pub trait ")
                                        || trimmed.starts_with("trait ")
                                        || trimmed.starts_with("impl ")
                                        || trimmed.starts_with("pub type ")
                                        || trimmed.starts_with("pub const ")
                                        || trimmed.starts_with("pub mod ")
                                        || trimmed.starts_with("mod "),
                                    "TypeScript" | "JavaScript" => trimmed.starts_with("function ")
                                        || trimmed.starts_with("export function ")
                                        || trimmed.starts_with("export const ")
                                        || trimmed.starts_with("export class ")
                                        || trimmed.starts_with("class ")
                                        || trimmed.starts_with("interface ")
                                        || trimmed.starts_with("export interface ")
                                        || trimmed.starts_with("type ")
                                        || trimmed.starts_with("export type "),
                                    "Python" => trimmed.starts_with("def ")
                                        || trimmed.starts_with("class ")
                                        || trimmed.starts_with("async def "),
                                    "Go" => trimmed.starts_with("func ")
                                        || trimmed.starts_with("type "),
                                    _ => false,
                                };
                                if is_symbol {
                                    symbols.push(format!("{:>4} | {}", i + 1, trimmed));
                                }
                            }
                            let mut msg = format!("Symbols in {} ({}):\n\n", args, lang);
                            if symbols.is_empty() {
                                msg.push_str("  (no symbols found)");
                            } else {
                                for s in &symbols {
                                    msg.push_str(&format!("  {}\n", s));
                                }
                                msg.push_str(&format!("\n  {} symbol{}", symbols.len(), if symbols.len() == 1 { "" } else { "s" }));
                            }
                            self.messages.push(Message {
                                role: MessageRole::System,
                                content: msg,
                                timestamp: timestamp.to_string(),
                                phase: None,
                            });
                        }
                        Err(e) => {
                            self.messages.push(Message {
                                role: MessageRole::System,
                                content: format!("Cannot read {}: {}", args, e),
                                timestamp: timestamp.to_string(),
                                phase: None,
                            });
                        }
                    }
                }
            }
            "/impact" => {
                if args.is_empty() {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: "Usage: /impact <file_path>".to_string(),
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                } else {
                    // Find what imports/uses this file
                    let basename = std::path::Path::new(args)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or(args);
                    let dir = &self.working_dir;
                    let output = std::process::Command::new("grep")
                        .args(["-rn", "--include=*.rs", "--include=*.ts", "--include=*.tsx",
                               "--include=*.js", "--include=*.py", "--include=*.go",
                               "-I", basename, dir])
                        .output();
                    match output {
                        Ok(o) if o.status.success() => {
                            let results = String::from_utf8_lossy(&o.stdout);
                            // Filter to only import/use lines
                            let imports: Vec<&str> = results.lines()
                                .filter(|l| {
                                    let lower = l.to_lowercase();
                                    lower.contains("use ") || lower.contains("mod ")
                                        || lower.contains("import ") || lower.contains("require(")
                                        || lower.contains("from ")
                                })
                                .take(30)
                                .collect();
                            let mut msg = format!("Impact analysis for \"{}\":\n\n", args);
                            if imports.is_empty() {
                                msg.push_str("  No imports/references found.");
                            } else {
                                for line in &imports {
                                    let display = line.strip_prefix(dir).unwrap_or(line);
                                    let display = display.strip_prefix('/').unwrap_or(display);
                                    msg.push_str(&format!("  {}\n", display));
                                }
                                msg.push_str(&format!("\n  {} reference{}", imports.len(), if imports.len() == 1 { "" } else { "s" }));
                            }
                            self.messages.push(Message {
                                role: MessageRole::System,
                                content: msg,
                                timestamp: timestamp.to_string(),
                                phase: None,
                            });
                        }
                        _ => {
                            self.messages.push(Message {
                                role: MessageRole::System,
                                content: format!("No references found for \"{}\"", args),
                                timestamp: timestamp.to_string(),
                                phase: None,
                            });
                        }
                    }
                }
            }
            "/diff" => {
                let dir = std::path::Path::new(&self.working_dir);
                match project::git_diff(dir) {
                    Some(diff) if !diff.is_empty() => {
                        let lines: Vec<&str> = diff.lines().take(100).collect();
                        let mut content = String::from("Uncommitted changes:\n\n");
                        for line in &lines {
                            content.push_str(line);
                            content.push('\n');
                        }
                        let total = diff.lines().count();
                        if total > 100 {
                            content.push_str(&format!("\n... {} more lines", total - 100));
                        }
                        self.messages.push(Message {
                            role: MessageRole::System,
                            content,
                            timestamp: timestamp.to_string(),
                            phase: None,
                        });
                    }
                    _ => {
                        self.messages.push(Message {
                            role: MessageRole::System,
                            content: "No uncommitted changes.".to_string(),
                            timestamp: timestamp.to_string(),
                            phase: None,
                        });
                    }
                }
            }
            "/git" => {
                let dir = std::path::Path::new(&self.working_dir);
                let subcmd = args.split_whitespace().next().unwrap_or("status");
                match subcmd {
                    "status" | "" => {
                        match project::git_status(dir) {
                            Some(s) => {
                                self.messages.push(Message {
                                    role: MessageRole::System,
                                    content: format!("Git status:\n\n{}", s),
                                    timestamp: timestamp.to_string(),
                                    phase: None,
                                });
                            }
                            None => {
                                self.messages.push(Message {
                                    role: MessageRole::System,
                                    content: "Not a git repository.".to_string(),
                                    timestamp: timestamp.to_string(),
                                    phase: None,
                                });
                            }
                        }
                    }
                    "log" => {
                        let count: usize = args.split_whitespace().nth(1)
                            .and_then(|s| s.parse().ok()).unwrap_or(10);
                        match project::git_log(dir, count) {
                            Some(s) => {
                                self.messages.push(Message {
                                    role: MessageRole::System,
                                    content: format!("Git log (last {}):\n\n{}", count, s),
                                    timestamp: timestamp.to_string(),
                                    phase: None,
                                });
                            }
                            None => {
                                self.messages.push(Message {
                                    role: MessageRole::System,
                                    content: "No git history.".to_string(),
                                    timestamp: timestamp.to_string(),
                                    phase: None,
                                });
                            }
                        }
                    }
                    "commit" => {
                        let msg = args.strip_prefix("commit").unwrap_or("").trim();
                        if msg.is_empty() {
                            self.messages.push(Message {
                                role: MessageRole::System,
                                content: "Usage: /git commit <message>".to_string(),
                                timestamp: timestamp.to_string(),
                                phase: None,
                            });
                        } else {
                            self.spawn_command(
                                &format!("git commit -am \"{}\"", msg),
                                "git",
                                &["commit", "-am", msg],
                            );
                        }
                    }
                    "push" => {
                        self.spawn_command("git push", "git", &["push"]);
                    }
                    "pull" => {
                        self.spawn_command("git pull", "git", &["pull"]);
                    }
                    "branch" => {
                        self.spawn_command("git branch", "git", &["branch", "-a"]);
                    }
                    _ => {
                        self.messages.push(Message {
                            role: MessageRole::System,
                            content: format!("Git subcommands: status, log, commit, push, pull, branch"),
                            timestamp: timestamp.to_string(),
                            phase: None,
                        });
                    }
                }
            }
            "/test" => {
                if let Some(ref info) = self.project_info {
                    let (prog, cmd_args) = info.kind.test_cmd();
                    let label = format!("{} {}", prog, cmd_args.join(" "));
                    self.spawn_command(&label, prog, cmd_args);
                } else {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: "No project detected. Cannot determine test command.".to_string(),
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                }
            }
            "/build" => {
                if let Some(ref info) = self.project_info {
                    let (prog, cmd_args) = info.kind.build_cmd();
                    let label = format!("{} {}", prog, cmd_args.join(" "));
                    self.spawn_command(&label, prog, cmd_args);
                } else {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: "No project detected. Cannot determine build command.".to_string(),
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                }
            }
            "/run" => {
                if let Some(ref info) = self.project_info {
                    let (prog, cmd_args) = info.kind.run_cmd();
                    let label = format!("{} {}", prog, cmd_args.join(" "));
                    self.spawn_command(&label, prog, cmd_args);
                } else {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: "No project detected. Cannot determine run command.".to_string(),
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                }
            }
            "/lint" => {
                if let Some(ref info) = self.project_info {
                    let (prog, cmd_args) = info.kind.lint_cmd();
                    let label = format!("{} {}", prog, cmd_args.join(" "));
                    self.spawn_command(&label, prog, cmd_args);
                } else {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: "No project detected. Cannot determine lint command.".to_string(),
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                }
            }
            "/fmt" => {
                if let Some(ref info) = self.project_info {
                    let (prog, cmd_args) = info.kind.fmt_cmd();
                    let label = format!("{} {}", prog, cmd_args.join(" "));
                    self.spawn_command(&label, prog, cmd_args);
                } else {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: "No project detected. Cannot determine format command.".to_string(),
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                }
            }
            "/deps" => {
                if let Some(ref info) = self.project_info {
                    let (prog, cmd_args) = info.kind.deps_cmd();
                    let label = format!("{} {}", prog, cmd_args.join(" "));
                    self.spawn_command(&label, prog, cmd_args);
                } else {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: "No project detected. Cannot determine deps command.".to_string(),
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                }
            }
            "/bench" => {
                if let Some(ref info) = self.project_info {
                    let (prog, cmd_args) = info.kind.bench_cmd();
                    let label = format!("{} {}", prog, cmd_args.join(" "));
                    self.spawn_command(&label, prog, cmd_args);
                } else {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: "No project detected. Cannot determine bench command.".to_string(),
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                }
            }
            "/doc" => {
                if let Some(ref info) = self.project_info {
                    let (prog, cmd_args) = info.kind.doc_cmd();
                    let label = format!("{} {}", prog, cmd_args.join(" "));
                    self.spawn_command(&label, prog, cmd_args);
                } else {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: "No project detected. Cannot determine doc command.".to_string(),
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                }
            }
            "/deploy" => {
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: "Deploy target not configured. Set HYDRA_DEPLOY_CMD env var or configure in /config.".to_string(),
                    timestamp: timestamp.to_string(),
                    phase: None,
                });
            }
            "/init" => {
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: "Hydra project initialization: coming soon. For now, Hydra auto-detects your project.".to_string(),
                    timestamp: timestamp.to_string(),
                    phase: None,
                });
                if let Some(ref info) = self.project_info {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: format!(
                            "Detected: {} {} ({})\nGit: {}{}",
                            info.kind.icon(), info.name, info.kind.label(),
                            info.git_branch.as_deref().unwrap_or("no git"),
                            match (info.git_ahead, info.git_behind) {
                                (Some(a), Some(b)) if a > 0 || b > 0 => format!(" (+{} -{} from remote)", a, b),
                                _ => String::new(),
                            }
                        ),
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                }
            }

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
                if self.messages.len() > 20 {
                    let drain_count = self.messages.len() - 20;
                    let archived: Vec<Message> = self.messages.drain(0..drain_count).collect();

                    // Archive to file so user can review later with /history archive
                    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
                    let archive_path = format!("{}/.hydra/conversation-archive.log", home);
                    if let Ok(mut file) = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&archive_path)
                    {
                        use std::io::Write;
                        let _ = writeln!(file, "--- Compacted {} messages at {} ---", drain_count, timestamp);
                        for msg in &archived {
                            let role = match msg.role {
                                MessageRole::User => "you",
                                MessageRole::Hydra => "hydra",
                                MessageRole::System => "system",
                            };
                            let _ = writeln!(file, "[{}] {}: {}", msg.timestamp, role, msg.content);
                        }
                        let _ = writeln!(file, "--- End compact ---\n");
                    }

                    self.messages.insert(0, Message {
                        role: MessageRole::System,
                        content: format!(
                            "Compacted {} messages (archived to ~/.hydra/conversation-archive.log).\n\
                             Use /history archive to review.",
                            drain_count
                        ),
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                } else {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: "Nothing to compact (< 20 messages).".to_string(),
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                }
                // Compact conversation history too (keep last 20 turns)
                if self.conversation_history.len() > 20 {
                    let drain_count = self.conversation_history.len() - 20;
                    self.conversation_history.drain(0..drain_count);
                }
                self.scroll_offset = 0;
            }
            "/history" => {
                if args == "archive" {
                    // Show archived/compacted messages from disk
                    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
                    let archive_path = format!("{}/.hydra/conversation-archive.log", home);
                    match std::fs::read_to_string(&archive_path) {
                        Ok(content) => {
                            // Show last 60 lines of archive
                            let tail: Vec<&str> = content.lines().rev().take(60).collect();
                            let display: String = tail.into_iter().rev().collect::<Vec<_>>().join("\n");
                            self.messages.push(Message {
                                role: MessageRole::System,
                                content: format!("Conversation Archive (last 60 lines):\n\n{}", display),
                                timestamp: timestamp.to_string(),
                                phase: None,
                            });
                        }
                        Err(_) => {
                            self.messages.push(Message {
                                role: MessageRole::System,
                                content: "No archive yet. Use /compact to archive older messages.".to_string(),
                                timestamp: timestamp.to_string(),
                                phase: None,
                            });
                        }
                    }
                } else if self.history.is_empty() {
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
                use crate::tui::commands::{COMMANDS, CommandCategory};
                let mut help = String::from("Hydra Commands:\n\n");
                let categories = [
                    ("Developer", CommandCategory::Developer),
                    ("System", CommandCategory::System),
                    ("Conversation", CommandCategory::Conversation),
                    ("Settings", CommandCategory::Settings),
                    ("Control", CommandCategory::Control),
                    ("Debug", CommandCategory::Debug),
                ];
                for (name, cat) in &categories {
                    help.push_str(&format!("  {}:\n", name));
                    for cmd in COMMANDS {
                        if cmd.category == *cat {
                            help.push_str(&format!("    {:<12} {}\n", cmd.name, cmd.description));
                        }
                    }
                    help.push('\n');
                }
                help.push_str("Shortcuts:\n");
                help.push_str("  Ctrl+B  Toggle sidebar\n");
                help.push_str("  Ctrl+K  Kill execution / stop command\n");
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
        // Kill running shell command by dropping the receiver
        if self.running_cmd.is_some() {
            self.running_cmd = None;
        }
        self.messages.push(Message {
            role: MessageRole::System,
            content: "Execution killed.".to_string(),
            timestamp,
            phase: None,
        });
        self.scroll_to_bottom();
    }

    // Scroll — line-based offset from the bottom.
    // 0 = pinned to bottom (auto-scroll), >0 = scrolled up by N lines.
    pub fn scroll_down(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset = self.scroll_offset.saturating_sub(3);
        }
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset += 3;
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn scroll_to_top(&mut self) {
        // Large number — renderer will clamp to actual line count
        self.scroll_offset = usize::MAX / 2;
    }

    pub fn page_up(&mut self) {
        self.scroll_offset += 20;
    }

    pub fn page_down(&mut self) {
        if self.scroll_offset > 20 {
            self.scroll_offset -= 20;
        } else {
            self.scroll_offset = 0;
        }
    }

    /// Whether the conversation is pinned to the bottom (auto-scroll active).
    pub fn is_at_bottom(&self) -> bool {
        self.scroll_offset == 0
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
        // scroll_down on 0 stays at 0 (already at bottom)
        app.scroll_down();
        assert_eq!(app.scroll_offset, 0);
        // scroll_up moves away from bottom
        app.scroll_up();
        assert!(app.scroll_offset > 0);
        // scroll_to_bottom pins back to 0
        app.scroll_to_bottom();
        assert_eq!(app.scroll_offset, 0);
        assert!(app.is_at_bottom());
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
