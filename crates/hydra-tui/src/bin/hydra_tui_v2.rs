//! Hydra TUI v2 — Event-Action-State architecture.

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
use hydra_tui::v2::action::*;
use hydra_tui::v2::commands;
use hydra_tui::v2::dispatch::dispatch_event;
use hydra_tui::v2::state::{AppState, reduce};
use hydra_tui::v2::tui_helpers::{
    build_command_context, build_render_state_full, ComputerUseState, drain_browser, redirect_stderr, sysn,
};
use hydra_tui::v2::view;

const TICK_RATE: Duration = Duration::from_millis(50);

fn main() -> Result<(), Box<dyn std::error::Error>> {
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

fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let pacer_chars = (15.0 * hydra_tui::config::HydraConfig::load().tui.pacer_speed).max(1.0) as usize;
    let mut cognitive = CognitiveLoop::new();
    let (provider, model) = (cognitive.llm_provider().to_string(), cognitive.llm_model().to_string());
    let mut state = AppState::new(&provider, &model);
    state.genome_count = cognitive.genome_len();
    state.middleware_count = cognitive.middleware_count();
    state.memory_size_kb = dirs::home_dir()
        .and_then(|h| std::fs::metadata(h.join(".hydra/data/hydra.amem")).ok())
        .map(|m| m.len() / 1024).unwrap_or(0);
    state.boot_complete = true;
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
    let mut metabolism = hydra_metabolism::MetabolismMonitor::new();
    let (boot_time, mut last_dream) = (Instant::now(), Instant::now());
    type ChunkRx = tokio::sync::mpsc::Receiver<hydra_kernel::loop_::llm_stream::StreamChunk>;
    let mut active_stream: Option<ChunkRx> = None;
    let mut streaming_text = String::new();
    let mut streaming_display_cursor: usize = 0;
    let mut streaming_prepared: Option<hydra_kernel::engine::PreparedCycle> = None;
    type BrowserRx = tokio::sync::mpsc::Receiver<hydra_tui::v2::browser_task::BrowserUpdate>;
    let mut browser_rx: Option<BrowserRx> = None;
    type AgentRx = tokio::sync::mpsc::Receiver<hydra_tui::v2::agent_task::AgentUpdate>;
    let mut agent_rx: Option<AgentRx> = None;
    let vision_provider = hydra_tui::v2::agent_task::create_vision_provider();
    type ShellRx = tokio::sync::mpsc::Receiver<hydra_tui::v2::shell_task::ShellUpdate>;
    let mut shell_rx: Option<ShellRx> = None;
    let mut shell_mode = false;
    type ConductorRx = tokio::sync::mpsc::Receiver<hydra_tui::v2::tui_helpers::ConductorUpdate>;
    let mut conductor_rx: Option<ConductorRx> = None;
    loop {
        state.session_minutes = boot_time.elapsed().as_secs() / 60;
        tick_spinner(&mut state);
        let cu_state = ComputerUseState {
            shell_mode, agent_active: agent_rx.is_some(), vision_budget_remaining: None,
        };
        let render_state = build_render_state_full(&state, metabolism.tracker().current().unwrap_or(0.42), &cu_state);
        terminal.draw(|frame| { view::render(frame, &render_state); })?;
        if event::poll(TICK_RATE)? {
            let ev = event::read()?;
            for action in dispatch_event(&ev, state.modal_open()) {
                handle_action(action, &mut state, &mut active_stream, &mut streaming_text,
                    &mut streaming_display_cursor, &mut streaming_prepared, &mut cognitive,
                    &registry, &companion_channel, &mut voice_loop, &rt,
                    &mut browser_rx, &mut agent_rx, &vision_provider,
                    &mut shell_rx, &mut shell_mode, &mut conductor_rx);
            }
        }
        drain_llm_stream(&mut active_stream, &mut streaming_text, &mut streaming_display_cursor,
            &mut streaming_prepared, &mut state, &mut cognitive, pacer_chars);
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
                        let icon = if success { "✓" } else { "✗" };
                        state.stream.push(sysn(&format!("  {icon} {description}")));
                    }
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
        for action in hydra_tui::v2::bridge_voice::poll_voice(&mut voice_loop) { reduce(&mut state, action); }
        for a in hydra_tui::v2::bridge_companion::poll_companion(&companion_channel) { reduce(&mut state, a); }
        companion_service.tick();
        if last_dream.elapsed() >= Duration::from_secs(5) {
            let r = hydra_kernel::loop_dream::cycle_with_subsystems(
                &hydra_kernel::state::HydraState::initial(), Some(&mut dream_subs));
            if r.genome_entries_created > 0 {
                state.stream.push(StreamItem::DreamNotification { id: uuid::Uuid::new_v4(),
                    content: format!("{} genome entries from experience", r.genome_entries_created),
                    timestamp: chrono::Utc::now() });
            }
            last_dream = Instant::now();
        }
        if state.should_quit { break; }
    }
    Ok(())
}

