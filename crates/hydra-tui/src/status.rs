//! Status line — the bottom bar showing system state.
//!
//! Layout from spec:
//! `◈ Hydra  session:42m  tasks:3  V=0.42      tokens:12k  ◑ Forging`
//!
//! Token display: count only. NEVER cost ($). Per spec.

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use crate::theme;
use crate::verb::ThinkingVerbState;

/// Data for the status line at the bottom of the cockpit.
#[derive(Debug, Clone)]
pub struct StatusLine {
    /// Current Lyapunov stability value.
    pub lyapunov: f64,
    /// Total tokens consumed this session.
    pub tokens: u64,
    /// Current thinking verb state.
    pub verb_state: ThinkingVerbState,
    /// Session duration in minutes.
    pub session_minutes: u64,
    /// Number of active tasks.
    pub active_tasks: usize,
    /// Active persona name (None = core, no brackets).
    pub persona: Option<String>,
}

impl StatusLine {
    /// Create a new status line with defaults.
    pub fn new() -> Self {
        Self {
            lyapunov: 1.0,
            tokens: 0,
            verb_state: ThinkingVerbState::default(),
            session_minutes: 0,
            active_tasks: 0,
            persona: None,
        }
    }

    /// Format token count per spec: under 1k exact, else Nk. Never cost.
    fn format_tokens(tokens: u64) -> String {
        if tokens < 1_000 {
            format!("{tokens}")
        } else {
            let k = tokens as f64 / 1_000.0;
            if k < 10.0 {
                format!("{k:.1}k")
            } else {
                format!("{}k", tokens / 1_000)
            }
        }
    }

    /// Format the status line as a ratatui Line for the bottom bar.
    pub fn format(&self) -> Line<'static> {
        let t = theme::current();
        let bg = t.status_bar_bg;
        let fg = t.status_bar_fg;
        let base_style = Style::default().fg(fg).bg(bg);

        // Lyapunov color: > 0.3 green, > 0 yellow, <= 0 red.
        let lyap_color = if self.lyapunov > 0.3 {
            Color::Rgb(74, 222, 128)
        } else if self.lyapunov > 0.0 {
            Color::Rgb(251, 191, 36)
        } else {
            Color::Rgb(248, 113, 113)
        };

        let entity = match &self.persona {
            Some(p) => format!("◈ Hydra [{p}]"),
            None => "◈ Hydra".to_string(),
        };

        let mut spans = vec![
            Span::styled(format!(" {entity}  "), base_style),
            Span::styled(format!("session:{}m  ", self.session_minutes), base_style),
            Span::styled(format!("tasks:{}  ", self.active_tasks), base_style),
            Span::styled("V=", base_style),
            Span::styled(
                format!("{:.2}", self.lyapunov),
                Style::default().fg(lyap_color).bg(bg),
            ),
        ];

        // Right section: tokens + thinking verb.
        let token_str = Self::format_tokens(self.tokens);
        spans.push(Span::styled(format!("  tokens:{token_str}"), base_style));

        // Add thinking verb if active.
        if self.verb_state.is_active() {
            let verb_color = self.verb_state.context().color();
            spans.push(Span::styled("  ", base_style));
            spans.push(Span::styled(
                self.verb_state.status_display(),
                Style::default().fg(verb_color).bg(bg),
            ));
        }

        spans.push(Span::styled(" ", base_style));

        Line::from(spans)
    }
}

impl Default for StatusLine {
    fn default() -> Self {
        Self::new()
    }
}
