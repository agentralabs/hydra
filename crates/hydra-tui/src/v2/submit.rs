//! Submit handling — extracted from the TUI binary for 400-line compliance.
//! Contains handle_submit() and drain_llm_stream().

use hydra_kernel::engine::CognitiveLoop;
use crate::stream_types::StreamItem;
use crate::v2::action::*;
use crate::v2::commands;
use crate::v2::state::{AppState, reduce};
use crate::v2::tui_helpers::{build_command_context, sysn};

#[allow(clippy::too_many_arguments)]
pub fn handle_action(
    action: Action, state: &mut AppState,
    active_stream: &mut Option<tokio::sync::mpsc::Receiver<hydra_kernel::loop_::llm_stream::StreamChunk>>,
    streaming_text: &mut String, streaming_display_cursor: &mut usize,
    streaming_prepared: &mut Option<hydra_kernel::engine::PreparedCycle>,
    cognitive: &mut CognitiveLoop, registry: &commands::registry::CommandRegistry,
    companion_channel: &hydra_signals::CompanionChannel, voice_loop: &mut hydra_voice::VoiceLoop,
    rt: &tokio::runtime::Runtime,
    browser_rx: &mut Option<tokio::sync::mpsc::Receiver<crate::v2::browser_task::BrowserUpdate>>,
    agent_rx: &mut Option<tokio::sync::mpsc::Receiver<crate::v2::agent_task::AgentUpdate>>,
    vision: &Option<std::sync::Arc<dyn hydra_browser::VisionProvider>>,
    shell_rx: &mut Option<tokio::sync::mpsc::Receiver<crate::v2::shell_task::ShellUpdate>>,
    shell_mode: &mut bool,
    conductor_rx: &mut Option<tokio::sync::mpsc::Receiver<crate::v2::tui_helpers::ConductorUpdate>>,
) {
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
                *shell_rx = Some(crate::v2::shell_task::spawn(rt, text));
            }
            return;
        }
    }
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
        Action::System(SystemAction::Quit) | Action::Streaming(StreamingAction::Interrupt) if active_stream.is_some() => {
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
            if let Some(crate::v2::modal::Modal::CommandPalette { query, .. }) = &state.modal {
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
    browser_rx: &mut Option<tokio::sync::mpsc::Receiver<crate::v2::browser_task::BrowserUpdate>>,
    agent_rx: &mut Option<tokio::sync::mpsc::Receiver<crate::v2::agent_task::AgentUpdate>>,
    vision: &Option<std::sync::Arc<dyn hydra_browser::VisionProvider>>,
    shell_rx: &mut Option<tokio::sync::mpsc::Receiver<crate::v2::shell_task::ShellUpdate>>,
    shell_mode: &mut bool,
    conductor_rx: &mut Option<tokio::sync::mpsc::Receiver<crate::v2::tui_helpers::ConductorUpdate>>,
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
        // Intercepted commands: handle directly, skip registry.dispatch() to avoid blocking
        let intercepted = if text.starts_with("/quit") || text.starts_with("/exit") || text.starts_with("/q") {
            state.should_quit = true; true
        } else if text == "/shell" {
            *shell_mode = true; state.stream.push(sysn("Shell mode on. Type commands directly. /exit to leave.")); true
        } else if text.starts_with("/clear") || text.starts_with("/cls") {
            state.stream.clear(); true
        } else if text == "/sessions" || text == "/resume" {
            reduce(state, Action::Modal(ModalAction::OpenSessionList)); true
        } else if text == "/settings" || text == "/config" {
            reduce(state, Action::Modal(ModalAction::OpenConfigEditor)); true
        } else if (text.starts_with("/do ") || text.starts_with("/execute ") || text.starts_with("/task ")) && conductor_rx.is_none() {
            let goal = text.splitn(2, ' ').nth(1).unwrap_or("").trim().to_string();
            if !goal.is_empty() {
                *conductor_rx = Some(crate::v2::tui_helpers::spawn_conductor(rt, goal));
                state.stream.push(sysn("◌ Executing via conductor..."));
            } else { state.stream.push(sysn("Usage: /do <goal>")); }
            true
        } else if text.starts_with("/code ") || text.starts_with("/coder ") {
            let goal = text.splitn(2, ' ').nth(1).unwrap_or("").trim().to_string();
            if !goal.is_empty() {
                state.stream.push(sysn("◌ Starting coder pipeline..."));
                *conductor_rx = Some(crate::v2::tui_helpers::spawn_coder(rt, goal));
            } else { state.stream.push(sysn("Usage: /code <description>")); }
            true
        } else if let Some(cmd) = match text.as_str() {
            "/pause" => Some(hydra_signals::CompanionCommand::Pause),
            "/resume" => Some(hydra_signals::CompanionCommand::Resume),
            "/digest" => Some(hydra_signals::CompanionCommand::RequestDigest),
            "/inbox" => Some(hydra_signals::CompanionCommand::RequestInbox),
            _ => None,
        } { companion_channel.send_command(cmd); true }
        else { false };
        // Only dispatch through registry for non-intercepted commands
        if !intercepted {
            let ctx = build_command_context(state);
            let items = registry.dispatch(&text, &ctx);
            for item in items { state.stream.push(item); }
        }
        state.stream.scroll_to_bottom();
        return;
    }
    state.stream.push(StreamItem::UserMessage { id: uuid::Uuid::new_v4(), text: text.clone(), timestamp: chrono::Utc::now() });
    state.stream.scroll_to_bottom();
    state.is_thinking = true;
    let api_key = std::env::var("ANTHROPIC_API_KEY").ok();
    let intent = rt.block_on(hydra_kernel::intent_classifier::classify(&text, api_key.as_deref()));
    let mut prepared = cognitive.prepare_cycle(&text);
    hydra_kernel::intent_classifier::inject_enrichments(&intent, &mut prepared.enrichments);
    state.stream.push(sysn(&format!("◌ route: {intent}")));
    match &intent {
        i if i.is_actionable() && conductor_rx.is_none() => {
            *conductor_rx = Some(crate::v2::tui_helpers::spawn_conductor(rt, text.clone()));
            state.stream.push(sysn("◌ Executing..."));
            state.is_thinking = false;
            return;
        }
        hydra_kernel::intent_classifier::AgentIntent::BrowserAgent if agent_rx.is_none() => {
            *agent_rx = Some(crate::v2::agent_task::spawn_browser_agent(rt, text.clone(), vision.clone()));
        }
        hydra_kernel::intent_classifier::AgentIntent::Desktop if agent_rx.is_none() => {
            *agent_rx = Some(crate::v2::agent_task::spawn_desktop_agent(rt, text.clone(), vision.clone()));
        }
        hydra_kernel::intent_classifier::AgentIntent::BrowserFetch if browser_rx.is_none() => {
            *browser_rx = Some(crate::v2::browser_task::spawn(rt, text.clone()));
        }
        _ => {}
    }
    if prepared.needs_llm {
        for item in crate::v2::enrichment_bridge::surface_enrichments(&prepared.enrichments) { state.stream.push(item); }
        if !cognitive.last_thinking.is_empty() {
            state.stream.push(StreamItem::thinking("", "hydra thinking...", 0));
            for step in &cognitive.last_thinking {
                let indent = match step.mode {
                    hydra_kernel::deliberation::CognitiveMode::Research => 1,
                    hydra_kernel::deliberation::CognitiveMode::Critique => 1,
                    _ => 0,
                };
                state.stream.push(StreamItem::thinking(step.mode.label(), &step.thought, indent));
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

pub fn drain_llm_stream(
    active_stream: &mut Option<tokio::sync::mpsc::Receiver<hydra_kernel::loop_::llm_stream::StreamChunk>>,
    streaming_text: &mut String, cursor: &mut usize,
    prepared: &mut Option<hydra_kernel::engine::PreparedCycle>,
    state: &mut AppState, cognitive: &mut CognitiveLoop, pacer_chars: usize,
    voice_loop: &mut hydra_voice::VoiceLoop,
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
                            let (clean_text, actions) = crate::v2::action_parser::parse_response(streaming_text);
                            for action in &actions {
                                state.stream.push(StreamItem::SystemNotification {
                                    id: uuid::Uuid::new_v4(), content: action.display.clone(), timestamp: chrono::Utc::now(),
                                });
                                let result = crate::v2::action_parser::execute_action(action);
                                state.stream.push(StreamItem::SystemNotification {
                                    id: uuid::Uuid::new_v4(), content: result, timestamp: chrono::Utc::now(),
                                });
                            }
                            *streaming_text = clean_text;
                            state.stream.update_last_text(streaming_text);
                            if let Some(prep) = prepared.take() {
                                cognitive.finalize_streaming(prep, streaming_text, tokens_used);
                                state.stream.push(crate::v2::enrichment_bridge::cycle_metadata(tokens_used, duration_ms, &state.provider));
                                let input_text = state.stream.items().iter().rev()
                                    .find_map(|i| if let StreamItem::UserMessage { text, .. } = i { Some(text.clone()) } else { None })
                                    .unwrap_or_default();
                                state.session.record(&input_text, streaming_text, tokens_used, duration_ms);
                            }
                            state.stream.push(StreamItem::ThinkingPill { duration_secs: duration_ms as f64 / 1000.0 });
                            state.input_placeholder = crate::v2::tui_helpers::suggest_placeholder(streaming_text);
                            if state.voice_active { voice_loop.speak_response(streaming_text); }
                            state.stream.push(StreamItem::Blank);
                            state.stream.scroll_to_bottom();
                            *active_stream = None; streaming_text.clear(); *cursor = 0;
                            break;
                        }
                        StreamChunk::Error(e) => {
                            state.is_thinking = false;
                            let err = format!("{e}");
                            if crate::v2::tui_helpers::is_transient_error(&err) {
                                state.stream.push(sysn(&format!("Transient error: {err}")));
                                state.stream.push(sysn("Press Enter to retry, or type a new message."));
                            } else {
                                state.stream.push(sysn(&format!("Error: {err}")));
                            }
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
            let display_text = &streaming_text[..*cursor];
            let clean = if display_text.contains("<computer_use>") {
                let (clean, _) = crate::v2::action_parser::parse_response(display_text);
                clean
            } else { display_text.to_string() };
            state.stream.update_last_text(&clean);
            if state.stream.is_auto_scroll() { state.stream.scroll_to_bottom(); }
        }
    }
}
