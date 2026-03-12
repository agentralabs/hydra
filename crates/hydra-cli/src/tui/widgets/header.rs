use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::tui::app::App;
use crate::tui::theme;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let version = env!("CARGO_PKG_VERSION");

    // Connection status: Local (sisters embedded) > Server > Offline
    let (status_dot, status_text) = if app.sisters_handle.is_some() && app.connected_count > 0 {
        (Span::styled("●", theme::status_ok()), " Local")
    } else if app.server_online {
        (Span::styled("●", theme::status_ok()), " Server")
    } else {
        (Span::styled("●", theme::status_err()), " Offline")
    };

    // Header line: ── Hydra v1.1.0 · project (branch) · Model · tools · ● status ──
    let mut spans = vec![
        Span::styled("── ", theme::dim()),
        Span::styled(
            format!("Hydra v{}", version),
            Style::default()
                .fg(theme::HYDRA_FG)
                .add_modifier(Modifier::BOLD),
        ),
    ];

    // Show project name if detected
    if let Some(ref info) = app.project_info {
        spans.push(Span::styled(" · ", theme::dim()));
        spans.push(Span::styled(
            format!("{} {}", info.kind.icon(), info.name),
            Style::default().fg(theme::HYDRA_BLUE),
        ));
        if let Some(ref branch) = info.git_branch {
            spans.push(Span::styled(
                format!(" ({})", branch),
                Style::default().fg(theme::HYDRA_GREEN),
            ));
        }
    }

    spans.extend([
        Span::styled(" · ", theme::dim()),
        Span::styled(&app.model_name, Style::default().fg(theme::HYDRA_PURPLE)),
        Span::styled(" · ", theme::dim()),
        Span::styled(
            format!("{}+ tools", app.tool_count),
            Style::default().fg(theme::HYDRA_DIM),
        ),
        Span::styled(" · ", theme::dim()),
        Span::styled(
            format!("{}/{} sisters", app.connected_count, app.total_sisters),
            if app.connected_count == app.total_sisters {
                Style::default().fg(theme::HYDRA_GREEN)
            } else {
                Style::default().fg(theme::HYDRA_YELLOW)
            },
        ),
        Span::styled(" · ", theme::dim()),
        Span::styled(
            format!("{}%", app.health_pct),
            if app.health_pct >= 90 {
                Style::default().fg(theme::HYDRA_GREEN)
            } else if app.health_pct >= 50 {
                Style::default().fg(theme::HYDRA_YELLOW)
            } else {
                Style::default().fg(theme::HYDRA_RED)
            },
        ),
        Span::styled(" · ", theme::dim()),
        status_dot,
        Span::styled(status_text, theme::dim()),
    ]);

    // Fill remaining width with ── to create a clean horizontal rule
    let content_width: usize = spans.iter().map(|s| s.content.chars().count()).sum();
    let remaining = (area.width as usize).saturating_sub(content_width + 1);
    if remaining > 2 {
        spans.push(Span::styled(" ", Style::default()));
        spans.push(Span::styled(
            "─".repeat(remaining),
            theme::dim(),
        ));
    }

    let header_line = Line::from(spans);
    let header = Paragraph::new(header_line);
    frame.render_widget(header, area);
}
