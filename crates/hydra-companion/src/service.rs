//! CompanionService — runs independently, communicates via signal channel.
//!
//! Receives commands from TUI via CompanionEndpoint.
//! Sends outputs (signals, status, digests) back to TUI.
//! No direct dependency on hydra-tui. Communication is through hydra-signals.

use hydra_signals::companion_channel::{
    CompanionCommand, CompanionEndpoint, CompanionOutput, DigestEntry, InboxEntry,
};

use crate::companion::Companion;

/// The companion service — wraps a Companion and bridges to the channel.
pub struct CompanionService {
    companion: Companion,
    endpoint: CompanionEndpoint,
}

impl CompanionService {
    /// Create a new companion service with the given channel endpoint.
    pub fn new(endpoint: CompanionEndpoint) -> Self {
        Self {
            companion: Companion::new(),
            endpoint,
        }
    }

    /// Tick the service — process commands, emit outputs.
    /// Call this from the ambient loop or a dedicated thread.
    pub fn tick(&mut self) {
        // Process all pending commands
        while let Some(cmd) = self.endpoint.poll_command() {
            self.handle_command(cmd);
        }
    }

    fn handle_command(&mut self, cmd: CompanionCommand) {
        match cmd {
            CompanionCommand::Pause => {
                self.companion.pause();
                self.endpoint.send_output(CompanionOutput::Message(
                    "Companion paused. Signals still collected.".into(),
                ));
            }
            CompanionCommand::Resume => {
                self.companion.resume();
                self.endpoint.send_output(CompanionOutput::Message(
                    "Companion resumed.".into(),
                ));
            }
            CompanionCommand::RequestDigest => {
                let items: Vec<DigestEntry> = self
                    .companion
                    .digest()
                    .iter()
                    .map(|r| DigestEntry {
                        symbol: r.class.symbol().to_string(),
                        content: r.content.clone(),
                    })
                    .collect();
                self.endpoint
                    .send_output(CompanionOutput::DigestItems(items));
            }
            CompanionCommand::RequestInbox => {
                let items: Vec<InboxEntry> = self
                    .companion
                    .inbox()
                    .iter()
                    .map(|r| InboxEntry {
                        symbol: r.class.symbol().to_string(),
                        content: r.content.clone(),
                        source: r.source.clone(),
                    })
                    .collect();
                self.endpoint
                    .send_output(CompanionOutput::InboxItems(items));
            }
            CompanionCommand::RequestStatus => {
                self.endpoint.send_output(CompanionOutput::Status {
                    paused: self.companion.is_paused(),
                    signal_count: self.companion.signals().len(),
                    task_count: self.companion.active_task_count(),
                });
            }
        }
    }

    /// Feed an external signal into the companion for classification.
    pub fn ingest_signal(&mut self, source: &str, content: &str) {
        let signal_item = crate::signal::SignalItem::new(source.to_string(), content.to_string());
        let routed = self.companion.receive_signal(signal_item);
        let class = match routed.class {
            crate::signal::SignalClass::Urgent => hydra_signals::companion_channel::SignalClass::Urgent,
            crate::signal::SignalClass::Notable => hydra_signals::companion_channel::SignalClass::Notable,
            crate::signal::SignalClass::Routine => hydra_signals::companion_channel::SignalClass::Routine,
            crate::signal::SignalClass::Noise => hydra_signals::companion_channel::SignalClass::Noise,
        };
        self.endpoint.send_output(CompanionOutput::Signal {
            class,
            content: routed.content,
            source: routed.source,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydra_signals::companion_channel::create_channel;

    #[test]
    fn service_handles_pause() {
        let (tui_side, endpoint) = create_channel();
        let mut service = CompanionService::new(endpoint);

        tui_side.send_command(CompanionCommand::Pause);
        service.tick();

        let output = tui_side.poll_output().unwrap();
        match output {
            CompanionOutput::Message(msg) => assert!(msg.contains("paused")),
            _ => panic!("expected Message"),
        }
    }

    #[test]
    fn service_handles_status() {
        let (tui_side, endpoint) = create_channel();
        let mut service = CompanionService::new(endpoint);

        tui_side.send_command(CompanionCommand::RequestStatus);
        service.tick();

        let output = tui_side.poll_output().unwrap();
        assert!(matches!(output, CompanionOutput::Status { .. }));
    }
}
