//! Companion slash commands — send commands via signal channel.
//!
//! Uses hydra-signals CompanionChannel to communicate with the
//! companion service. No direct dependency on hydra-companion.

use hydra_signals::companion_channel::{CompanionChannel, CompanionCommand, CompanionOutput};

use crate::stream_types::StreamItem;

fn sys(content: &str) -> StreamItem {
    StreamItem::SystemNotification {
        id: uuid::Uuid::new_v4(),
        content: content.to_string(),
        timestamp: chrono::Utc::now(),
    }
}

/// Send a pause command and return UI feedback.
pub fn cmd_pause(channel: Option<&CompanionChannel>) -> Vec<StreamItem> {
    match channel {
        Some(ch) => {
            ch.send_command(CompanionCommand::Pause);
            vec![sys("Companion pause requested...")]
        }
        None => vec![sys("Companion not active. Enable in /settings.")],
    }
}

/// Send a resume command.
pub fn cmd_resume(channel: Option<&CompanionChannel>) -> Vec<StreamItem> {
    match channel {
        Some(ch) => {
            ch.send_command(CompanionCommand::Resume);
            vec![sys("Companion resume requested...")]
        }
        None => vec![sys("Companion not active. Enable in /settings.")],
    }
}

/// Request digest and display any immediately available items.
pub fn cmd_digest(channel: Option<&CompanionChannel>) -> Vec<StreamItem> {
    match channel {
        Some(ch) => {
            ch.send_command(CompanionCommand::RequestDigest);
            // Poll for immediate response (companion may respond next tick)
            if let Some(CompanionOutput::DigestItems(items)) = ch.poll_output() {
                if items.is_empty() {
                    return vec![sys("No batched signals. Inbox is clear.")];
                }
                let mut result = vec![sys(&format!("Digest — {} signals:", items.len()))];
                for item in items.iter().take(20) {
                    result.push(sys(&format!("  {} {}", item.symbol, item.content)));
                }
                result
            } else {
                vec![sys("Digest requested. Results will appear shortly.")]
            }
        }
        None => vec![sys("Companion not active. Enable in /settings.")],
    }
}

/// Request inbox.
pub fn cmd_inbox(channel: Option<&CompanionChannel>) -> Vec<StreamItem> {
    match channel {
        Some(ch) => {
            ch.send_command(CompanionCommand::RequestInbox);
            if let Some(CompanionOutput::InboxItems(items)) = ch.poll_output() {
                if items.is_empty() {
                    return vec![sys("Inbox empty.")];
                }
                let mut result = vec![sys(&format!("Inbox — {} signals:", items.len()))];
                for item in items.iter().take(30) {
                    result.push(sys(&format!(
                        "  {} {} [{}]",
                        item.symbol, item.content, item.source
                    )));
                }
                result
            } else {
                vec![sys("Inbox requested. Results will appear shortly.")]
            }
        }
        None => vec![sys("Companion not active. Enable in /settings.")],
    }
}

/// Request status.
pub fn cmd_status(channel: Option<&CompanionChannel>) -> Vec<StreamItem> {
    match channel {
        Some(ch) => {
            ch.send_command(CompanionCommand::RequestStatus);
            if let Some(CompanionOutput::Status {
                paused,
                signal_count,
                task_count,
            }) = ch.poll_output()
            {
                let state = if paused { "paused" } else { "active" };
                vec![
                    sys("Companion status:"),
                    sys(&format!("  state:   {state}")),
                    sys(&format!("  signals: {signal_count} in buffer")),
                    sys(&format!("  tasks:   {task_count} active")),
                ]
            } else {
                vec![sys("Companion status requested. Will update shortly.")]
            }
        }
        None => vec![sys("Companion not active. Enable in /settings.")],
    }
}
