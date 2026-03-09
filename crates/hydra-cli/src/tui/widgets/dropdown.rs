use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::tui::app::App;
use crate::tui::theme;

/// Render the slash command dropdown above the input bar.
///
/// ```text
/// ┌──────────────────────────────────────────────┐
/// │  /sisters    Show sister diagnostic table     │
/// │▸ /sidebar    Toggle sidebar                   │
/// ├──────────────────────────────────────────────┤
/// │ INSERT › /si_                                 │
/// └──────────────────────────────────────────────┘
/// ```
pub fn render(frame: &mut Frame, app: &App, input_area: Rect) {
    if !app.command_dropdown.visible {
        return;
    }

    let items = &app.command_dropdown.filtered;
    let count = app.command_dropdown.display_count();
    if count == 0 {
        return;
    }

    // Position dropdown above the input bar
    let height = count as u16 + 2; // +2 for borders
    let y = input_area.y.saturating_sub(height);
    let width = input_area.width.min(52);
    let x = input_area.x + 1; // slight indent

    let area = Rect::new(x, y, width, height);

    // Clear the area behind the dropdown
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::HYDRA_BLUE))
        .style(Style::default().bg(ratatui::style::Color::Rgb(30, 32, 40)));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::with_capacity(count);
    for (i, cmd) in items.iter().take(count).enumerate() {
        let is_selected = i == app.command_dropdown.selected;

        let marker = if is_selected { "▸ " } else { "  " };
        let name_style = if is_selected {
            Style::default()
                .fg(theme::HYDRA_CYAN)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::HYDRA_BLUE)
        };
        let desc_style = if is_selected {
            Style::default().fg(theme::HYDRA_FG)
        } else {
            Style::default().fg(theme::HYDRA_DIM)
        };

        lines.push(Line::from(vec![
            Span::styled(
                marker,
                if is_selected {
                    Style::default().fg(theme::HYDRA_CYAN)
                } else {
                    Style::default().fg(theme::HYDRA_DIM)
                },
            ),
            Span::styled(format!("{:<12}", cmd.name), name_style),
            Span::styled(cmd.description, desc_style),
        ]));
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, inner);
}
