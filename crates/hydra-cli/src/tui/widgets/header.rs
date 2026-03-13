use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::tui::app::App;
use crate::tui::theme;

/// Persistent header bar — always visible at top of screen.
/// Style: ── Hydra v1.1.0 · project (branch) · model · 14/14 sisters ── ● Online ──
pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let version = env!("CARGO_PKG_VERSION");

    let (status_dot, status_label) = if app.sisters_handle.is_some() && app.connected_count > 0 {
        ("●", "Local")
    } else if app.server_online {
        ("●", "Server")
    } else {
        ("●", "Offline")
    };
    let status_color = if app.connected_count > 0 || app.server_online {
        theme::HYDRA_GREEN
    } else {
        theme::HYDRA_RED
    };

    let mut spans = vec![
        Span::styled("╭─ ", theme::dim()),
        Span::styled(format!("Hydra v{}", version), Style::default().fg(theme::HYDRA_BLUE).add_modifier(Modifier::BOLD)),
    ];

    // Project + branch
    if let Some(ref info) = app.project_info {
        spans.push(Span::styled(" · ", theme::dim()));
        spans.push(Span::styled(info.name.clone(), Style::default().fg(theme::HYDRA_FG)));
        if let Some(ref branch) = info.git_branch {
            spans.push(Span::styled(format!(" ({})", branch), Style::default().fg(theme::HYDRA_GREEN)));
        }
    }

    // Model
    spans.push(Span::styled(" · ", theme::dim()));
    spans.push(Span::styled(&app.model_name, Style::default().fg(theme::HYDRA_PURPLE)));

    // Sisters count
    spans.push(Span::styled(" · ", theme::dim()));
    let sister_style = if app.connected_count == app.total_sisters {
        Style::default().fg(theme::HYDRA_GREEN)
    } else {
        Style::default().fg(theme::HYDRA_YELLOW)
    };
    spans.push(Span::styled(format!("{}/{} sisters", app.connected_count, app.total_sisters), sister_style));

    // Status dot
    spans.push(Span::styled(" · ", theme::dim()));
    spans.push(Span::styled(status_dot, Style::default().fg(status_color)));
    spans.push(Span::styled(format!(" {}", status_label), theme::dim()));

    // Fill remaining width with ─
    let content_width: usize = spans.iter().map(|s| s.content.chars().count()).sum();
    let remaining = (area.width as usize).saturating_sub(content_width + 1);
    if remaining > 2 {
        spans.push(Span::styled(" ", Style::default()));
        spans.push(Span::styled("─".repeat(remaining), theme::dim()));
    }

    let header = Paragraph::new(Line::from(spans));
    frame.render_widget(header, area);
}
