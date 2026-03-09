use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::tui::app::App;
use crate::tui::theme;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let version = env!("CARGO_PKG_VERSION");

    // Connection status: Local (sisters embedded) > Server > Offline
    let (status_dot, status_text) = if app.sisters_handle.is_some() && app.connected_count > 0 {
        (Span::styled("●", theme::status_ok()), "Local")
    } else if app.server_online {
        (Span::styled("●", theme::status_ok()), "Server")
    } else {
        (Span::styled("●", theme::status_err()), "Offline")
    };

    let mut spans = vec![
        Span::styled("  ◉ ", Style::default().fg(theme::HYDRA_CYAN)),
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
        Span::raw("  "),
        status_dot,
        Span::styled(format!(" {}", status_text), theme::dim()),
    ]);

    let header_line = Line::from(spans);

    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(theme::border());

    let header = Paragraph::new(header_line).block(block);
    frame.render_widget(header, area);
}
