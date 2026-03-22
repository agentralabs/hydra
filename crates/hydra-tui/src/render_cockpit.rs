//! Cockpit renderer — the conversation view.
//!
//! Layout (pinned regions):
//! ┌─────────────────────────────────────────┐
//! │  CONVERSATION STREAM (fills height)     │  ← scrollable, independent
//! ├─────────────────────────────────────────┤
//! │  ◑ Cogitating        (thinking verb)    │  ← 1 row, hidden when idle
//! ├─────────────────────────────────────────┤
//! │  ◈  [user input text]█                  │  ← 1 row, always pinned
//! ├─────────────────────────────────────────┤
//! │  ◈ Hydra  session:0m  V=1.00  tokens:0 │  ← 1 row, always pinned
//! └─────────────────────────────────────────┘

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap};
use ratatui::Frame;

use crate::alert::AlertLevel;
use crate::app::HydraTui;
use crate::theme;

/// Render the cockpit (conversation mode).
pub fn render(f: &mut Frame, area: Rect, tui: &HydraTui, cursor_visible: bool) {
    let t = theme::current();

    let bg = Block::default().style(Style::default().bg(t.bg_primary));
    f.render_widget(bg, area);

    let thinking_height = if tui.status.verb_state.is_active() {
        1
    } else {
        0
    };

    let alert_height: u16 = if tui.alerts.current().is_some() { 1 } else { 0 };

    // Input height grows with content (1-3 lines)
    let input_text_len = tui.input.text().len();
    let term_width = area.width.max(1) as usize;
    let input_lines = ((input_text_len / term_width) + 1).min(3) as u16;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),                     // stream (fills)
            Constraint::Length(thinking_height),     // verb (0 or 1)
            Constraint::Length(alert_height),        // alert bar (0 or 1)
            Constraint::Length(1),                   // separator
            Constraint::Length(input_lines),         // input (1-3 lines)
            Constraint::Length(1),                   // status bar
        ])
        .split(area);

    render_stream(f, chunks[0], tui, &t);

    if thinking_height > 0 {
        render_thinking_line(f, chunks[1], tui, &t);
    }

    if alert_height > 0 {
        render_alert_bar(f, chunks[2], tui, &t);
    }

    render_separator(f, chunks[3], &t);
    render_input_line(f, chunks[4], tui, cursor_visible, &t);
    render_status_line(f, chunks[5], tui, &t);
}

fn render_stream(f: &mut Frame, area: Rect, tui: &HydraTui, t: &theme::Theme) {
    f.render_widget(Clear, area);

    let viewport = area.height as usize;
    let mut lines = tui.stream.to_lines(viewport);

    // "▼ more below" indicator when scrolled up
    if tui.stream.scroll_offset() > 0 && !lines.is_empty() {
        let indicator = Line::from(Span::styled(
            format!(
                "  ▼ {} newer messages below — scroll down or press End",
                tui.stream.scroll_offset()
            ),
            Style::default().fg(t.warning),
        ));
        // Replace last line with indicator
        if let Some(last) = lines.last_mut() {
            *last = indicator;
        }
    }

    let para = Paragraph::new(lines)
        .style(Style::default().bg(t.bg_primary).fg(t.fg_primary))
        .wrap(Wrap { trim: false });
    f.render_widget(para, area);

    // Scrollbar on right edge when there's content to scroll
    let total = tui.stream.len();
    if total > viewport {
        let position = total.saturating_sub(tui.stream.scroll_offset()).saturating_sub(viewport);
        let mut scrollbar_state = ScrollbarState::new(total).position(position);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .style(Style::default().fg(t.fg_muted));
        f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}

fn render_thinking_line(f: &mut Frame, area: Rect, tui: &HydraTui, t: &theme::Theme) {
    f.render_widget(Clear, area);
    let verb_color = tui.status.verb_state.context().color();
    let display = tui.status.verb_state.status_display();

    let line = Line::from(vec![
        Span::styled("  ", Style::default().bg(t.bg_primary)),
        Span::styled(display, Style::default().fg(verb_color).bg(t.bg_primary)),
    ]);

    let para = Paragraph::new(vec![line]).style(Style::default().bg(t.bg_primary));
    f.render_widget(para, area);
}

fn render_alert_bar(f: &mut Frame, area: Rect, tui: &HydraTui, t: &theme::Theme) {
    f.render_widget(Clear, area);
    if let Some(alert) = tui.alerts.current() {
        let (prefix, color) = match alert.level {
            AlertLevel::Emergency => ("▲ ", t.error),
            AlertLevel::Frame => ("⚠ ", t.warning),
            AlertLevel::Stream => ("ℹ ", t.system_notification),
        };
        let line = Line::from(vec![
            Span::styled(prefix, Style::default().fg(color).bg(t.bg_primary)),
            Span::styled(
                alert.message.clone(),
                Style::default().fg(color).bg(t.bg_primary),
            ),
        ]);
        let para = Paragraph::new(vec![line]).style(Style::default().bg(t.bg_primary));
        f.render_widget(para, area);
    }
}

fn render_separator(f: &mut Frame, area: Rect, t: &theme::Theme) {
    f.render_widget(Clear, area);
    let sep = "─".repeat(area.width as usize);
    let line = Line::from(Span::styled(
        sep,
        Style::default().fg(t.separator).bg(t.bg_primary),
    ));
    let para = Paragraph::new(vec![line]).style(Style::default().bg(t.bg_primary));
    f.render_widget(para, area);
}

fn render_input_line(
    f: &mut Frame,
    area: Rect,
    tui: &HydraTui,
    cursor_visible: bool,
    t: &theme::Theme,
) {
    f.render_widget(Clear, area);
    let cursor_char = if cursor_visible { "█" } else { " " };

    // Search mode — show search prompt instead of normal input
    if tui.input.is_searching() {
        let prompt = tui.input.search_prompt();
        let line = Line::from(vec![
            Span::styled(" ", Style::default().bg(t.bg_primary)),
            Span::styled(prompt, Style::default().fg(t.warning).bg(t.bg_primary)),
            Span::styled(cursor_char, Style::default().fg(t.warning).bg(t.bg_primary)),
        ]);
        let para = Paragraph::new(vec![line])
            .style(Style::default().bg(t.bg_primary))
            .wrap(Wrap { trim: false });
        f.render_widget(para, area);
        return;
    }

    let text = tui.input.text();
    let line = if text.is_empty() {
        Line::from(vec![
            Span::styled(" ◈  ", Style::default().fg(t.accent).bg(t.bg_primary)),
            Span::styled(
                "what are we building today?",
                Style::default().fg(t.fg_muted).bg(t.bg_primary),
            ),
            Span::styled(cursor_char, Style::default().fg(t.accent).bg(t.bg_primary)),
        ])
    } else {
        Line::from(vec![
            Span::styled(" ◈  ", Style::default().fg(t.accent).bg(t.bg_primary)),
            Span::styled(
                text.to_string(),
                Style::default().fg(t.fg_primary).bg(t.bg_primary),
            ),
            Span::styled(cursor_char, Style::default().fg(t.accent).bg(t.bg_primary)),
        ])
    };

    let para = Paragraph::new(vec![line])
        .style(Style::default().bg(t.bg_primary))
        .wrap(Wrap { trim: false });
    f.render_widget(para, area);
}

fn render_status_line(f: &mut Frame, area: Rect, tui: &HydraTui, t: &theme::Theme) {
    f.render_widget(Clear, area);
    let status_line = tui.status.format();
    let para = Paragraph::new(vec![status_line]).style(Style::default().bg(t.status_bar_bg));
    f.render_widget(para, area);
}
