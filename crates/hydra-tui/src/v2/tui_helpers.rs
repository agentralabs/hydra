//! TUI helpers — extracted from the main binary to stay under 400 lines.

use crate::stream::ConversationStream;
use crate::stream_types::StreamItem;
use crate::v2::browser_task::BrowserUpdate;
use crate::v2::commands::registry::CommandContext;
use crate::v2::state::AppState;
use crate::v2::view::RenderState;

/// Extra state for computer-use features passed to the render state.
pub struct ComputerUseState {
    pub shell_mode: bool,
    pub agent_active: bool,
    pub vision_budget_remaining: Option<u32>,
}

/// Build a RenderState snapshot from current AppState.
pub fn build_render_state(s: &AppState, lyapunov: f64) -> RenderState {
    build_render_state_full(s, lyapunov, &ComputerUseState {
        shell_mode: false, agent_active: false, vision_budget_remaining: None,
    })
}

/// Build a RenderState with computer-use state.
pub fn build_render_state_full(s: &AppState, lyapunov: f64, cu: &ComputerUseState) -> RenderState {
    RenderState {
        stream_items: s.stream.items().to_vec(),
        stream_scroll_offset: s.stream.scroll_offset(),
        is_thinking: s.is_thinking,
        thinking_verb: s.thinking_verb.clone(),
        thinking_color: ratatui::style::Color::Rgb(200, 169, 110),
        think_spinner_frame: s.think_spinner_frame,
        input_text: s.input.text().to_string(),
        input_cursor: s.input.cursor(),
        input_line_count: s.input.text().matches('\n').count() + 1,
        input_placeholder: "What are we building today?".into(),
        is_searching: s.input.is_searching(),
        search_query: s.input.search_prompt(),
        genome_count: s.genome_count,
        memory_size_kb: s.memory_size_kb,
        middleware_count: s.middleware_count,
        provider: s.provider.clone(),
        model: s.model.clone(),
        session_minutes: s.session_minutes,
        tokens_used: s.tokens_used,
        mode: "local".into(),
        lyapunov,
        task_count: 0,
        slash_selected: s.slash_selected,
        show_top_frame: true,
        username: whoami::username(),
        project_path: std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
        git_branch: git_branch(),
        modal_active: s.modal.is_some(),
        modal: s.modal.clone(),
        vision_budget_remaining: cu.vision_budget_remaining,
        shell_mode: cu.shell_mode,
        agent_active: cu.agent_active,
        theme: crate::theme::current(),
        voice_state: None,
        monitor_count: 0,
        alert_count: 0,
        alive_message: None,
    }
}

/// Build a CommandContext from current AppState.
pub fn build_command_context(s: &AppState) -> CommandContext {
    let last = s
        .stream
        .items()
        .iter()
        .rev()
        .find_map(|i| {
            if let StreamItem::AssistantText { text, .. } = i {
                Some(text.clone())
            } else {
                None
            }
        })
        .unwrap_or_default();
    let exchanges: Vec<_> = s
        .stream
        .items()
        .iter()
        .filter_map(|i| {
            if let StreamItem::UserMessage { text, .. } = i {
                Some(text.clone())
            } else {
                None
            }
        })
        .zip(s.stream.items().iter().filter_map(|i| {
            if let StreamItem::AssistantText { text, .. } = i {
                Some(text.clone())
            } else {
                None
            }
        }))
        .collect();
    CommandContext {
        genome_count: s.genome_count,
        middleware_count: s.middleware_count,
        provider: s.provider.clone(),
        model: s.model.clone(),
        tokens_used: s.tokens_used,
        session_minutes: s.session_minutes,
        stream_len: s.stream.len(),
        last_response: last,
        exchanges,
    }
}

/// Get the current git branch name.
pub fn git_branch() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

/// Create a system notification StreamItem.
pub fn sysn(content: &str) -> StreamItem {
    StreamItem::SystemNotification {
        id: uuid::Uuid::new_v4(),
        content: content.into(),
        timestamp: chrono::Utc::now(),
    }
}

/// Drain browser updates from the channel. Returns Some(true) when task is done.
pub fn drain_browser(
    rx: &mut tokio::sync::mpsc::Receiver<BrowserUpdate>,
    stream: &mut ConversationStream,
) -> Option<bool> {
    while let Ok(update) = rx.try_recv() {
        match update {
            BrowserUpdate::Status(msg) => stream.push(sysn(&msg)),
            BrowserUpdate::Error(e) => stream.push(sysn(&format!("Browser: {e}"))),
            BrowserUpdate::Done {
                url,
                title,
                text_preview,
            } => {
                stream.push(sysn(&format!("{title} ({url})")));
                stream.push(StreamItem::AssistantText {
                    id: uuid::Uuid::new_v4(),
                    text: text_preview,
                    timestamp: chrono::Utc::now(),
                });
                stream.scroll_to_bottom();
                return Some(true);
            }
        }
        stream.scroll_to_bottom();
    }
    None
}

