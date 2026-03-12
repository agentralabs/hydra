use std::collections::VecDeque;
use std::sync::Arc;

use tokio::sync::mpsc;

use crate::client::HydraClient;
use crate::tui::commands::CommandDropdown;
use crate::tui::project::{self, ProjectInfo};
use hydra_native::cognitive::{
    AgentSpawner, CognitiveUpdate, DecideEngine, InventionEngine,
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

/// Permission mode — cycles with Shift+Tab (Claude Code parity).
#[derive(Clone, Debug, PartialEq)]
pub enum PermissionMode {
    /// Every action requires approval.
    Normal,
    /// File edits auto-approved, bash still prompts.
    AutoAccept,
    /// Plan but don't execute.
    Plan,
}

impl PermissionMode {
    /// Cycle to the next mode.
    pub fn next(&self) -> Self {
        match self {
            Self::Normal => Self::AutoAccept,
            Self::AutoAccept => Self::Plan,
            Self::Plan => Self::Normal,
        }
    }

    /// Display label for the input prompt (empty for Normal).
    pub fn label(&self) -> &'static str {
        match self {
            Self::Normal => "",
            Self::AutoAccept => "[Auto-Accept]",
            Self::Plan => "[Plan]",
        }
    }
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
    pub permission_mode: PermissionMode,

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
    pub(crate) client: HydraClient,

    // Tab completion
    pub completions: Vec<String>,
    pub completion_index: usize,

    // ── Cognitive loop infrastructure (same as Desktop) ──
    pub cognitive_rx: Option<mpsc::UnboundedReceiver<CognitiveUpdate>>,
    pub(crate) decide_engine: Arc<DecideEngine>,
    pub(crate) invention_engine: Arc<InventionEngine>,
    pub(crate) proactive_notifier: Arc<parking_lot::Mutex<ProactiveNotifier>>,
    pub(crate) agent_spawner: Arc<AgentSpawner>,
    pub(crate) undo_stack: Arc<parking_lot::Mutex<UndoStack>>,
    pub(crate) approval_manager: Arc<ApprovalManager>,
    pub(crate) federation_manager: Arc<FederationManager>,
    pub(crate) db: Option<Arc<hydra_db::HydraDb>>,

    // Conversation history for cognitive loop
    pub conversation_history: Vec<(String, String)>,

    // Approval flow — pending approval for y/n prompt
    pub pending_approval: Option<PendingApproval>,

    // Challenge phrase gate for CRITICAL actions (Visual Overhaul spec)
    pub challenge_phrase: Option<String>,
    pub challenge_action: Option<String>,

    // Double-Esc detection (§6.1: Esc+Esc opens rewind menu)
    pub last_esc_tick: u64,

    // Is the cognitive loop currently running?
    pub is_thinking: bool,
    /// Contextual status for what Hydra is doing right now (replaces generic "Thinking...")
    pub thinking_status: String,
    /// Phase 3, C5.3: Elapsed time for current thinking phase (milliseconds)
    pub thinking_elapsed_ms: u64,

    // Slash command dropdown
    pub command_dropdown: CommandDropdown,

    // Project awareness
    pub project_info: Option<ProjectInfo>,

    // Async command execution — child process handle for /test, /build, etc.
    pub running_cmd: Option<RunningCommand>,

    // Tool output expand/collapse toggle (Visual Overhaul: ctrl+o)
    pub tool_output_expanded: bool,

    // PR status indicator (spec §11) — updated periodically
    pub pr_status: Option<PrStatus>,
    pub pr_check_tick: u64,
}

/// PR status for the footer indicator (spec §11).
#[derive(Clone, Debug)]
pub struct PrStatus {
    pub number: u32,
    pub state: PrState,
    pub url: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PrState {
    Open,
    Approved,
    ChangesRequested,
    ReviewRequested,
    Merged,
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
            sidebar_visible: false,
            boot_state: BootState::Booting,
            permission_mode: PermissionMode::Normal,

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
            challenge_phrase: None,
            challenge_action: None,
            last_esc_tick: 0,
            is_thinking: false,
            thinking_status: String::new(),
            thinking_elapsed_ms: 0,

            command_dropdown: CommandDropdown::default(),
            project_info: detected_project,
            running_cmd: None,
            tool_output_expanded: false,
            pr_status: None,
            pr_check_tick: 0,
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
#[path = "app_tests.rs"]
mod tests;
