//! Voice bridge — converts voice events into Actions.
//! No state mutation. Just produces actions for the reducer.

use crate::v2::action::{Action, VoiceAction};
use crate::stream_types::StreamItem;

/// Poll the voice loop and return actions.
pub fn poll_voice(voice_loop: &mut hydra_voice::VoiceLoop) -> Vec<Action> {
    let mut actions = Vec::new();

    for event in voice_loop.poll() {
        match event {
            hydra_voice::voice_loop::VoiceUiEvent::Listening => {
                actions.push(Action::Voice(VoiceAction::Listening));
            }
            hydra_voice::voice_loop::VoiceUiEvent::PartialTranscript(text) => {
                actions.push(Action::Voice(VoiceAction::PartialTranscript(text)));
            }
            hydra_voice::voice_loop::VoiceUiEvent::FinalTranscript(text) => {
                actions.push(Action::Voice(VoiceAction::FinalTranscript(text)));
            }
            hydra_voice::voice_loop::VoiceUiEvent::Speaking(text) => {
                actions.push(Action::Voice(VoiceAction::Speaking(text)));
            }
            hydra_voice::voice_loop::VoiceUiEvent::DoneSpeaking => {
                actions.push(Action::Voice(VoiceAction::SpeakingDone));
            }
            hydra_voice::voice_loop::VoiceUiEvent::WakeWordDetected => {
                actions.push(Action::Voice(VoiceAction::WakeWordDetected));
            }
            hydra_voice::voice_loop::VoiceUiEvent::SessionTimeout => {
                actions.push(Action::Voice(VoiceAction::SessionTimeout));
            }
            hydra_voice::voice_loop::VoiceUiEvent::Stopped => {}
            hydra_voice::voice_loop::VoiceUiEvent::Error(e) => {
                actions.push(Action::Voice(VoiceAction::Error(e)));
            }
        }
    }

    // O17: Also poll presence state machine for always-listening mode
    for event in voice_loop.poll_presence() {
        match event {
            hydra_voice::voice_loop::VoiceUiEvent::WakeWordDetected => {
                actions.push(Action::Voice(VoiceAction::WakeWordDetected));
            }
            hydra_voice::voice_loop::VoiceUiEvent::SessionTimeout => {
                actions.push(Action::Voice(VoiceAction::SessionTimeout));
            }
            hydra_voice::voice_loop::VoiceUiEvent::Error(e) => {
                actions.push(Action::Voice(VoiceAction::Error(e)));
            }
            _ => {}
        }
    }

    actions
}

/// Create a stream item for voice status display.
pub fn voice_status_item(text: &str) -> StreamItem {
    StreamItem::SystemNotification {
        id: uuid::Uuid::new_v4(),
        content: format!("🎤 {text}"),
        timestamp: chrono::Utc::now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_status_creates_notification() {
        let item = voice_status_item("Listening...");
        assert!(matches!(item, StreamItem::SystemNotification { .. }));
    }
}
