//! Companion signal channel — decoupled communication between TUI and Companion.
//!
//! Neither TUI nor Companion depends on the other. Both depend on this module
//! which lives in hydra-signals (the shared fabric crate).
//!
//! Flow:
//!   External sources → Companion classifier → CompanionOutput → fabric → TUI subscriber
//!   TUI /commands → CompanionCommand → fabric → Companion subscriber
//!
//! This eliminates the circular dependency entirely.

use std::sync::mpsc;

/// Commands sent FROM the TUI TO the Companion system.
#[derive(Debug, Clone)]
pub enum CompanionCommand {
    Pause,
    Resume,
    RequestDigest,
    RequestInbox,
    RequestStatus,
}

/// Outputs sent FROM the Companion TO the TUI for display.
#[derive(Debug, Clone)]
pub enum CompanionOutput {
    /// A classified signal ready for display.
    Signal {
        class: SignalClass,
        content: String,
        source: String,
    },
    /// Companion status report.
    Status {
        paused: bool,
        signal_count: usize,
        task_count: usize,
    },
    /// Digest items.
    DigestItems(Vec<DigestEntry>),
    /// Inbox items.
    InboxItems(Vec<InboxEntry>),
    /// Companion message (free text).
    Message(String),
}

/// Signal urgency class (shared between companion and TUI).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalClass {
    Urgent,
    Notable,
    Routine,
    Noise,
}

impl SignalClass {
    /// TUI display symbol.
    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Urgent => "▲",
            Self::Notable => "●",
            Self::Routine => "○",
            Self::Noise => "",
        }
    }
}

/// One digest entry for display.
#[derive(Debug, Clone)]
pub struct DigestEntry {
    pub symbol: String,
    pub content: String,
}

/// One inbox entry for display.
#[derive(Debug, Clone)]
pub struct InboxEntry {
    pub symbol: String,
    pub content: String,
    pub source: String,
}

/// The companion channel — two-way communication via mpsc channels.
pub struct CompanionChannel {
    /// Send commands to the companion.
    pub command_tx: mpsc::Sender<CompanionCommand>,
    /// Receive outputs from the companion.
    pub output_rx: mpsc::Receiver<CompanionOutput>,
}

/// The companion's end of the channel.
pub struct CompanionEndpoint {
    /// Receive commands from the TUI.
    pub command_rx: mpsc::Receiver<CompanionCommand>,
    /// Send outputs to the TUI.
    pub output_tx: mpsc::Sender<CompanionOutput>,
}

/// Create a paired companion channel. Returns (TUI side, Companion side).
pub fn create_channel() -> (CompanionChannel, CompanionEndpoint) {
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let (out_tx, out_rx) = mpsc::channel();

    let channel = CompanionChannel {
        command_tx: cmd_tx,
        output_rx: out_rx,
    };
    let endpoint = CompanionEndpoint {
        command_rx: cmd_rx,
        output_tx: out_tx,
    };

    (channel, endpoint)
}

impl CompanionChannel {
    /// Send a command to the companion (non-blocking).
    pub fn send_command(&self, cmd: CompanionCommand) {
        let _ = self.command_tx.send(cmd);
    }

    /// Poll for outputs from the companion (non-blocking).
    pub fn poll_output(&self) -> Option<CompanionOutput> {
        self.output_rx.try_recv().ok()
    }
}

impl CompanionEndpoint {
    /// Poll for commands from the TUI (non-blocking).
    pub fn poll_command(&self) -> Option<CompanionCommand> {
        self.command_rx.try_recv().ok()
    }

    /// Send an output to the TUI (non-blocking).
    pub fn send_output(&self, output: CompanionOutput) {
        let _ = self.output_tx.send(output);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_round_trip() {
        let (tui_side, companion_side) = create_channel();

        // TUI sends command
        tui_side.send_command(CompanionCommand::Pause);
        let cmd = companion_side.poll_command().unwrap();
        assert!(matches!(cmd, CompanionCommand::Pause));

        // Companion sends output
        companion_side.send_output(CompanionOutput::Message("paused".into()));
        let out = tui_side.poll_output().unwrap();
        assert!(matches!(out, CompanionOutput::Message(_)));
    }

    #[test]
    fn poll_empty_returns_none() {
        let (tui_side, companion_side) = create_channel();
        assert!(tui_side.poll_output().is_none());
        assert!(companion_side.poll_command().is_none());
    }
}
