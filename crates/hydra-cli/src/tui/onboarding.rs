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
fn char_len(s: &str) -> usize {
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
enum Step {
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

fn draw_step(
    frame: &mut Frame,
    step: &Step,
    input: &str,
    _cursor_pos: usize,
    profile: &PersistedProfile,
    error: Option<&str>,
) {
    let area = frame.area();
    let mut lines: Vec<Line> = Vec::new();

    // Vertical centering
    let content_height: u16 = 18;
    let pad_top = area.height.saturating_sub(content_height) / 2;
    for _ in 0..pad_top {
        lines.push(Line::default());
    }

    // Hydra logo (always shown)
    lines.push(Line::from(Span::styled(
        "          ◉",
        Style::default().fg(Color::Rgb(0, 210, 210)),
    )));
    lines.push(Line::from(Span::styled(
        "        ╱   ╲",
        Style::default().fg(Color::Rgb(100, 149, 237)),
    )));
    lines.push(Line::from(Span::styled(
        "       ◉─────◉",
        Style::default().fg(Color::Rgb(100, 149, 237)),
    )));
    lines.push(Line::from(Span::styled(
        "        ╲   ╱",
        Style::default().fg(Color::Rgb(100, 149, 237)),
    )));
    lines.push(Line::from(Span::styled(
        "          ◉",
        Style::default().fg(Color::Rgb(0, 210, 210)),
    )));
    lines.push(Line::default());

    // Progress indicator
    let progress = match step {
        Step::Welcome =>      "○ ○ ○ ○ ○",
        Step::AskName =>      "● ○ ○ ○ ○",
        Step::AskWorkingDir =>"● ● ○ ○ ○",
        Step::AskApiKey =>    "● ● ● ○ ○",
        Step::SelectModel =>  "● ● ● ● ○",
        Step::Complete =>     "● ● ● ● ●",
    };
    lines.push(Line::from(Span::styled(
        format!("        {}", progress),
        Style::default().fg(Color::Rgb(100, 149, 237)),
    )));
    lines.push(Line::default());

    match step {
        Step::Welcome => {
            let version = env!("CARGO_PKG_VERSION");
            lines.push(Line::from(Span::styled(
                format!("  Welcome to Hydra v{}", version),
                Style::default().add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::default());
            lines.push(Line::from(Span::styled(
                "  The agentic orchestrator. Let's get you set up.",
                Style::default().fg(Color::DarkGray),
            )));
            lines.push(Line::default());
            lines.push(Line::from(Span::styled(
                "  Press Enter to begin →",
                Style::default().fg(Color::Rgb(100, 149, 237)),
            )));
        }
        Step::AskName => {
            lines.push(Line::from(Span::styled(
                "  What should Hydra call you?",
                Style::default().add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::default());
            lines.push(Line::from(vec![
                Span::styled("  ❯ ", Style::default().fg(Color::Rgb(100, 149, 237))),
                Span::styled(input, Style::default()),
                Span::styled("_", Style::default().fg(Color::Rgb(100, 149, 237))),
            ]));
            if let Some(err) = error {
                lines.push(Line::default());
                lines.push(Line::from(Span::styled(
                    format!("  {}", err),
                    Style::default().fg(Color::Rgb(220, 80, 80)),
                )));
            }
            lines.push(Line::default());
            lines.push(Line::from(Span::styled(
                "  Press Enter to continue",
                Style::default().fg(Color::DarkGray),
            )));
        }
        Step::AskWorkingDir => {
            lines.push(Line::from(Span::styled(
                "  Where should Hydra work?",
                Style::default().add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::styled(
                "  Enter the path to your project directory.",
                Style::default().fg(Color::DarkGray),
            )));
            lines.push(Line::default());
            lines.push(Line::from(vec![
                Span::styled("  ❯ ", Style::default().fg(Color::Rgb(100, 149, 237))),
                Span::styled(input, Style::default()),
                Span::styled("_", Style::default().fg(Color::Rgb(100, 149, 237))),
            ]));
            if let Some(err) = error {
                lines.push(Line::default());
                lines.push(Line::from(Span::styled(
                    format!("  {}", err),
                    Style::default().fg(Color::Rgb(220, 80, 80)),
                )));
            }
            lines.push(Line::default());
            lines.push(Line::from(Span::styled(
                "  Tab to autocomplete · Enter to confirm",
                Style::default().fg(Color::DarkGray),
            )));
        }
        Step::AskApiKey => {
            lines.push(Line::from(Span::styled(
                "  API Key (Anthropic or OpenAI)",
                Style::default().add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::default());

            // Mask the key for display (char-safe)
            let masked = if char_len(input) > 8 {
                let prefix: String = input.chars().take(6).collect();
                let suffix: String = input.chars().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect();
                format!("{}...{}", prefix, suffix)
            } else {
                input.to_string()
            };
            lines.push(Line::from(vec![
                Span::styled("  ❯ ", Style::default().fg(Color::Rgb(100, 149, 237))),
                Span::styled(masked, Style::default()),
                Span::styled("_", Style::default().fg(Color::Rgb(100, 149, 237))),
            ]));

            // Check env vars
            let has_env = std::env::var("ANTHROPIC_API_KEY").is_ok()
                || std::env::var("OPENAI_API_KEY").is_ok();
            if has_env {
                lines.push(Line::default());
                lines.push(Line::from(Span::styled(
                    "  ✓ API key detected in environment",
                    Style::default().fg(Color::Rgb(80, 200, 120)),
                )));
            }

            if let Some(err) = error {
                lines.push(Line::default());
                lines.push(Line::from(Span::styled(
                    format!("  {}", err),
                    Style::default().fg(Color::Rgb(220, 80, 80)),
                )));
            }
            lines.push(Line::default());
            lines.push(Line::from(Span::styled(
                "  Enter to continue · Esc to skip",
                Style::default().fg(Color::DarkGray),
            )));
        }
        Step::SelectModel => {
            lines.push(Line::from(Span::styled(
                "  Choose your default model",
                Style::default().add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::default());

            let models = [
                ("1", "Sonnet 4.6", "Fast, capable — recommended"),
                ("2", "Opus 4.6",   "Most powerful, slower"),
                ("3", "Haiku 4.5",  "Fastest, lightweight"),
            ];
            for (key, name, desc) in models {
                let selected = profile.selected_model.as_deref() == Some(match key {
                    "1" => "claude-sonnet-4-6",
                    "2" => "claude-opus-4-6",
                    "3" => "claude-haiku-4-5",
                    _ => "",
                });
                let marker = if selected { "▸" } else { " " };
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {} [{}] ", marker, key),
                        Style::default().fg(Color::Rgb(100, 149, 237)),
                    ),
                    Span::styled(
                        format!("{:<12}", name),
                        if selected {
                            Style::default().fg(Color::Rgb(0, 210, 210)).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default()
                        },
                    ),
                    Span::styled(desc, Style::default().fg(Color::DarkGray)),
                ]));
            }
            lines.push(Line::default());
            lines.push(Line::from(Span::styled(
                "  Press 1, 2, or 3 · Enter for default",
                Style::default().fg(Color::DarkGray),
            )));
        }
        Step::Complete => {
            let name = profile.user_name.as_deref().unwrap_or("user");
            lines.push(Line::from(vec![
                Span::styled("  You're all set, ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(
                    name,
                    Style::default().fg(Color::Rgb(0, 210, 210)).add_modifier(Modifier::BOLD),
                ),
                Span::styled("!", Style::default().add_modifier(Modifier::BOLD)),
            ]));
            lines.push(Line::default());

            let model = profile.selected_model.as_deref().unwrap_or("claude-sonnet-4-6");
            let model_name = match model {
                s if s.contains("opus") => "Opus 4.6",
                s if s.contains("haiku") => "Haiku 4.5",
                _ => "Sonnet 4.6",
            };
            let has_key = profile.anthropic_api_key.is_some()
                || profile.openai_api_key.is_some()
                || profile.api_key.is_some()
                || std::env::var("ANTHROPIC_API_KEY").is_ok();

            let work_dir = profile.working_directory.as_deref().unwrap_or("(current directory)");
            lines.push(Line::from(vec![
                Span::styled("  Project: ", Style::default().fg(Color::DarkGray)),
                Span::styled(work_dir, Style::default()),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Model:   ", Style::default().fg(Color::DarkGray)),
                Span::styled(model_name, Style::default()),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  API Key: ", Style::default().fg(Color::DarkGray)),
                if has_key {
                    Span::styled("✓ configured", Style::default().fg(Color::Rgb(80, 200, 120)))
                } else {
                    Span::styled("✗ not set (use ANTHROPIC_API_KEY env)", Style::default().fg(Color::Rgb(220, 80, 80)))
                },
            ]));
            lines.push(Line::default());
            lines.push(Line::from(Span::styled(
                "  Press Enter to start Hydra →",
                Style::default().fg(Color::Rgb(100, 149, 237)),
            )));
        }
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, area);
}
