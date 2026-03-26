//! Hydra TUI v3 — Event loop shell. Submit/streaming logic in v2/submit.rs.

use std::io;
use std::time::{Duration, Instant};

use crossterm::event;
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use hydra_kernel::engine::CognitiveLoop;
use hydra_tui::stream_types::StreamItem;
use hydra_tui::v2::commands;
use hydra_tui::v2::dispatch::dispatch_event;
use hydra_tui::v2::state::{AppState, reduce};
use hydra_tui::v2::submit;
use hydra_tui::v2::tui_helpers::{
    build_render_state_full, boot_systems, shutdown_systems,
    ComputerUseState, drain_browser, redirect_stderr, sysn,
};
use hydra_tui::v2::view;

const TICK_RATE: Duration = Duration::from_millis(50);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if hydra_kernel::first_run::is_first_run() {
        hydra_kernel::first_run::run_wizard();
    }
    hydra_desktop::deps::preflight();
    let config = hydra_tui::config::HydraConfig::load();
    hydra_tui::theme::init(hydra_tui::theme::Theme::by_name(&config.tui.theme));
    redirect_stderr();
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;
    terminal.clear()?;
    let result = run(&mut terminal);
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    if let Err(e) = result { eprintln!("hydra-tui: {e}"); }
    Ok(())
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<(), Box<dyn std::error::Error>> {
    let pacer_chars = (15.0 * hydra_tui::config::HydraConfig::load().tui.pacer_speed).max(1.0) as usize;
    let mut cognitive = CognitiveLoop::new();
    let (provider, model) = (cognitive.llm_provider().to_string(), cognitive.llm_model().to_string());
    let mut state = AppState::new(&provider, &model);
    state.genome_count = cognitive.genome_len();
    state.middleware_count = cognitive.middleware_count();
    state.memory_size_kb = dirs::home_dir()
        .and_then(|h| std::fs::metadata(h.join(".hydra/data/hydra.amem")).ok())
        .map(|m| m.len() / 1024).unwrap_or(0);
    let booted = boot_systems();
    state.boot_complete = true;
    state.health_issues = booted.health_issues.len();
    for msg in &booted.boot_log { state.stream.push(sysn(&format!("◌ {msg}"))); }
    for issue in &booted.health_issues { state.stream.push(sysn(&format!("[Health] {issue}"))); }
    for item in hydra_tui::v2::morning_brief::generate_briefing(state.genome_count) { state.stream.push(item); }
    if let Some(exs) = hydra_kernel::conversation_store::ConversationStore::load_latest() {
        for ex in exs.iter().rev().take(3).rev() {
            state.stream.push(StreamItem::UserMessage { id: uuid::Uuid::new_v4(), text: ex.input.clone(), timestamp: chrono::Utc::now() });
            state.stream.push(StreamItem::AssistantText { id: uuid::Uuid::new_v4(), text: ex.response.clone(), timestamp: chrono::Utc::now() });
    } }
    let registry = commands::build_registry();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2).enable_all().build().expect("tokio runtime");
    let mut voice_loop = hydra_voice::VoiceLoop::new();
    let (companion_channel, companion_endpoint) = hydra_signals::create_companion_channel();
    let mut companion_service = hydra_companion::CompanionService::new(companion_endpoint);
    let mut dream_subs = hydra_kernel::loop_dream::DreamSubsystems::new();
    let mut ambient_subs = hydra_kernel::loop_ambient::AmbientSubsystems::new();
    let metabolism = hydra_metabolism::MetabolismMonitor::new();
    let (boot_time, mut last_dream, mut last_ambient, mut last_user_input) = (Instant::now(), Instant::now(), Instant::now(), Instant::now());
    type ChunkRx = tokio::sync::mpsc::Receiver<hydra_kernel::loop_::llm_stream::StreamChunk>;
    let mut active_stream: Option<ChunkRx> = None;
    let mut streaming_text = String::new();
    let mut streaming_display_cursor: usize = 0;
    let mut streaming_prepared: Option<hydra_kernel::engine::PreparedCycle> = None;
    let mut browser_rx: Option<tokio::sync::mpsc::Receiver<hydra_tui::v2::browser_task::BrowserUpdate>> = None;
    let mut agent_rx: Option<tokio::sync::mpsc::Receiver<hydra_tui::v2::agent_task::AgentUpdate>> = None;
    let vision_provider = hydra_tui::v2::agent_task::create_vision_provider();
    let mut shell_rx: Option<tokio::sync::mpsc::Receiver<hydra_tui::v2::shell_task::ShellUpdate>> = None;
    let mut shell_mode = false;
    let mut conductor_rx: Option<tokio::sync::mpsc::Receiver<hydra_tui::v2::tui_helpers::ConductorUpdate>> = None;
    let mut frame_cache = hydra_tui::v2::cache::FrameCache::new();
    loop {
        state.session_minutes = boot_time.elapsed().as_secs() / 60;
        state.lyapunov = metabolism.tracker().current().unwrap_or(0.42);
        state.active_tasks = if conductor_rx.is_some() { 1 } else { 0 };
        hydra_tui::v2::tui_helpers::tick_spinner(&mut state);
        let cu = ComputerUseState { shell_mode, agent_active: agent_rx.is_some(), vision_budget_remaining: None };
        let rs = build_render_state_full(&state, state.lyapunov, &cu, &mut frame_cache);
        terminal.draw(|frame| { view::render(frame, &rs); })?;
        // Event polling
        if event::poll(TICK_RATE)? {
            let ev = event::read()?;
            last_user_input = Instant::now();
            for action in dispatch_event(&ev, state.modal_open(), active_stream.is_some(), state.input.text().is_empty()) {
                submit::handle_action(action, &mut state, &mut active_stream, &mut streaming_text,
                    &mut streaming_display_cursor, &mut streaming_prepared, &mut cognitive,
                    &registry, &companion_channel, &mut voice_loop, &rt,
                    &mut browser_rx, &mut agent_rx, &vision_provider,
                    &mut shell_rx, &mut shell_mode, &mut conductor_rx);
            }
        }
        // Drain channels
        submit::drain_llm_stream(&mut active_stream, &mut streaming_text, &mut streaming_display_cursor,
            &mut streaming_prepared, &mut state, &mut cognitive, pacer_chars, &mut voice_loop);
        if let Some(rx) = &mut browser_rx {
            if let Some(_done) = drain_browser(rx, &mut state.stream) { browser_rx = None; }
        }
        if let Some(rx) = &mut agent_rx {
            if hydra_tui::v2::agent_task::drain_agent(rx, &mut state.stream) { agent_rx = None; }
        }
        if let Some(rx) = &mut shell_rx {
            if hydra_tui::v2::shell_task::drain_shell(rx, &mut state.stream) { shell_rx = None; }
        }
        if let Some(rx) = &mut conductor_rx {
            use hydra_tui::v2::tui_helpers::ConductorUpdate;
            while let Ok(update) = rx.try_recv() {
                match update {
                    ConductorUpdate::Step { description, success } => {
                        state.stream.push(sysn(&format!("  {} {description}", if success { "✓" } else { "✗" })));
                    }
                    ConductorUpdate::Info { message } => { state.stream.push(sysn(&format!("  ◌ {message}"))); }
                    ConductorUpdate::Done { steps, success } => {
                        state.stream.push(sysn(&format!("Conductor: {steps} steps, {}", if success { "all passed" } else { "some failed" })));
                        conductor_rx = None; break;
                    }
                    ConductorUpdate::Failed { step, error } => {
                        state.stream.push(sysn(&format!("Conductor failed at step {}: {error}", step + 1)));
                        conductor_rx = None; break;
                    }
                }
                state.stream.scroll_to_bottom();
            }
        }
        // Voice auto-submit
        let mut voice_auto_submit = false;
        for action in hydra_tui::v2::bridge_voice::poll_voice(&mut voice_loop) {
            if matches!(&action, hydra_tui::v2::action::Action::Voice(hydra_tui::v2::action::VoiceAction::FinalTranscript(_))) {
                voice_auto_submit = true;
            }
            reduce(&mut state, action);
        }
        if voice_auto_submit && !state.input.text().is_empty() {
            submit::handle_action(hydra_tui::v2::action::Action::Input(hydra_tui::v2::action::InputAction::Submit),
                &mut state, &mut active_stream, &mut streaming_text,
                &mut streaming_display_cursor, &mut streaming_prepared, &mut cognitive,
                &registry, &companion_channel, &mut voice_loop, &rt,
                &mut browser_rx, &mut agent_rx, &vision_provider,
                &mut shell_rx, &mut shell_mode, &mut conductor_rx);
        }
        for a in hydra_tui::v2::bridge_companion::poll_companion(&companion_channel) { reduce(&mut state, a); }
        companion_service.tick();
        // Background loops
        if last_dream.elapsed() >= Duration::from_secs(5) {
            dream_subs.idle_secs = last_user_input.elapsed().as_secs();
            let r = hydra_kernel::loop_dream::cycle_with_subsystems(
                &hydra_kernel::state::HydraState::initial(), Some(&mut dream_subs));
            if r.did_work {
                let mut parts = Vec::new();
                if r.genome_entries_created > 0 { parts.push(format!("+{} genome", r.genome_entries_created)); }
                if r.beliefs_consolidated > 0 { parts.push(format!("+{} beliefs", r.beliefs_consolidated)); }
                if r.predictions_rehearsed > 0 { parts.push(format!("{} predictions", r.predictions_rehearsed)); }
                if !parts.is_empty() {
                    state.stream.push(StreamItem::DreamNotification { id: uuid::Uuid::new_v4(),
                        content: parts.join(" | "), timestamp: chrono::Utc::now() });
                }
            }
            last_dream = Instant::now();
        }
        if last_ambient.elapsed() >= Duration::from_secs(10) {
            hydra_kernel::loop_ambient::tick_with_subsystems(
                &hydra_kernel::state::HydraState::initial(), 10.0, Some(&mut ambient_subs));
            last_ambient = Instant::now();
        }
        if state.should_quit { break; }
    }
    voice_loop.stop_listening();
    shutdown_systems();
    rt.shutdown_timeout(Duration::from_secs(3));
    Ok(())
}
