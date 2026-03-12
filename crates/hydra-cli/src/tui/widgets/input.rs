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
    // Two-line input area: line 1 = "> input", line 2 = status bar
    let w = area.width as usize;

    // --- Line 1: separator + prompt ---
    let sep_area = Rect::new(area.x, area.y, area.width, 1);
    let sep = Paragraph::new(Line::from(Span::styled(
        "─".repeat(w),
        Style::default().fg(theme::HYDRA_DIM),
    )));
    frame.render_widget(sep, sep_area);

    // Prompt on line 2
    let prompt_area = Rect::new(area.x, area.y + 1, area.width, 1);
    let input_spans = vec![
        Span::styled("> ", theme::prompt()),
        Span::styled(&app.input, theme::user_msg()),
    ];
    let input_line = Line::from(input_spans);
    let para = Paragraph::new(input_line);
    frame.render_widget(para, prompt_area);

    // Cursor
    let cursor_x = area.x + 2 + app.cursor_pos as u16;
    let cursor_y = area.y + 1;
    if cursor_x < area.x + area.width {
        frame.set_cursor_position((cursor_x, cursor_y));
    }

    // --- Line 3: status bar with hints ---
    if area.height >= 3 {
        let status_area = Rect::new(area.x, area.y + 2, area.width, 1);
        let hint_style = Style::default().fg(theme::HYDRA_DIM);
        let accent = Style::default().fg(theme::HYDRA_CYAN);

        // Left side: permission mode + hints
        let mode_label = app.permission_mode.label();
        let mut left: Vec<Span> = Vec::new();
        if !mode_label.is_empty() {
            left.push(Span::styled("▸▸ ", accent));
            left.push(Span::styled(mode_label, accent));
            left.push(Span::styled(" (shift+tab to cycle) · esc to interrupt", hint_style));
        } else if app.is_thinking {
            left.push(Span::styled("esc to interrupt", hint_style));
        } else if app.search_mode {
            left.push(Span::styled("reverse-search: ", accent));
            left.push(Span::styled(&app.search_query, hint_style));
        }

        // Right side: PR status or token usage
        let mut right: Vec<Span> = Vec::new();
        if let Some(ref pr) = app.pr_status {
            let (color, label) = match pr.state {
                PrState::Approved => (theme::HYDRA_GREEN, "approved"),
                PrState::ReviewRequested | PrState::Open => (theme::HYDRA_YELLOW, "review"),
                PrState::ChangesRequested => (theme::HYDRA_RED, "changes"),
                PrState::Merged => (theme::HYDRA_PURPLE, "merged"),
            };
            right.push(Span::styled(
                format!("PR #{} ({})", pr.number, label),
                Style::default().fg(color).add_modifier(Modifier::UNDERLINED),
            ));
        }

        let left_len: usize = left.iter().map(|s| s.content.chars().count()).sum();
        let right_len: usize = right.iter().map(|s| s.content.chars().count()).sum();
        let gap = w.saturating_sub(left_len + right_len + 2);

        let mut bar: Vec<Span> = Vec::new();
        bar.push(Span::raw(" "));
        bar.extend(left);
        bar.push(Span::raw(" ".repeat(gap)));
        bar.extend(right);
        bar.push(Span::raw(" "));

        let status_line = Line::from(bar);
        let status_para = Paragraph::new(status_line);
        frame.render_widget(status_para, status_area);
    }
}
