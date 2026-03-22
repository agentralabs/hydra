//! HydraTui — top-level coordinator for the TUI.
//!
//! Ties together stream, status, input, pacer, and views.

use crate::cockpit::{CockpitMode, CockpitView};
use crate::input::InputBox;
use crate::pacer::{OutputPacer, PacerSignals};
use crate::status::StatusLine;
use crate::stream::ConversationStream;
use crate::stream_types::StreamItem;
use crate::verb::{ThinkingVerbState, VerbContext};
use crate::welcome::WelcomeScreen;

/// The top-level TUI state.
#[derive(Debug, Clone)]
pub struct HydraTui {
    /// The conversation stream.
    pub stream: ConversationStream,
    /// The status line.
    pub status: StatusLine,
    /// The input box.
    pub input: InputBox,
    /// The output pacer.
    pub pacer: OutputPacer,
    /// The cockpit view.
    pub cockpit: CockpitView,
    /// The welcome screen.
    pub welcome: WelcomeScreen,
    /// Whether the TUI should quit.
    pub should_quit: bool,
}

impl HydraTui {
    /// Create a new HydraTui instance.
    pub fn new() -> Self {
        Self {
            stream: ConversationStream::new(),
            status: StatusLine::new(),
            input: InputBox::new(),
            pacer: OutputPacer::new(),
            cockpit: CockpitView::new(),
            welcome: WelcomeScreen::new(),
            should_quit: false,
        }
    }

    /// Push a stream item through the pacer.
    pub fn push_item(&mut self, item: StreamItem) {
        self.pacer.reset_for_new_item();
        self.stream.push(item);
    }

    /// Update the thinking verb context.
    pub fn set_verb_context(&mut self, context: VerbContext) {
        self.status.verb_state.set_context(context);
    }

    /// Start the thinking animation.
    pub fn start_thinking(&mut self) {
        self.status.verb_state.start();
    }

    /// Stop the thinking animation.
    pub fn stop_thinking(&mut self) {
        self.status.verb_state.stop();
    }

    /// Tick the spinner and optionally rotate verb.
    pub fn tick(&mut self) {
        self.status.verb_state.tick_spinner();
    }

    /// Rotate the thinking verb to the next alternative.
    pub fn rotate_verb(&mut self) {
        self.status.verb_state.rotate_verb();
    }

    /// Update pacer signals from UI events.
    pub fn update_pacer_signals(&mut self, signals: PacerSignals) {
        self.pacer.update_signals(signals);
    }

    /// Mark the kernel as ready and enter conversation mode.
    pub fn kernel_ready(&mut self) {
        self.welcome.kernel_ready = true;
        self.cockpit.enter_conversation();
    }

    /// Request the TUI to quit.
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    /// Return whether the cockpit is in conversation mode.
    pub fn is_conversation(&self) -> bool {
        self.cockpit.is_conversation()
    }

    /// Return the current cockpit mode.
    pub fn mode(&self) -> &CockpitMode {
        &self.cockpit.mode
    }

    /// Return a snapshot of the current thinking verb state.
    pub fn verb_state(&self) -> &ThinkingVerbState {
        &self.status.verb_state
    }
}

impl Default for HydraTui {
    fn default() -> Self {
        Self::new()
    }
}
