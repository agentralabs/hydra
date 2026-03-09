use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::tui::theme;

/// ASCII art Hydra logo for splash/loading.
#[allow(dead_code)]
const HYDRA_ASCII: &[&str] = &[
    r"  ██╗  ██╗██╗   ██╗██████╗ ██████╗  █████╗ ",
    r"  ██║  ██║╚██╗ ██╔╝██╔══██╗██╔══██╗██╔══██╗",
    r"  ███████║ ╚████╔╝ ██║  ██║██████╔╝███████║",
    r"  ██╔══██║  ╚██╔╝  ██║  ██║██╔══██╗██╔══██║",
    r"  ██║  ██║   ██║   ██████╔╝██║  ██║██║  ██║",
    r"  ╚═╝  ╚═╝   ╚═╝   ╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═╝",
];

/// Render the welcome/splash screen (used on first load or if desired).
#[allow(dead_code)]
pub fn render(frame: &mut Frame, area: Rect) {
    let version = env!("CARGO_PKG_VERSION");

    let mut lines: Vec<Line> = Vec::new();

    // Vertical centering padding
    let total_height = HYDRA_ASCII.len() + 6;
    let pad_top = area.height.saturating_sub(total_height as u16) / 2;
    for _ in 0..pad_top {
        lines.push(Line::default());
    }

    // Logo
    for line in HYDRA_ASCII {
        lines.push(Line::from(Span::styled(
            *line,
            Style::default().fg(theme::HYDRA_BLUE),
        )));
    }

    lines.push(Line::default());
    lines.push(Line::from(vec![
        Span::styled(
            format!("  v{}", version),
            Style::default()
                .fg(theme::HYDRA_CYAN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" — agentic orchestrator", theme::dim()),
    ]));
    lines.push(Line::default());
    lines.push(Line::from(Span::styled(
        "  14 sisters · 740+ tools · cognitive loop",
        theme::dim(),
    )));
    lines.push(Line::default());
    lines.push(Line::from(Span::styled(
        "  Loading...",
        Style::default().fg(theme::HYDRA_DIM),
    )));

    let para = Paragraph::new(lines).alignment(Alignment::Left);
    frame.render_widget(para, area);
}
