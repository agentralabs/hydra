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

    let server_dot = if app.server_online {
        Span::styled("●", theme::status_ok())
    } else {
        Span::styled("●", theme::status_err())
    };

    let status_text = if app.server_online {
        "online"
    } else {
        "offline"
    };

    let header_line = Line::from(vec![
        Span::styled("  ◉ ", Style::default().fg(theme::HYDRA_CYAN)),
        Span::styled(
            format!("Hydra v{}", version),
            Style::default()
                .fg(theme::HYDRA_FG)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" · ", theme::dim()),
        Span::styled(&app.model_name, Style::default().fg(theme::HYDRA_PURPLE)),
        Span::styled(" · ", theme::dim()),
        Span::styled(
            format!("{}+ tools", app.tool_count),
            Style::default().fg(theme::HYDRA_DIM),
        ),
        Span::styled(" · ", theme::dim()),
        Span::styled(&app.working_dir, Style::default().fg(theme::HYDRA_DIM)),
        Span::raw("  "),
        server_dot,
        Span::styled(format!(" {}", status_text), theme::dim()),
    ]);

    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(theme::border());

    let header = Paragraph::new(header_line).block(block);
    frame.render_widget(header, area);
}