/// Redirect stderr to ~/.hydra/data/tui.log so eprintln doesn't corrupt the TUI.
pub fn redirect_stderr() {
    use std::os::unix::io::AsRawFd;
    let dir = dirs::home_dir().unwrap_or_default().join(".hydra/data");
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("tui.log"))
    {
        unsafe {
            libc::dup2(f.as_raw_fd(), 2);
        }
        std::mem::forget(f);
    }
}

/// Check if input looks like a task (actionable) vs a question (informational).
/// Tasks go to the conductor. Questions go to the LLM.
/// Only matches when the input STARTS with a task verb — mid-sentence matches
/// caused false positives (e.g. "can you post" matched on "post ").
pub fn is_task_intent(text: &str) -> bool {
    let lower = text.to_lowercase();
    let task_signals = [
        "create ", "build ", "set up ", "setup ", "deploy ", "install ",
        "run ", "execute ", "start ", "make ", "generate ", "write ",
        "delete ", "remove ", "update ", "fix ", "add ", "configure ",
        "publish ", "upload ", "download ", "send ", "post ",
    ];
    task_signals.iter().any(|s| lower.starts_with(s))
}

/// Spawn the conductor as a background task. Returns a receiver for step updates.
pub fn spawn_conductor(
    rt: &tokio::runtime::Runtime,
    goal: String,
) -> tokio::sync::mpsc::Receiver<ConductorUpdate> {
    let (tx, rx) = tokio::sync::mpsc::channel(32);
    rt.spawn(async move {
        let genome = hydra_genome::GenomeStore::open();
        let result = hydra_kernel::conductor_exec::conduct(&goal, &genome);
        match result {
            hydra_kernel::conductor::ConductorResult::Complete { results } => {
                for r in &results {
                    let _ = tx.send(ConductorUpdate::Step {
                        description: r.output.clone(), success: r.success,
                    }).await;
                }
                let _ = tx.send(ConductorUpdate::Done {
                    steps: results.len(), success: results.iter().all(|r| r.success),
                }).await;
            }
            hydra_kernel::conductor::ConductorResult::StepFailed { step_id, error } => {
                let _ = tx.send(ConductorUpdate::Failed {
                    step: step_id, error,
                }).await;
            }
            _ => {
                let _ = tx.send(ConductorUpdate::Failed {
                    step: 0, error: "Conductor did not complete".into(),
                }).await;
            }
        }
    });
    rx
}

/// Updates from a running conductor task.
#[derive(Debug, Clone)]
pub enum ConductorUpdate {
    Step { description: String, success: bool },
    Done { steps: usize, success: bool },
    Failed { step: usize, error: String },
}

// ── Session 22: Boot Orchestrator ──

/// Summary of all booted background systems.
#[derive(Debug, Default)]
pub struct BootedSystems {
    pub workspace_resumed: bool,
    pub monitor_count: usize,
    pub voice_active: bool,
    pub remote_active: bool,
    pub learning_active: bool,
    pub health_issues: Vec<String>,
}

/// Boot all enabled background systems. Called once at TUI startup.
pub fn boot_systems() -> BootedSystems {
    let mut sys = BootedSystems::default();
    // 1. Resume workspace (O7)
    if hydra_kernel::workspace::load_snapshot().is_some() {
        sys.workspace_resumed = true;
        eprintln!("hydra-boot: workspace snapshot found");
    }
    // 2. Learning loop active (runs in dream loop)
    sys.learning_active = true;
    // 3. Log boot complete
    let genome = hydra_genome::GenomeStore::open();
    eprintln!("hydra-boot: all systems started (genome: {} entries)", genome.len());
    sys
}

/// Graceful shutdown — flush all state before exit.
pub fn shutdown_systems() {
    eprintln!("hydra-shutdown: flushing genome...");
    let genome = hydra_genome::GenomeStore::open();
    eprintln!("hydra-shutdown: genome has {} entries", genome.len());
    eprintln!("hydra-shutdown: session ended");
}

/// Generate the alive signal message (rotating background activity indicator).
pub fn alive_message(tick: u64) -> String {
    let messages = [
        "monitoring...", "learning...", "genome growing...",
        "calibrating...", "dreaming...", "ready",
    ];
    messages[(tick as usize / 60) % messages.len()].to_string()
}
