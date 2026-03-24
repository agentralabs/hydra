//! Action — every possible state transition in the TUI.
//! The TUI never mutates state directly. All changes go through actions.
//! dispatch(event) → Vec<Action> → reduce(state, action)

/// Top-level action enum.
#[derive(Debug, Clone)]
pub enum Action {
    /// Input box mutations.
    Input(InputAction),
    /// Conversation stream mutations.
    Stream(StreamAction),
    /// Modal (palette, config editor, etc.) mutations.
    Modal(ModalAction),
    /// Slash command execution.
    Command(String),
    /// LLM streaming lifecycle.
    Streaming(StreamingAction),
    /// Voice system.
    Voice(VoiceAction),
    /// Companion/bridge signals.
    Companion(CompanionAction),
    /// System-level actions.
    System(SystemAction),
}

/// Input box actions.
#[derive(Debug, Clone)]
pub enum InputAction {
    InsertChar(char),
    Backspace,
    Delete,
    Submit,
    MoveLeft,
    MoveRight,
    MoveWordLeft,
    MoveWordRight,
    MoveHome,
    MoveEnd,
    KillToEnd,
    KillLine,
    KillWord,
    Yank,
    HistoryUp,
    HistoryDown,
    SearchStart,
    SearchInsert(char),
    SearchNext,
    SearchAccept,
    SearchCancel,
}

/// Stream actions.
#[derive(Debug, Clone)]
pub enum StreamAction {
    ScrollUp(usize),
    ScrollDown(usize),
    ScrollToBottom,
    Clear,
    PushItem(crate::stream_types::StreamItem),
}

/// Modal actions.
#[derive(Debug, Clone)]
pub enum ModalAction {
    OpenPalette,
    OpenConfigEditor,
    OpenKeybindingEditor,
    OpenSessionList,
    Confirm { message: String, on_yes: Box<Action> },
    Close,
    NavigateUp,
    NavigateDown,
    Select,
    TypeChar(char),
    Backspace,
}

/// LLM streaming actions.
#[derive(Debug, Clone)]
pub enum StreamingAction {
    Start { session_id: String },
    Chunk(String),
    Done { tokens: usize, duration_ms: u64 },
    Error(String),
    Interrupt,
}

/// Voice actions.
#[derive(Debug, Clone)]
pub enum VoiceAction {
    Toggle,
    Listening,
    PartialTranscript(String),
    FinalTranscript(String),
    Speaking(String),
    SpeakingDone,
    Error(String),
    /// O17: Wake word detected — transitioning to active listening.
    WakeWordDetected,
    /// O17: Session timed out — back to dormant.
    SessionTimeout,
}

/// Companion/bridge actions.
#[derive(Debug, Clone)]
pub enum CompanionAction {
    Signal { source: String, content: String },
    TaskBlocked { id: String, reason: String },
    Pause,
    Resume,
}

/// System-level actions.
#[derive(Debug, Clone)]
pub enum SystemAction {
    Quit,
    Resize { width: u16, height: u16 },
    Tick,
    ToggleExpand,
    ConfigChanged,
    BootComplete { genome_count: usize, middleware_count: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Action>();
    }

    #[test]
    fn actions_clone() {
        let a = Action::System(SystemAction::Quit);
        let _b = a.clone();
    }
}
