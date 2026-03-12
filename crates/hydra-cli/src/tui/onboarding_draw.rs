use ratatui::{
    prelude::*,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use hydra_native::profile::PersistedProfile;

use super::onboarding::{Step, char_len};

pub(crate) fn draw_step(
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
