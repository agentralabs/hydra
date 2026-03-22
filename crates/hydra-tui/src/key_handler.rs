//! Key handler — processes keyboard input in conversation mode.
//!
//! Separated from binary to keep hydra_tui.rs under 400 lines.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::stream_types::StreamItem;
use crate::HydraTui;

/// Process voice events from the VoiceLoop (call in TUI event loop tick).
pub fn poll_voice(
    tui: &mut HydraTui,
    voice: &mut hydra_voice::VoiceLoop,
    cognitive: &mut hydra_kernel::engine::CognitiveLoop,
    rt: &tokio::runtime::Runtime,
) {
    // Check if TTS finished
    voice.check_tts_done();

    // Poll mic events
    let events = voice.poll();
    for event in events {
        match event {
            hydra_voice::voice_loop::VoiceUiEvent::Listening => {
                tui.push_item(StreamItem::SystemNotification {
                    id: uuid::Uuid::new_v4(),
                    content: "● Listening...".into(),
                    timestamp: chrono::Utc::now(),
                });
            }
            hydra_voice::voice_loop::VoiceUiEvent::PartialTranscript(text) => {
                // Update a "live" notification (replace last system notification)
                tui.push_item(StreamItem::SystemNotification {
                    id: uuid::Uuid::new_v4(),
                    content: format!("● {text}"),
                    timestamp: chrono::Utc::now(),
                });
            }
            hydra_voice::voice_loop::VoiceUiEvent::FinalTranscript(text) => {
                // Submit as if user typed it
                tui.push_item(StreamItem::UserMessage {
                    id: uuid::Uuid::new_v4(),
                    text: text.clone(),
                    timestamp: chrono::Utc::now(),
                });
                tui.start_thinking();

                let output = match std::panic::catch_unwind(
                    std::panic::AssertUnwindSafe(|| rt.block_on(cognitive.cycle_full(&text))),
                ) {
                    Ok(o) => o,
                    Err(_) => hydra_kernel::engine::CycleOutput {
                        response: "[Hydra recovered from error]".into(),
                        enrichments: std::collections::HashMap::new(),
                        tokens_used: 0,
                        duration_ms: 0,
                        path: "error".into(),
                        success: false,
                    },
                };

                tui.stop_thinking();

                // Surface enrichments
                let surface = crate::enrichment_bridge::surface_enrichments(
                    &output.enrichments,
                    output.tokens_used,
                );
                for item in surface.items {
                    tui.push_item(item);
                }

                tui.push_item(StreamItem::AssistantText {
                    id: uuid::Uuid::new_v4(),
                    text: output.response.clone(),
                    timestamp: chrono::Utc::now(),
                });

                tui.status.tokens += output.tokens_used as u64;

                // Speak the response back
                voice.speak_response(&output.response);

                tui.stream.scroll_to_bottom();
            }
            hydra_voice::voice_loop::VoiceUiEvent::Speaking(_) => {}
            hydra_voice::voice_loop::VoiceUiEvent::DoneSpeaking => {}
            hydra_voice::voice_loop::VoiceUiEvent::Stopped => {
                tui.push_item(StreamItem::SystemNotification {
                    id: uuid::Uuid::new_v4(),
                    content: "● Recording stopped".into(),
                    timestamp: chrono::Utc::now(),
                });
            }
            hydra_voice::voice_loop::VoiceUiEvent::Error(e) => {
                tui.push_item(StreamItem::SystemNotification {
                    id: uuid::Uuid::new_v4(),
                    content: format!("● Voice error: {e}"),
                    timestamp: chrono::Utc::now(),
                });
            }
        }
    }
}

/// Request to start streaming — returned by handle_cockpit_key when Enter is pressed.
pub struct StreamRequest {
    pub prepared: hydra_kernel::engine::PreparedCycle,
}

