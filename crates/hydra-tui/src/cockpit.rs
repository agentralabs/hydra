//! Cockpit view — the main conversation layout.

/// The cockpit view state — determines what is rendered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CockpitMode {
    /// Welcome screen before conversation starts.
    Welcome,
    /// Active conversation.
    Conversation,
    /// Companion task panel visible.
    CompanionPanel,
}

/// Render data for the cockpit.
#[derive(Debug, Clone)]
pub struct CockpitView {
    /// Current mode.
    pub mode: CockpitMode,
    /// Whether the input box is focused.
    pub input_focused: bool,
    /// Title for the cockpit header.
    pub title: String,
}

impl CockpitView {
    /// Create a new cockpit view in welcome mode.
    pub fn new() -> Self {
        Self {
            mode: CockpitMode::Welcome,
            input_focused: true,
            title: String::from("Hydra"),
        }
    }

    /// Switch to conversation mode.
    pub fn enter_conversation(&mut self) {
        self.mode = CockpitMode::Conversation;
    }

    /// Toggle the companion panel.
    pub fn toggle_companion_panel(&mut self) {
        self.mode = match self.mode {
            CockpitMode::CompanionPanel => CockpitMode::Conversation,
            _ => CockpitMode::CompanionPanel,
        };
    }

    /// Return whether the cockpit is in conversation mode.
    pub fn is_conversation(&self) -> bool {
        matches!(
            self.mode,
            CockpitMode::Conversation | CockpitMode::CompanionPanel
        )
    }
}

impl Default for CockpitView {
    fn default() -> Self {
        Self::new()
    }
}
