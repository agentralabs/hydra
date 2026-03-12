use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::tui::app::{App, PrState};
use crate::tui::theme;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    // Clean prompt: "> _" with mode indicator and PR status
    let mode_label = app.permission_mode.label();

    let mut input_spans: Vec<Span> = vec![
        Span::styled("> ", theme::prompt()),
        Span::styled(&app.input, theme::user_msg()),
    ];

    // Build right-side indicators: [mode] [PR status]
    let mut right_parts: Vec<Span> = Vec::new();
    if !mode_label.is_empty() {
        right_parts.push(Span::styled(mode_label, theme::dim()));
    }
    // PR status indicator (spec §11)
    if let Some(ref pr) = app.pr_status {
        let (color, label) = match pr.state {
            PrState::Approved => (theme::HYDRA_GREEN, "approved"),
            PrState::ReviewRequested | PrState::Open => (theme::HYDRA_YELLOW, "review"),
            PrState::ChangesRequested => (theme::HYDRA_RED, "changes"),
            PrState::Merged => (theme::HYDRA_PURPLE, "merged"),
        };
        if !right_parts.is_empty() {
            right_parts.push(Span::raw("  "));
        }
        right_parts.push(Span::styled(
            format!("PR #{} ({})", pr.number, label),
            Style::default().fg(color).add_modifier(Modifier::UNDERLINED),
        ));
    }

    // Right-align all indicators
    if !right_parts.is_empty() {
        let used = 2 + app.input.chars().count();
        let right_len: usize = right_parts.iter().map(|s| s.content.len()).sum();
        let remaining = (area.width as usize).saturating_sub(used + right_len + 1);
        if remaining > 0 {
            input_spans.push(Span::raw(" ".repeat(remaining)));
            input_spans.extend(right_parts);
        }
    }

    let input_line = Line::from(input_spans);
    let para = Paragraph::new(input_line);
    frame.render_widget(para, area);

    // Cursor position: "> " = 2 chars before input
    let cursor_x = area.x + 2 + app.cursor_pos as u16;
    let cursor_y = area.y;
    if cursor_x < area.x + area.width {
        frame.set_cursor_position((cursor_x, cursor_y));
    }
}
