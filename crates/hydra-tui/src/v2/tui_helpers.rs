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
pub fn build_render_state(s: &AppState, lyapunov: f64, cache: &mut crate::v2::cache::FrameCache) -> RenderState {
    build_render_state_full(s, lyapunov, &ComputerUseState {
        shell_mode: false, agent_active: false, vision_budget_remaining: None,
    }, cache)
}

/// Build a RenderState with computer-use state.
pub fn build_render_state_full(
    s: &AppState, lyapunov: f64, cu: &ComputerUseState,
    cache: &mut crate::v2::cache::FrameCache,
) -> RenderState {
    // Refresh cached values (only recomputes if TTL expired)
    cache.refresh();
    RenderState {
        stream_items: s.stream.items_shared(),
        stream_scroll_offset: s.stream.scroll_offset(),
        is_thinking: s.is_thinking,
        thinking_verb: std::sync::Arc::from(s.thinking_verb.as_str()),
        thinking_color: ratatui::style::Color::Rgb(200, 169, 110),
        think_spinner_frame: s.think_spinner_frame,
        input_text: s.input.text().to_string(),
        input_cursor: s.input.cursor(),
        input_line_count: s.input.text().matches('\n').count() + 1,
        input_placeholder: std::sync::Arc::from(s.input_placeholder.as_str()),
        is_searching: s.input.is_searching(),
        search_query: s.input.search_prompt(),
        genome_count: s.genome_count,
        memory_size_kb: s.memory_size_kb,
        middleware_count: s.middleware_count,
        provider: std::sync::Arc::from(s.provider.as_str()),
        model: std::sync::Arc::from(s.model.as_str()),
        session_minutes: s.session_minutes,
        tokens_used: s.tokens_used,
        mode: "local".into(),
        lyapunov,
        task_count: 0,
        slash_selected: s.slash_selected,
        show_top_frame: true,
        username: std::sync::Arc::from(cache.username.get().as_str()),
        project_path: std::sync::Arc::from(cache.project_path.get().as_str()),
        git_branch: std::sync::Arc::from(cache.git_branch.get().as_str()),
        modal_active: s.modal.is_some(),
        modal: s.modal.clone(),
        vision_budget_remaining: cu.vision_budget_remaining,
        shell_mode: cu.shell_mode,
        agent_active: cu.agent_active,
        theme: crate::theme::current(),
        voice_state: s.voice_state.clone(),
        monitor_count: 0,
        alert_count: 0,
        alive_message: Some(alive_message(s.session_minutes)),
        presence_state: None,
        new_while_scrolled: s.stream.new_while_scrolled(),
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
    let task_verbs = [
        // Creation & building
        "create ", "build ", "set up ", "setup ", "deploy ", "install ",
        "make ", "generate ", "write ", "add ", "configure ", "init ",
        // Execution & running
        "run ", "execute ", "start ", "launch ", "spawn ", "trigger ",
        // Modification & management
        "delete ", "remove ", "update ", "fix ", "edit ", "modify ", "change ",
        "restart ", "stop ", "kill ", "pause ", "resume ",
        // File & system operations
        "publish ", "upload ", "download ", "send ", "post ", "move ", "copy ",
        "rename ", "save ", "export ", "import ", "backup ",
        // Browsing & navigation
        "open ", "browse ", "navigate ", "go to ", "visit ", "search for ",
        // Monitoring & checking
        "check ", "scan ", "monitor ", "test ", "verify ", "audit ", "inspect ",
        "analyze ", "diagnose ",
        // Desktop & application control
        "click ", "type ", "drag ", "scroll ", "press ", "switch to ",
        "close ", "minimize ", "maximize ", "focus ",
        // Learning & research
        "learn ", "research ", "study ", "find ",
    ];
    task_verbs.iter().any(|s| lower.starts_with(s))
        // Also catch "can you <verb>" and "please <verb>" patterns
        || lower.starts_with("can you ") && task_verbs.iter().any(|s| lower[8..].starts_with(s))
        || lower.starts_with("please ") && task_verbs.iter().any(|s| lower[7..].starts_with(s))
        || lower.starts_with("could you ") && task_verbs.iter().any(|s| lower[10..].starts_with(s))
        || lower.starts_with("i want you to ") && task_verbs.iter().any(|s| lower[14..].starts_with(s))
        || lower.starts_with("i need you to ") && task_verbs.iter().any(|s| lower[14..].starts_with(s))
}

/// Open a URL in the user's visible default browser (not headless CDP).
pub fn open_visible_browser(url: &str) -> Result<(), String> {
    let cmd = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
    std::process::Command::new(cmd).arg(url).spawn()
        .map_err(|e| format!("Failed to open browser: {e}"))?;
    Ok(())
}

/// Extract a URL from user input text (explicit URLs or bare domains).
pub fn extract_url(text: &str) -> Option<String> {
    for word in text.split_whitespace() {
        let w = word.trim_end_matches(|c: char| ".,;:!?)\"'".contains(c));
        if w.starts_with("http://") || w.starts_with("https://") { return Some(w.into()); }
        if w.contains('.') && !w.starts_with('.') && w.len() > 3
            && !w.ends_with(".rs") && !w.ends_with(".md") && !w.ends_with(".toml") {
            return Some(format!("https://{w}"));
        }
    }
    None
}

/// Spawn the conductor as a background task. Returns a receiver for step updates.
pub fn spawn_conductor(
    rt: &tokio::runtime::Runtime,
    goal: String,
) -> tokio::sync::mpsc::Receiver<ConductorUpdate> {
    let (tx, rx) = tokio::sync::mpsc::channel(32);
    rt.spawn(async move {
        let _ = tx.send(ConductorUpdate::Info { message: "Mining assumptions...".into() }).await;
        let genome = hydra_genome::GenomeStore::open();
        let result = hydra_kernel::conductor_exec::conduct(&goal, &genome);
        let _ = tx.send(ConductorUpdate::Info { message: "Evaluating quality...".into() }).await;
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
    Info { message: String },
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
    /// Boot phase messages for TUI display.
    pub boot_log: Vec<String>,
}

/// Boot all enabled background systems. Called once at TUI startup.
pub fn boot_systems() -> BootedSystems {
    let mut sys = BootedSystems::default();
    sys.boot_log.push("Verifying constitution...".into());
    // Self-preservation health check on startup (O23)
    let mut integrity = hydra_kernel::integrity::IntegrityMonitor::new();
    let report = integrity.check();
    if !report.is_healthy() {
        for issue in &report.issues { sys.health_issues.push(issue.clone()); }
        sys.boot_log.push(format!("Health: {} issues detected", sys.health_issues.len()));
    } else {
        sys.boot_log.push("Health check passed".into());
    }
    // Resume workspace (O7)
    if hydra_kernel::workspace::load_snapshot().is_some() {
        sys.workspace_resumed = true;
        sys.boot_log.push("Workspace resumed from last session".into());
    }
    sys.learning_active = true;
    let genome = hydra_genome::GenomeStore::open();
    sys.boot_log.push(format!("Genome: {} entries loaded", genome.len()));
    sys.boot_log.push("All systems ready".into());
    eprintln!("hydra-boot: all systems started (genome: {} entries, health: {} issues)",
        genome.len(), sys.health_issues.len());
    sys
}

/// Graceful shutdown — flush all state before exit.
pub fn shutdown_systems() {
    // Flush genome to DB
    let genome = hydra_genome::GenomeStore::open();
    eprintln!("hydra-shutdown: genome flushed ({} entries)", genome.len());
    // Local backup on clean exit (O23)
    match hydra_kernel::backup::create_backup() {
        Ok(r) => eprintln!("hydra-shutdown: backup {} files", r.files_copied),
        Err(e) => eprintln!("hydra-shutdown: backup skipped: {e}"),
    }
    eprintln!("hydra-shutdown: session ended cleanly");
}

/// Generate the alive signal message (rotating background activity indicator).
pub fn alive_message(tick: u64) -> String {
    let messages = [
        "monitoring...", "learning...", "genome growing...",
        "calibrating...", "dreaming...", "ready",
    ];
    messages[(tick as usize / 60) % messages.len()].to_string()
}

/// Tick the thinking spinner — verb rotation + frame advancement.
/// Extracted from main binary for 400-line compliance.
pub fn tick_spinner(state: &mut AppState) {
    if state.is_thinking {
        state.think_spinner_frame = state.think_spinner_frame.wrapping_add(1);
        if state.think_spinner_frame % 44 == 0 {
            let contexts = crate::verb::VerbContext::all();
            let ctx = &contexts[(state.think_spinner_frame / 44) % contexts.len()];
            let alts = ctx.alternatives();
            state.thinking_verb = alts[(state.think_spinner_frame / 11) % alts.len()].to_string();
        }
        state.touch(); // Dirty flag: spinner changed
    }
}

/// Check if an LLM error is transient and worth retrying.
pub fn is_transient_error(error: &str) -> bool {
    let e = error.to_lowercase();
    e.contains("rate") || e.contains("429") || e.contains("529")
        || e.contains("timeout") || e.contains("connection") || e.contains("overloaded")
}

/// Suggest a context-aware input placeholder based on last response (GAP 6).
pub fn suggest_placeholder(response: &str) -> String {
    let lower = response.to_lowercase();
    if lower.contains("error") || lower.contains("failed") || lower.contains("panic") {
        "How should we fix this?".into()
    } else if lower.contains("```") {
        "Want me to test this?".into()
    } else if lower.contains("deployed") || lower.contains("published") {
        "Check the status?".into()
    } else if lower.contains("created") || lower.contains("wrote") || lower.contains("added") {
        "What's next?".into()
    } else if lower.contains("found") || lower.contains("searched") {
        "Want me to dig deeper?".into()
    } else {
        "What are we building today?".into()
    }
}

/// Re-export greeting from top_frame (the actual implementation lives there).
pub fn greeting_items(model: &str, genome_count: usize, mw_count: usize) -> Vec<crate::stream_types::StreamItem> {
    crate::v2::view::top_frame::greeting_items(model, genome_count, mw_count)
}