fn tick_spinner(state: &mut AppState) {
    if state.is_thinking {
        state.think_spinner_frame = state.think_spinner_frame.wrapping_add(1);
        if state.think_spinner_frame % 44 == 0 {
            let contexts = hydra_tui::verb::VerbContext::all();
            let ctx = &contexts[(state.think_spinner_frame / 44) % contexts.len()];
            let alts = ctx.alternatives();
            state.thinking_verb = alts[(state.think_spinner_frame / 11) % alts.len()].to_string();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_action(
    action: Action, state: &mut AppState,
    active_stream: &mut Option<tokio::sync::mpsc::Receiver<hydra_kernel::loop_::llm_stream::StreamChunk>>,
    streaming_text: &mut String, streaming_display_cursor: &mut usize,
    streaming_prepared: &mut Option<hydra_kernel::engine::PreparedCycle>,
    cognitive: &mut CognitiveLoop, registry: &commands::registry::CommandRegistry,
    companion_channel: &hydra_signals::CompanionChannel, voice_loop: &mut hydra_voice::VoiceLoop,
    rt: &tokio::runtime::Runtime,
    browser_rx: &mut Option<tokio::sync::mpsc::Receiver<hydra_tui::v2::browser_task::BrowserUpdate>>,
    agent_rx: &mut Option<tokio::sync::mpsc::Receiver<hydra_tui::v2::agent_task::AgentUpdate>>,
    vision: &Option<std::sync::Arc<dyn hydra_browser::VisionProvider>>,
    shell_rx: &mut Option<tokio::sync::mpsc::Receiver<hydra_tui::v2::shell_task::ShellUpdate>>,
    shell_mode: &mut bool,
    conductor_rx: &mut Option<tokio::sync::mpsc::Receiver<hydra_tui::v2::tui_helpers::ConductorUpdate>>,
) {
    // Shell mode: treat input as shell command
    if *shell_mode {
        if let Action::Input(InputAction::Submit) = &action {
            let text = state.input.submit();
            if text == "/exit" || text == "/shell" {
                *shell_mode = false;
                state.stream.push(sysn("Shell mode off."));
                state.stream.scroll_to_bottom();
                return;
            }
            if !text.is_empty() {
                state.stream.push(sysn(&format!("$ {text}")));
                *shell_rx = Some(hydra_tui::v2::shell_task::spawn(rt, text));
            }
            return;
        }
    }
    // Slash menu navigation
    let slash_active = state.input.text().starts_with('/') && !state.is_thinking;
    if slash_active {
        match &action {
            Action::Input(InputAction::HistoryUp) => { state.slash_selected = state.slash_selected.saturating_sub(1); return; }
            Action::Input(InputAction::HistoryDown) => { state.slash_selected += 1; return; }
            Action::Input(InputAction::InsertChar(_)) | Action::Input(InputAction::Backspace) => { state.slash_selected = 0; }
            _ => {}
        }
    }
    match &action {
        Action::System(SystemAction::Quit) if active_stream.is_some() => {
            *active_stream = None; streaming_text.clear(); *streaming_display_cursor = 0;
            state.is_thinking = false; state.stream.push(sysn("Interrupted.")); state.stream.scroll_to_bottom();
        }
        Action::Voice(VoiceAction::Toggle) => {
            if state.voice_active { voice_loop.stop_listening(); state.voice_active = false; state.stream.push(sysn("Voice off.")); }
            else { let _ = voice_loop.start_listening(); state.voice_active = true; state.stream.push(sysn("Listening...")); }
        }
        Action::Input(InputAction::Submit) => {
            handle_submit(state, active_stream, streaming_text, streaming_display_cursor,
                streaming_prepared, cognitive, registry, companion_channel, rt, browser_rx, agent_rx, vision,
                shell_rx, shell_mode, conductor_rx);
        }
        Action::Modal(ModalAction::Select) => {
            if let Some(hydra_tui::v2::modal::Modal::CommandPalette { query, .. }) = &state.modal {
                let q = query.clone();
                let results = registry.fuzzy_search(&q);
                if let Some((idx, _)) = results.first() {
                    let cmd = &registry.all()[*idx];
                    let ctx = build_command_context(state);
                    let items = (cmd.handler)("", &ctx);
                    state.modal = None;
                    for item in items { state.stream.push(item); }
                } else { state.modal = None; }
            } else { reduce(state, action); }
        }
        _ => reduce(state, action),
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_submit(
    state: &mut AppState,
    active_stream: &mut Option<tokio::sync::mpsc::Receiver<hydra_kernel::loop_::llm_stream::StreamChunk>>,
    streaming_text: &mut String, streaming_display_cursor: &mut usize,
    streaming_prepared: &mut Option<hydra_kernel::engine::PreparedCycle>,
    cognitive: &mut CognitiveLoop, registry: &commands::registry::CommandRegistry,
    companion_channel: &hydra_signals::CompanionChannel, rt: &tokio::runtime::Runtime,
    browser_rx: &mut Option<tokio::sync::mpsc::Receiver<hydra_tui::v2::browser_task::BrowserUpdate>>,
    agent_rx: &mut Option<tokio::sync::mpsc::Receiver<hydra_tui::v2::agent_task::AgentUpdate>>,
    vision: &Option<std::sync::Arc<dyn hydra_browser::VisionProvider>>,
    shell_rx: &mut Option<tokio::sync::mpsc::Receiver<hydra_tui::v2::shell_task::ShellUpdate>>,
    shell_mode: &mut bool,
    conductor_rx: &mut Option<tokio::sync::mpsc::Receiver<hydra_tui::v2::tui_helpers::ConductorUpdate>>,
) {
    let text = state.input.submit();
    if text.is_empty() { return; }
    if text.starts_with('/') {
        let query = text.trim_start_matches('/');
        let sel = state.slash_selected;
        let search_results = registry.fuzzy_search(query);
        let exec_text = if text.contains(' ') { text.clone() }
            else if let Some(cmd_name) = search_results.get(sel).map(|(idx, _)| format!("/{}", registry.all()[*idx].name)) { cmd_name }
            else { text.clone() };
        let text = exec_text;
        state.slash_selected = 0;
        let ctx = build_command_context(state);
        let items = registry.dispatch(&text, &ctx);
        if text.starts_with("/quit") || text.starts_with("/exit") || text.starts_with("/q") { state.should_quit = true; }
        else if text == "/shell" { *shell_mode = true; state.stream.push(sysn("Shell mode on. Type commands directly. /exit to leave.")); }
        else if text.starts_with("/clear") || text.starts_with("/cls") { state.stream.clear(); }
        else if let Some(cmd) = match text.as_str() {
            "/pause" => Some(hydra_signals::CompanionCommand::Pause),
            "/resume" => Some(hydra_signals::CompanionCommand::Resume),
            "/digest" => Some(hydra_signals::CompanionCommand::RequestDigest),
            "/inbox" => Some(hydra_signals::CompanionCommand::RequestInbox),
            _ => None,
        } { companion_channel.send_command(cmd); }
        for item in items { state.stream.push(item); }
        state.stream.scroll_to_bottom();
        return;
    }
    state.stream.push(StreamItem::UserMessage { id: uuid::Uuid::new_v4(), text: text.clone(), timestamp: chrono::Utc::now() });
    state.stream.scroll_to_bottom();
    state.is_thinking = true;
    // Run async intent classification before prepare_cycle (enriches with LLM if key available)
    let api_key = std::env::var("ANTHROPIC_API_KEY").ok();
    let intent = rt.block_on(hydra_kernel::intent_classifier::classify(&text, api_key.as_deref()));
    let mut prepared = cognitive.prepare_cycle(&text);
    hydra_kernel::intent_classifier::inject_enrichments(&intent, &mut prepared.enrichments);
    // O1: Spawn conductor for task-like inputs (runs in parallel with LLM)
    if hydra_tui::v2::tui_helpers::is_task_intent(&text) && conductor_rx.is_none() {
        *conductor_rx = Some(hydra_tui::v2::tui_helpers::spawn_conductor(rt, text.clone()));
        state.stream.push(sysn("Conductor: planning task..."));
    }
    if prepared.needs_llm {
        for item in hydra_tui::v2::enrichment_bridge::surface_enrichments(&prepared.enrichments) { state.stream.push(item); }
        if browser_rx.is_none() && agent_rx.is_none() {
            let intent = prepared.enrichments.get("agent_intent").map(|s| s.as_str());
            match intent {
                Some("browser_agent") => {
                    *agent_rx = Some(hydra_tui::v2::agent_task::spawn_browser_agent(rt, text.clone(), vision.clone()));
                }
                Some("desktop") => {
                    *agent_rx = Some(hydra_tui::v2::agent_task::spawn_desktop_agent(rt, text.clone(), vision.clone()));
                }
                _ if prepared.enrichments.contains_key("browser_relevant") => {
                    *browser_rx = Some(hydra_tui::v2::browser_task::spawn(rt, text.clone()));
                }
                _ => {}
            }
        }
        state.stream.push(StreamItem::AssistantText { id: uuid::Uuid::new_v4(), text: String::new(), timestamp: chrono::Utc::now() });
        match rt.block_on(cognitive.start_streaming(&prepared)) {
            Ok(rx) => { *active_stream = Some(rx); streaming_text.clear(); *streaming_display_cursor = 0; *streaming_prepared = Some(prepared); }
            Err(e) => { state.is_thinking = false; state.stream.push(sysn(&format!("[Hydra] {e}"))); }
        }
    } else {
        state.is_thinking = false;
        if let Some(resolved) = &prepared.resolved_text {
            state.stream.push(StreamItem::AssistantText { id: uuid::Uuid::new_v4(), text: resolved.clone(), timestamp: chrono::Utc::now() });
        }
        state.session.record(&text, prepared.resolved_text.as_deref().unwrap_or(""), 0, 0);
    }
}

fn drain_llm_stream(
    active_stream: &mut Option<tokio::sync::mpsc::Receiver<hydra_kernel::loop_::llm_stream::StreamChunk>>,
    streaming_text: &mut String, cursor: &mut usize,
    prepared: &mut Option<hydra_kernel::engine::PreparedCycle>,
    state: &mut AppState, cognitive: &mut CognitiveLoop, pacer_chars: usize,
) {
    if let Some(rx) = active_stream.as_mut() {
        loop {
            match rx.try_recv() {
                Ok(chunk) => {
                    use hydra_kernel::loop_::llm_stream::StreamChunk;
                    match chunk {
                        StreamChunk::Text(t) => { streaming_text.push_str(&t); continue; }
                        StreamChunk::Done { tokens_used, duration_ms } => {
                            state.is_thinking = false;
                            state.tokens_used += tokens_used as u64;
                            state.stream.update_last_text(streaming_text);
                            if let Some(prep) = prepared.take() {
                                cognitive.finalize_streaming(prep, streaming_text, tokens_used);
                                state.stream.push(hydra_tui::v2::enrichment_bridge::cycle_metadata(tokens_used, duration_ms, &state.provider));
                                let input_text = state.stream.items().iter().rev()
                                    .find_map(|i| if let StreamItem::UserMessage { text, .. } = i { Some(text.clone()) } else { None })
                                    .unwrap_or_default();
                                state.session.record(&input_text, streaming_text, tokens_used, duration_ms);
                            }
                            state.stream.push(StreamItem::Blank);
                            state.stream.scroll_to_bottom();
                            *active_stream = None; streaming_text.clear(); *cursor = 0;
                            break;
                        }
                        StreamChunk::Error(e) => {
                            state.is_thinking = false;
                            state.stream.push(sysn(&format!("Error: {e}")));
                            *active_stream = None; streaming_text.clear(); *cursor = 0;
                            break;
                        }
                    }
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => { state.is_thinking = false; *active_stream = None; break; }
            }
        }
        if *cursor < streaming_text.len() {
            *cursor = (*cursor + pacer_chars).min(streaming_text.len());
            while *cursor < streaming_text.len() && !streaming_text.is_char_boundary(*cursor) { *cursor += 1; }
            state.stream.update_last_text(&streaming_text[..*cursor]);
            state.stream.scroll_to_bottom();
        }
    }
}
