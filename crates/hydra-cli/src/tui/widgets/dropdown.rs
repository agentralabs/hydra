use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::tui::app::App;
use crate::tui::theme;

/// Render the slash command dropdown above the input bar.
///
/// High-contrast design with scrolling support:
/// - Unselected: bright white text on dark background
/// - Selected: black text on cyan background (inverted)
/// - Scrolls to keep selection visible
pub fn render(frame: &mut Frame, app: &App, input_area: Rect) {
    if !app.command_dropdown.visible {
        return;
    }

    let visible = app.command_dropdown.visible_items();
    if visible.is_empty() {
        return;
    }

    let count = visible.len();
    let total = app.command_dropdown.filtered.len();
    let scroll = app.command_dropdown.scroll;

    // Position dropdown above the input bar
    let height = count as u16 + 2; // +2 for borders
    let y = input_area.y.saturating_sub(height);
    let width = input_area.width.min(56);
    let x = input_area.x + 1;

    let area = Rect::new(x, y, width, height);

    // Clear the area behind the dropdown
    frame.render_widget(Clear, area);

    // Dark background with blue border
    let bg = Color::Rgb(25, 25, 35);

    // Show scroll indicators in title
    let title = if total > count {
        let pos = scroll + 1;
        let end = (scroll + count).min(total);
        format!(" {}-{} of {} ", pos, end, total)
    } else {
        String::new()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::HYDRA_BLUE))
        .title(Span::styled(title, Style::default().fg(theme::HYDRA_DIM)))
        .style(Style::default().bg(bg));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::with_capacity(count);
    for (i, cmd) in visible.iter().enumerate() {
        let absolute_idx = scroll + i;
        let is_selected = absolute_idx == app.command_dropdown.selected;

        if is_selected {
            // Selected: black on cyan — high contrast, unmistakable
            let sel_bg = theme::HYDRA_CYAN;
            lines.push(Line::from(vec![
                Span::styled("▸ ", Style::default().fg(Color::White).bg(sel_bg).add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!("{:<13}", cmd.name),
                    Style::default().fg(Color::Black).bg(sel_bg).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    cmd.description,
                    Style::default().fg(Color::Rgb(40, 40, 50)).bg(sel_bg),
                ),
                // Fill remaining width with the selection background
                Span::styled(
                    " ".repeat(20),
                    Style::default().bg(sel_bg),
                ),
            ]));
        } else {
            // Unselected: bright white name, light gray description on dark bg
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default().bg(bg)),
                Span::styled(
                    format!("{:<13}", cmd.name),
                    Style::default().fg(Color::White).bg(bg),
                ),
                Span::styled(
                    cmd.description,
                    Style::default().fg(Color::Gray).bg(bg),
                ),
            ]));
        }
    }

    // Scroll indicators at top/bottom edges
    if scroll > 0 {
        if let Some(first) = lines.first_mut() {
            // Add up-arrow hint at the end of first line
            first.spans.push(Span::styled(" ▲", Style::default().fg(theme::HYDRA_DIM).bg(bg)));
        }
    }
    if scroll + count < total {
        if let Some(last) = lines.last_mut() {
            // Add down-arrow hint at the end of last line
            last.spans.push(Span::styled(" ▼", Style::default().fg(theme::HYDRA_DIM).bg(bg)));
        }
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, inner);
}
