//! AppState — the single state container for the v2 TUI.
//! All mutation happens through reduce(). Views are pure functions of state.

use crate::input::InputBox;
use crate::stream::ConversationStream;
// StreamItem used transitively via bridge modules
use crate::v2::action::*;
use crate::v2::bridge_streaming::StreamingState;
use crate::v2::modal::Modal;
use crate::v2::session::SessionManager;

/// The complete application state.
pub struct AppState {
    // Core
    pub stream: ConversationStream,
    pub input: InputBox,
    pub modal: Option<Modal>,
    pub session: SessionManager,

    // Status
    pub genome_count: usize,
    pub middleware_count: usize,
    pub memory_size_kb: u64,
    pub provider: String,
    pub model: String,
    pub tokens_used: u64,
    pub session_minutes: u64,

    // Streaming
    pub streaming: StreamingState,
    pub is_thinking: bool,
    pub thinking_verb: String,
    pub think_spinner_frame: usize,

    // Slash menu
    pub slash_selected: usize,

    // Voice
    pub voice_active: bool,
    /// GAP 8: Voice presence state for status bar.
    pub voice_state: Option<String>,

    /// GAP 6: Context-aware input placeholder.
    pub input_placeholder: String,

    // System metrics
    pub lyapunov: f64,

    // Flags
    pub should_quit: bool,
    pub boot_complete: bool,

    /// Generation counter — incremented on every state mutation.
    /// Render loop skips frames when generation hasn't changed (dirty flag).
    pub generation: u64,
}

impl AppState {
    pub fn new(provider: &str, model: &str) -> Self {
        Self {
            stream: ConversationStream::new(),
            input: InputBox::new(),
            modal: None,
            session: SessionManager::new(true),
            genome_count: 0,
            middleware_count: 0,
            memory_size_kb: 0,
            provider: provider.into(),
            model: model.into(),
            tokens_used: 0,
            session_minutes: 0,
            streaming: StreamingState::default(),
            is_thinking: false,
            thinking_verb: "Deliberating".into(),
            think_spinner_frame: 0,
            slash_selected: 0,
            voice_active: false,
            voice_state: None,
            input_placeholder: "What are we building today?".into(),
            lyapunov: 0.42,
            should_quit: false,
            boot_complete: false,
            generation: 0,
        }
    }

    /// Check if a modal is currently open.
    pub fn modal_open(&self) -> bool {
        self.modal.is_some()
    }

    /// Mark state as dirty — must be called after any mutation outside reduce().
    pub fn touch(&mut self) { self.generation = self.generation.wrapping_add(1); }
}

/// The single mutation point. All state changes go through here.
pub fn reduce(state: &mut AppState, action: Action) {
    match action {
        Action::Input(ia) => reduce_input(state, ia),
        Action::Stream(sa) => reduce_stream(state, sa),
        Action::Modal(ma) => reduce_modal(state, ma),
        Action::Command(_cmd) => {
            // Commands are dispatched by the main loop, not reduced directly
        }
        Action::Streaming(sa) => reduce_streaming(state, sa),
        Action::Voice(va) => reduce_voice(state, va),
        Action::Companion(ca) => reduce_companion(state, ca),
        Action::System(sa) => reduce_system(state, sa),
    }
    // Dirty flag: mark state as changed so render loop knows to redraw
    state.touch();
}

fn reduce_input(state: &mut AppState, action: InputAction) {
    match action {
        InputAction::InsertChar(c) => state.input.insert(c),
        InputAction::Backspace => state.input.backspace(),
        InputAction::Delete => state.input.delete(),
        InputAction::Submit => {
            // Handled by main loop (needs cognitive loop access)
        }
        InputAction::MoveLeft => state.input.move_left(),
        InputAction::MoveRight => state.input.move_right(),
        InputAction::MoveWordLeft => state.input.move_word_backward(),
        InputAction::MoveWordRight => state.input.move_word_forward(),
        InputAction::MoveHome => state.input.move_home(),
        InputAction::MoveEnd => state.input.move_end(),
        InputAction::KillToEnd => state.input.kill_to_end(),
        InputAction::KillLine => state.input.kill_line(),
        InputAction::KillWord => state.input.delete_word_backward(),
        InputAction::Yank => state.input.yank(),
        InputAction::HistoryUp => { state.input.history_up(); }
        InputAction::HistoryDown => { state.input.history_down(); }
        InputAction::SearchStart => state.input.start_search(),
        InputAction::SearchInsert(c) => state.input.search_insert(c),
        InputAction::SearchNext => state.input.search_next(),
        InputAction::SearchAccept => state.input.search_accept(),
        InputAction::SearchCancel => state.input.search_cancel(),
    }
}

fn reduce_stream(state: &mut AppState, action: StreamAction) {
    match action {
        StreamAction::ScrollUp(n) => state.stream.scroll_up(n),
        StreamAction::ScrollDown(n) => state.stream.scroll_down(n),
        StreamAction::ScrollToBottom => state.stream.scroll_to_bottom(),
        StreamAction::Clear => state.stream.clear(),
        StreamAction::PushItem(item) => state.stream.push(item),
    }
}

