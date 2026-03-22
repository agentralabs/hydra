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
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::HydraTui;

// Spec colors
const BG: Color = Color::Rgb(12, 12, 12);
const AMBER: Color = Color::Rgb(200, 169, 110);
const DIMMER: Color = Color::Rgb(68, 68, 68);
const SEPARATOR: Color = Color::Rgb(34, 34, 34);
const STATUS_BG: Color = Color::Rgb(30, 30, 46);

/// Render the cockpit (conversation mode).
pub fn render(f: &mut Frame, area: Rect, tui: &HydraTui, cursor_visible: bool) {
    // Clear entire area with background color first
    let bg = Block::default().style(Style::default().bg(BG));
    f.render_widget(bg, area);

    let thinking_height = if tui.status.verb_state.is_active() {
        1
    } else {
        0
    };

    // Fixed layout: stream fills, bottom 2-3 rows pinned
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),                     // stream (fills)
            Constraint::Length(thinking_height),     // verb (0 or 1)
            Constraint::Length(1),                   // separator
            Constraint::Length(1),                   // input
            Constraint::Length(1),                   // status bar
        ])
        .split(area);

    render_stream(f, chunks[0], tui);

    if thinking_height > 0 {
        render_thinking_line(f, chunks[1], tui);
    }

    // Separator line between stream and input
    render_separator(f, chunks[2]);

    render_input_line(f, chunks[3], tui, cursor_visible);
    render_status_line(f, chunks[4], tui);
}

fn render_stream(f: &mut Frame, area: Rect, tui: &HydraTui) {
    // Clear the stream area
    f.render_widget(Clear, area);

    let viewport = area.height as usize;
    let lines = tui.stream.to_lines(viewport);

    let para = Paragraph::new(lines)
        .style(Style::default().bg(BG).fg(Color::Rgb(153, 153, 153)))
        .wrap(Wrap { trim: false });
    f.render_widget(para, area);
}

fn render_thinking_line(f: &mut Frame, area: Rect, tui: &HydraTui) {
    f.render_widget(Clear, area);
    let verb_color = tui.status.verb_state.context().color();
    let display = tui.status.verb_state.status_display();

    let line = Line::from(vec![
        Span::styled("  ", Style::default().bg(BG)),
        Span::styled(display, Style::default().fg(verb_color).bg(BG)),
    ]);

    let para = Paragraph::new(vec![line]).style(Style::default().bg(BG));
    f.render_widget(para, area);
}

fn render_separator(f: &mut Frame, area: Rect) {
    f.render_widget(Clear, area);
    let sep = "─".repeat(area.width as usize);
    let line = Line::from(Span::styled(sep, Style::default().fg(SEPARATOR).bg(BG)));
    let para = Paragraph::new(vec![line]).style(Style::default().bg(BG));
    f.render_widget(para, area);
}

fn render_input_line(f: &mut Frame, area: Rect, tui: &HydraTui, cursor_visible: bool) {
    f.render_widget(Clear, area);
    let text = tui.input.text();
    let cursor_char = if cursor_visible { "█" } else { " " };

    let line = if text.is_empty() {
        Line::from(vec![
            Span::styled(" ◈  ", Style::default().fg(AMBER).bg(BG)),
            Span::styled(
                "what are we building today?",
                Style::default().fg(DIMMER).bg(BG),
            ),
            Span::styled(cursor_char, Style::default().fg(AMBER).bg(BG)),
        ])
    } else {
        Line::from(vec![
            Span::styled(" ◈  ", Style::default().fg(AMBER).bg(BG)),
            Span::styled(
                text.to_string(),
                Style::default().fg(Color::Rgb(153, 153, 153)).bg(BG),
            ),
            Span::styled(cursor_char, Style::default().fg(AMBER).bg(BG)),
        ])
    };

    let para = Paragraph::new(vec![line]).style(Style::default().bg(BG));
    f.render_widget(para, area);
}

fn render_status_line(f: &mut Frame, area: Rect, tui: &HydraTui) {
    // Clear the status bar area completely, then fill with status bar bg
    f.render_widget(Clear, area);
    let status_line = tui.status.format();
    // Render with explicit background that fills the entire width
    let para = Paragraph::new(vec![status_line])
        .style(Style::default().bg(STATUS_BG));
    f.render_widget(para, area);
}
