//! Welcome screen — the initial view before conversation starts.

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use crate::constants;

/// Data for the welcome screen.
#[derive(Debug, Clone)]
pub struct WelcomeScreen {
    /// Hydra version string.
    pub version: String,
    /// Whether the kernel has booted.
    pub kernel_ready: bool,
    /// Number of sisters connected.
    pub sisters_connected: usize,
    /// Boot status message.
    pub boot_status: String,
}

impl WelcomeScreen {
    /// Create a new welcome screen with defaults.
    pub fn new() -> Self {
        Self {
            version: String::from("0.1.0"),
            kernel_ready: false,
            sisters_connected: 0,
            boot_status: String::from("Initializing..."),
        }
    }

    /// Render the welcome screen as a list of lines.
    pub fn to_lines(&self) -> Vec<Line<'static>> {
        let (ar, ag, ab) = constants::WELCOME_ACCENT;
        let accent = Color::Rgb(ar, ag, ab);
        let (fr, fg, fb) = constants::ASSISTANT_TEXT_COLOR;
        let text_color = Color::Rgb(fr, fg, fb);

        let ready_indicator = if self.kernel_ready {
            Span::styled("● online", Style::default().fg(Color::Rgb(74, 222, 128)))
        } else {
            Span::styled("○ booting", Style::default().fg(Color::Rgb(251, 191, 36)))
        };

        vec![
            Line::from(""),
            Line::from(Span::styled(
                "  ╦ ╦╦ ╦╔╦╗╦═╗╔═╗",
                Style::default().fg(accent),
            )),
            Line::from(Span::styled(
                "  ╠═╣╚╦╝ ║║╠╦╝╠═╣",
                Style::default().fg(accent),
            )),
            Line::from(Span::styled(
                "  ╩ ╩ ╩ ═╩╝╩╚═╩ ╩",
                Style::default().fg(accent),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    format!("  v{}", self.version),
                    Style::default().fg(text_color),
                ),
                Span::raw("  "),
                ready_indicator,
            ]),
            Line::from(Span::styled(
                format!("  Sisters: {}", self.sisters_connected),
                Style::default().fg(text_color),
            )),
            Line::from(Span::styled(
                format!("  {}", self.boot_status),
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
        ]
    }
}

impl Default for WelcomeScreen {
    fn default() -> Self {
        Self::new()
    }
}