fn reduce_modal(state: &mut AppState, action: ModalAction) {
    match action {
        ModalAction::OpenPalette => {
            state.modal = Some(Modal::palette());
        }
        ModalAction::OpenConfigEditor => {
            state.modal = Some(Modal::config_editor(&std::collections::HashMap::new()));
        }
        ModalAction::OpenSessionList => {
            let sessions = state.session.list_sessions();
            state.modal = Some(Modal::SessionList {
                sessions,
                selected: 0,
            });
        }
        ModalAction::Close => {
            state.modal = None;
        }
        ModalAction::NavigateUp => {
            if let Some(modal) = &mut state.modal {
                modal.navigate_up();
            }
        }
        ModalAction::NavigateDown => {
            if let Some(modal) = &mut state.modal {
                modal.navigate_down();
            }
        }
        ModalAction::Select => {
            // Handle selection based on modal type
            // The main loop processes this and dispatches further actions
        }
        ModalAction::TypeChar(c) => {
            if let Some(modal) = &mut state.modal {
                modal.type_char(c);
            }
        }
        ModalAction::Backspace => {
            if let Some(modal) = &mut state.modal {
                modal.backspace();
            }
        }
        _ => {}
    }
}

fn reduce_streaming(state: &mut AppState, action: StreamingAction) {
    let sub_actions = crate::v2::bridge_streaming::process_streaming_action(
        &action,
        &mut state.streaming,
    );
    match &action {
        StreamingAction::Start { .. } => state.is_thinking = true,
        StreamingAction::Done { tokens, .. } => {
            state.is_thinking = false;
            state.tokens_used += *tokens as u64;
        }
        StreamingAction::Error(_) | StreamingAction::Interrupt => {
            state.is_thinking = false;
        }
        _ => {}
    }
    // Process sub-actions
    for sa in sub_actions {
        reduce(state, sa);
    }
}

fn reduce_voice(state: &mut AppState, action: VoiceAction) {
    match action {
        VoiceAction::Listening => { state.voice_state = Some("listening".into()); }
        VoiceAction::Speaking(_) => { state.voice_state = Some("speaking".into()); }
        VoiceAction::SpeakingDone => { state.voice_state = Some("dormant".into()); }
        VoiceAction::WakeWordDetected => { state.voice_state = Some("listening".into()); }
        VoiceAction::SessionTimeout => { state.voice_state = Some("dormant".into()); }
        VoiceAction::FinalTranscript(text) => {
            state.input.clear();
            for ch in text.chars() { state.input.insert(ch); }
            state.voice_state = Some("processing".into());
        }
        VoiceAction::Toggle => {
            state.voice_active = !state.voice_active;
            state.voice_state = if state.voice_active { Some("dormant".into()) } else { None };
        }
        _ => {}
    }
}

fn reduce_companion(state: &mut AppState, action: CompanionAction) {
    if let CompanionAction::Signal { source, content } = action {
        state.stream.push(
            crate::v2::bridge_companion::signal_item(&source, &content),
        );
    }
}

fn reduce_system(state: &mut AppState, action: SystemAction) {
    match action {
        SystemAction::Quit => state.should_quit = true,
        SystemAction::BootComplete { genome_count, middleware_count } => {
            state.genome_count = genome_count;
            state.middleware_count = middleware_count;
            state.boot_complete = true;
        }
        SystemAction::Tick => {
            // Update session minutes
            // (called from main loop timer)
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state() {
        let state = AppState::new("anthropic", "sonnet");
        assert!(!state.should_quit);
        assert!(!state.modal_open());
        assert!(!state.is_thinking);
    }

    #[test]
    fn reduce_quit() {
        let mut state = AppState::new("anthropic", "sonnet");
        reduce(&mut state, Action::System(SystemAction::Quit));
        assert!(state.should_quit);
    }

    #[test]
    fn reduce_open_palette() {
        let mut state = AppState::new("anthropic", "sonnet");
        reduce(&mut state, Action::Modal(ModalAction::OpenPalette));
        assert!(state.modal_open());
    }

    #[test]
    fn reduce_close_modal() {
        let mut state = AppState::new("anthropic", "sonnet");
        reduce(&mut state, Action::Modal(ModalAction::OpenPalette));
        reduce(&mut state, Action::Modal(ModalAction::Close));
        assert!(!state.modal_open());
    }

    #[test]
    fn reduce_insert_char() {
        let mut state = AppState::new("anthropic", "sonnet");
        reduce(&mut state, Action::Input(InputAction::InsertChar('h')));
        reduce(&mut state, Action::Input(InputAction::InsertChar('i')));
        assert_eq!(state.input.text(), "hi");
    }

    #[test]
    fn reduce_boot_complete() {
        let mut state = AppState::new("anthropic", "sonnet");
        reduce(&mut state, Action::System(SystemAction::BootComplete {
            genome_count: 390,
            middleware_count: 9,
        }));
        assert!(state.boot_complete);
        assert_eq!(state.genome_count, 390);
    }
}
