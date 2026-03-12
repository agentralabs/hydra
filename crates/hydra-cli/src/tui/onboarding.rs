//! TUI onboarding wizard — runs before TUI if ~/.hydra/profile.json
//! doesn't exist or onboarding_complete is false.
//!
//! Shares the same profile as desktop so users only onboard once.

use std::io;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{
    prelude::*,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use hydra_native::profile::{self, PersistedProfile};

/// Convert a char index to a byte index in a string (clamped).
fn char_to_byte(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

/// Get the number of chars in a string.
pub(crate) fn char_len(s: &str) -> usize {
    s.chars().count()
}

/// Check if onboarding is needed.
pub fn needs_onboarding() -> bool {
    match profile::load_profile() {
        Some(p) => !p.onboarding_complete,
        None => true,
    }
}

/// Onboarding step.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Step {
    Welcome,
    AskName,
    AskWorkingDir,
    AskApiKey,
    SelectModel,
    Complete,
}

/// Run the onboarding wizard inside the already-initialized terminal.
/// Returns the completed profile.
pub fn run_onboarding(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> io::Result<PersistedProfile> {
    let mut profile = profile::load_profile().unwrap_or_default();
    let mut step = Step::Welcome;
    let mut input = String::new();
    let mut cursor_pos: usize = 0;
    let mut error_msg: Option<String> = None;

    // Pre-fill name from env if available
    if profile.user_name.is_none() {
        let env_name = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_default();
        if !env_name.is_empty() {
            input = env_name;
            cursor_pos = char_len(&input);
        }
    }

    loop {
        // Draw current step
        terminal.draw(|frame| {
            draw_step(frame, &step, &input, cursor_pos, &profile, error_msg.as_deref());
        })?;

        // Handle input
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // Ctrl+C exits
                if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('c') {
                    // Save partial progress but DON'T mark as complete
                    // so onboarding runs again next time
                    profile::save_profile(&profile);
                    return Err(io::Error::new(io::ErrorKind::Interrupted, "User cancelled onboarding"));
                }

                match step {
                    Step::Welcome => {
                        if key.code == KeyCode::Enter {
                            step = Step::AskName;
                            // Pre-fill input with env name or existing profile name
                            input = profile.user_name.clone().unwrap_or_else(|| {
                                std::env::var("USER")
                                    .or_else(|_| std::env::var("USERNAME"))
                                    .unwrap_or_default()
                            });
                            cursor_pos = char_len(&input);
                            error_msg = None;
                        }
                    }
                    Step::AskName => match key.code {
                        KeyCode::Enter => {
                            let name = input.trim().to_string();
                            if name.is_empty() {
                                error_msg = Some("Name cannot be empty.".to_string());
                            } else {
                                profile.user_name = Some(name);
                                step = Step::AskWorkingDir;
                                // Pre-fill with current directory or saved working_directory
                                input = profile.working_directory.clone()
                                    .or_else(|| std::env::current_dir().ok().map(|p| p.display().to_string()))
                                    .unwrap_or_else(|| "~".to_string());
                                cursor_pos = char_len(&input);
                                error_msg = None;
                            }
                        }
                        KeyCode::Char(c) => {
                            let byte_idx = char_to_byte(&input, cursor_pos);
                            input.insert(byte_idx, c);
                            cursor_pos += 1;
                            error_msg = None;
                        }
                        KeyCode::Backspace => {
                            if cursor_pos > 0 {
                                cursor_pos -= 1;
                                let byte_idx = char_to_byte(&input, cursor_pos);
                                input.remove(byte_idx);
                            }
                        }
                        KeyCode::Left => { cursor_pos = cursor_pos.saturating_sub(1); }
                        KeyCode::Right => { if cursor_pos < char_len(&input) { cursor_pos += 1; } }
                        _ => {}
                    },
                    Step::AskWorkingDir => match key.code {
                        KeyCode::Enter => {
                            let dir = input.trim().to_string();
                            // Expand ~ to home
                            let expanded = if dir.starts_with('~') {
                                let home = std::env::var("HOME").unwrap_or_default();
                                dir.replacen('~', &home, 1)
                            } else {
                                dir.clone()
                            };
                            let path = std::path::Path::new(&expanded);
                            if expanded.is_empty() {
                                // Use current directory
                                profile.working_directory = std::env::current_dir()
                                    .ok().map(|p| p.display().to_string());
                                step = Step::AskApiKey;
                                input = profile.anthropic_api_key.clone()
                                    .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
                                    .unwrap_or_default();
                                cursor_pos = char_len(&input);
                                error_msg = None;
                            } else if path.is_dir() {
                                profile.working_directory = Some(expanded);
                                step = Step::AskApiKey;
                                input = profile.anthropic_api_key.clone()
                                    .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
                                    .unwrap_or_default();
                                cursor_pos = char_len(&input);
                                error_msg = None;
                            } else {
                                error_msg = Some(format!("Directory not found: {}", expanded));
                            }
                        }
                        KeyCode::Tab => {
                            // Basic tab completion for directories
                            let dir = input.trim();
                            let expanded = if dir.starts_with('~') {
                                let home = std::env::var("HOME").unwrap_or_default();
                                dir.replacen('~', &home, 1)
                            } else {
                                dir.to_string()
                            };
                            let path = std::path::Path::new(&expanded);
                            let (parent, prefix) = if path.is_dir() {
                                (path.to_path_buf(), "".to_string())
                            } else {
                                let parent = path.parent().unwrap_or(std::path::Path::new(".")).to_path_buf();
                                let prefix = path.file_name()
                                    .map(|f| f.to_string_lossy().to_string())
                                    .unwrap_or_default();
                                (parent, prefix)
                            };
                            if let Ok(entries) = std::fs::read_dir(&parent) {
                                let matches: Vec<String> = entries
                                    .filter_map(|e| e.ok())
                                    .filter(|e| e.path().is_dir())
                                    .filter(|e| {
                                        let name = e.file_name().to_string_lossy().to_string();
                                        name.starts_with(&prefix) && !name.starts_with('.')
                                    })
                                    .map(|e| e.path().display().to_string())
                                    .collect();
                                if matches.len() == 1 {
                                    input = matches[0].clone();
                                    if !input.ends_with('/') {
                                        input.push('/');
                                    }
                                    cursor_pos = char_len(&input);
                                }
                            }
                        }
                        KeyCode::Char(c) => {
                            let byte_idx = char_to_byte(&input, cursor_pos);
                            input.insert(byte_idx, c);
                            cursor_pos += 1;
                            error_msg = None;
                        }
                        KeyCode::Backspace => {
                            if cursor_pos > 0 {
                                cursor_pos -= 1;
                                let byte_idx = char_to_byte(&input, cursor_pos);
                                input.remove(byte_idx);
                            }
                        }
                        KeyCode::Left => { cursor_pos = cursor_pos.saturating_sub(1); }
                        KeyCode::Right => { if cursor_pos < char_len(&input) { cursor_pos += 1; } }
                        _ => {}
                    },
                    Step::AskApiKey => match key.code {
                        KeyCode::Enter => {
                            let key_val = input.trim().to_string();
                            if key_val.is_empty() {
                                // Allow skipping — user might have env var set
                            } else if key_val.starts_with("sk-ant-") {
                                profile.anthropic_api_key = Some(key_val);
                            } else if key_val.starts_with("sk-") {
                                profile.openai_api_key = Some(key_val);
                            } else {
                                profile.api_key = Some(key_val);
                            }
                            step = Step::SelectModel;
                            input.clear();
                            cursor_pos = 0;
                            error_msg = None;
                        }
                        KeyCode::Esc => {
                            // Skip API key step
                            step = Step::SelectModel;
                            input.clear();
                            cursor_pos = 0;
                            error_msg = None;
                        }
                        KeyCode::Char(c) => {
                            let byte_idx = char_to_byte(&input, cursor_pos);
                            input.insert(byte_idx, c);
                            cursor_pos += 1;
                            error_msg = None;
                        }
                        KeyCode::Backspace => {
                            if cursor_pos > 0 {
                                cursor_pos -= 1;
                                let byte_idx = char_to_byte(&input, cursor_pos);
                                input.remove(byte_idx);
                            }
                        }
                        KeyCode::Left => { cursor_pos = cursor_pos.saturating_sub(1); }
                        KeyCode::Right => { if cursor_pos < char_len(&input) { cursor_pos += 1; } }
                        _ => {}
                    },
                    Step::SelectModel => match key.code {
                        KeyCode::Char('1') => {
                            profile.selected_model = Some("claude-sonnet-4-6".to_string());
                            step = Step::Complete;
                        }
                        KeyCode::Char('2') => {
                            profile.selected_model = Some("claude-opus-4-6".to_string());
                            step = Step::Complete;
                        }
                        KeyCode::Char('3') => {
                            profile.selected_model = Some("claude-haiku-4-5".to_string());
                            step = Step::Complete;
                        }
                        KeyCode::Enter => {
                            // Default to Sonnet
                            if profile.selected_model.is_none() {
                                profile.selected_model = Some("claude-sonnet-4-6".to_string());
                            }
                            step = Step::Complete;
                        }
                        _ => {}
                    },
                    Step::Complete => {
                        if key.code == KeyCode::Enter {
                            // Save and exit onboarding
                            profile.onboarding_complete = true;
                            profile::save_profile(&profile);
                            return Ok(profile);
                        }
                    }
                }
            }
        }
    }
}

// draw_step is extracted to onboarding_draw.rs for compilation memory reduction
use super::onboarding_draw::draw_step;
