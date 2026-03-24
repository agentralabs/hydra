//! Companion bridge — converts companion signals into Actions.

use crate::v2::action::{Action, CompanionAction};
use crate::stream_types::StreamItem;

/// Poll the companion channel and return actions.
pub fn poll_companion(
    channel: &hydra_signals::CompanionChannel,
) -> Vec<Action> {
    let mut actions = Vec::new();

    while let Some(output) = channel.poll_output() {
        match output {
            hydra_signals::CompanionOutput::Signal { content, source, .. } => {
                actions.push(Action::Companion(CompanionAction::Signal {
                    source,
                    content,
                }));
            }
            hydra_signals::CompanionOutput::Message(msg) => {
                actions.push(Action::Companion(CompanionAction::Signal {
                    source: "companion".into(),
                    content: msg,
                }));
            }
            _ => {}
        }
    }

    actions
}

/// Create a stream item for a companion signal.
pub fn signal_item(source: &str, content: &str) -> StreamItem {
    StreamItem::SystemNotification {
        id: uuid::Uuid::new_v4(),
        content: format!("◈ {source}: {content}"),
        timestamp: chrono::Utc::now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_item_formats() {
        let item = signal_item("bridge:telegram", "New message from user");
        if let StreamItem::SystemNotification { content, .. } = item {
            assert!(content.contains("telegram"));
        }
    }
}