/// Handle a key event in conversation/companion mode.
/// Returns Some(StreamRequest) if the main loop should start streaming.
pub fn handle_cockpit_key(
    key: KeyEvent,
    tui: &mut HydraTui,
    cognitive: &mut hydra_kernel::engine::CognitiveLoop,
    rt: &tokio::runtime::Runtime,
    voice: &mut hydra_voice::VoiceLoop,
) -> Option<StreamRequest> {
    // Ctrl+V: toggle voice capture
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('v') {
        if voice.is_listening() {
            voice.stop_listening();
            tui.push_item(StreamItem::SystemNotification {
                id: uuid::Uuid::new_v4(),
                content: "● Voice capture stopped".into(),
                timestamp: chrono::Utc::now(),
            });
        } else if voice.can_listen() {
            match voice.start_listening() {
                Ok(()) => {
                    tui.push_item(StreamItem::SystemNotification {
                        id: uuid::Uuid::new_v4(),
                        content: format!(
                            "● Listening on {} — speak now",
                            voice.mic_name()
                        ),
                        timestamp: chrono::Utc::now(),
                    });
                }
                Err(e) => {
                    tui.push_item(StreamItem::SystemNotification {
                        id: uuid::Uuid::new_v4(),
                        content: format!("● Mic error: {e}"),
                        timestamp: chrono::Utc::now(),
                    });
                }
            }
        } else {
            tui.push_item(StreamItem::SystemNotification {
                id: uuid::Uuid::new_v4(),
                content: "● No microphone found. Connect a mic and try again.".into(),
                timestamp: chrono::Utc::now(),
            });
        }
        return None;
    }

    // Handle search mode (Ctrl+R active)
    if tui.input.is_searching() {
        match key.code {
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                tui.input.search_next();
            }
            KeyCode::Enter | KeyCode::Tab => {
                tui.input.search_accept();
            }
            KeyCode::Esc => {
                tui.input.search_cancel();
            }
            KeyCode::Backspace => {
                tui.input.search_backspace();
            }
            KeyCode::Char(c) => {
                tui.input.search_insert(c);
            }
            _ => {}
        }
        return None;
    }

    // Ctrl+key shortcuts (non-voice)
    if key.modifiers.contains(KeyModifiers::CONTROL)
        && handle_ctrl_key(key.code, tui)
    {
        return None;
    }

    // Alt+key shortcuts
    if key.modifiers.contains(KeyModifiers::ALT) && handle_alt_key(key.code, tui) {
        return None;
    }

    match key.code {
        KeyCode::Esc => {
            tui.alerts.dismiss();
        }
        KeyCode::Enter => return handle_enter(tui, cognitive, rt),
        KeyCode::Backspace => tui.input.backspace(),
        KeyCode::Delete => tui.input.delete(),
        KeyCode::Left => tui.input.move_left(),
        KeyCode::Right => tui.input.move_right(),
        KeyCode::Home => tui.input.move_home(),
        KeyCode::End => tui.input.move_end(),
        KeyCode::Up => {
            if !tui.input.history_up() {
                tui.stream.scroll_up(1);
            }
        }
        KeyCode::Down => {
            if !tui.input.history_down() {
                tui.stream.scroll_down(1);
            }
        }
        KeyCode::PageUp => tui.stream.scroll_up(10),
        KeyCode::PageDown => tui.stream.scroll_down(10),
        KeyCode::Char(c) => tui.input.insert(c),
        _ => {}
    }
    None
}

/// Handle Ctrl+key combinations (non-voice). Returns true if handled.
fn handle_ctrl_key(code: KeyCode, tui: &mut HydraTui) -> bool {
    match code {
        KeyCode::Char('c') => {
            if tui.input.is_empty() {
                tui.quit();
            } else {
                tui.input.clear();
            }
            true
        }
        KeyCode::Char('d') => {
            tui.quit();
            true
        }
        KeyCode::Char('v') => true, // handled above via voice loop
        KeyCode::Char('a') => { tui.input.move_home(); true }
        KeyCode::Char('e') => { tui.input.move_end(); true }
        KeyCode::Char('k') => { tui.input.kill_to_end(); true }
        KeyCode::Char('u') => { tui.input.kill_line(); true }
        KeyCode::Char('y') => { tui.input.yank(); true }
        KeyCode::Char('w') => { tui.input.delete_word_backward(); true }
        KeyCode::Char('l') => { tui.stream.clear(); true }
        KeyCode::Char('r') => { tui.input.start_search(); true }
        KeyCode::Char('b') => {
            // Toggle companion panel mode
            tui.cockpit.toggle_companion_panel();
            true
        }
        KeyCode::Char('t') => {
            // Show task summary in stream
            tui.push_item(crate::stream_types::StreamItem::SystemNotification {
                id: uuid::Uuid::new_v4(),
                content: "Tasks: use /companion for status.".into(),
                timestamp: chrono::Utc::now(),
            });
            true
        }
        _ => false,
    }
}

/// Handle Alt+key combinations. Returns true if handled.
fn handle_alt_key(code: KeyCode, tui: &mut HydraTui) -> bool {
    match code {
        KeyCode::Char('b') => { tui.input.move_word_backward(); true }
        KeyCode::Char('f') => { tui.input.move_word_forward(); true }
        _ => false,
    }
}

/// Handle Enter key — submit input, prepare streaming cycle or slash command.
/// Returns Some(StreamRequest) if streaming should start.
fn handle_enter(
    tui: &mut HydraTui,
    cognitive: &mut hydra_kernel::engine::CognitiveLoop,
    _rt: &tokio::runtime::Runtime,
) -> Option<StreamRequest> {
    let text = tui.input.submit();
    if text.is_empty() {
        return None;
    }

    // Slash commands — no streaming needed
    if text.starts_with('/') {
        if text.trim() == "/clear" {
            tui.stream.clear();
            return None;
        }
        if text.trim() == "/quit" || text.trim() == "/exit" {
            tui.quit();
            return None;
        }
        let items = crate::commands::dispatch(
            &text,
            cognitive,
            &tui.stream,
            tui.status.tokens,
            tui.status.session_minutes,
            tui.companion_channel.as_ref(),
        );
        for item in items {
            tui.push_item(item);
        }
        tui.stream.scroll_to_bottom();
        return None;
    }

    // Large paste check — if text is very long, show a summary prefix
    let display_text = if text.len() > 2000 {
        let line_count = text.lines().count();
        format!(
            "[Pasted {}KB, {} lines]\n{}...",
            text.len() / 1024,
            line_count,
            text.chars().take(200).collect::<String>()
        )
    } else {
        text.clone()
    };

    tui.push_item(StreamItem::UserMessage {
        id: uuid::Uuid::new_v4(),
        text: display_text,
        timestamp: chrono::Utc::now(),
    });

    // Prepare the cycle: perceive, route, prompt, enrichments (fast, sync)
    let prepared = cognitive.prepare_cycle(&text);

    // Surface enrichments BEFORE response starts streaming
    let surface = crate::enrichment_bridge::surface_enrichments(
        &prepared.enrichments,
        0, // tokens not known yet
    );
    for item in surface.items {
        tui.push_item(item);
    }
    for alert in surface.alerts {
        tui.alerts.push(alert);
    }
    if tui.alerts.should_bell() {
        print!("\x07");
        if let Some(alert_text) = tui.alerts.voice_text() {
            let engine = hydra_voice::TtsEngine::detect();
            if engine.is_available() {
                hydra_voice::native_tts::speak_async(&engine, alert_text);
            }
        }
    }

    // If resolved without LLM (0 tokens — compiled pattern), show immediately
    if let Some(resolved) = prepared.resolved_text.clone() {
        tui.push_item(StreamItem::AssistantText {
            id: uuid::Uuid::new_v4(),
            text: resolved.clone(),
            timestamp: chrono::Utc::now(),
        });
        tui.push_item(StreamItem::Blank);
        cognitive.finalize_streaming(prepared, &resolved, 0);
        tui.stream.scroll_to_bottom();
        return None;
    }

    tui.start_thinking();

    // Push placeholder AssistantText that will be updated by streaming
    tui.push_item(StreamItem::AssistantText {
        id: uuid::Uuid::new_v4(),
        text: String::new(),
        timestamp: chrono::Utc::now(),
    });

    tui.stream.scroll_to_bottom();

    // Return StreamRequest — main loop will start the async LLM call
    Some(StreamRequest { prepared })
}
