//! Input view — borderless prompt with placeholder.
//! Claude Code style: `> _` with no chrome.

use super::RenderState;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Render the input box — borderless, with `>` prompt.
pub fn render(frame: &mut Frame, area: Rect, state: &RenderState) {

    if state.is_searching {
        // Search mode: show search query
        let line = Line::from(vec![
            Span::styled(" search: ", Style::default().fg(Color::Yellow)),
            Span::raw(&state.search_query),
            Span::styled("_", Style::default().add_modifier(Modifier::SLOW_BLINK)),
        ]);
        let p = Paragraph::new(vec![line]);
        frame.render_widget(p, area);
        return;
    }

    let prompt_style = Style::default().fg(Color::Green).add_modifier(Modifier::BOLD);

    if state.input_text.is_empty() {
        // Placeholder
        let line = Line::from(vec![
            Span::styled(" > ", prompt_style),
            Span::styled(
                &state.input_placeholder,
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
            ),
        ]);
        let p = Paragraph::new(vec![line]);
        frame.render_widget(p, area);
    } else {
        // Input with cursor
        let text = &state.input_text;
        let cursor_pos = state.input_cursor.min(text.len());

        let (before, after) = text.split_at(cursor_pos);
        let cursor_char = after.chars().next().unwrap_or(' ');
        let rest = if after.len() > cursor_char.len_utf8() {
            &after[cursor_char.len_utf8()..]
        } else {
            ""
        };

        // Handle multi-line: split by newlines
        let lines_text: Vec<&str> = text.split('\n').collect();
        if lines_text.len() > 1 {
            // Multi-line rendering
            let mut lines = Vec::new();
            for (i, line_text) in lines_text.iter().enumerate() {
                let prefix = if i == 0 { " > " } else { "   " };
                lines.push(Line::from(vec![
                    Span::styled(prefix, prompt_style),
                    Span::raw(*line_text),
                ]));
            }
            let p = Paragraph::new(lines);
            frame.render_widget(p, area);
        } else {
            // Single line with cursor
            let line = Line::from(vec![
                Span::styled(" > ", prompt_style),
                Span::raw(before),
                Span::styled(
                    cursor_char.to_string(),
                    Style::default().add_modifier(Modifier::REVERSED),
                ),
                Span::raw(rest),
            ]);
            let p = Paragraph::new(vec![line]);
            frame.render_widget(p, area);
        }
    }

    // Set cursor position for terminal
    let x = area.x + 3 + state.input_cursor as u16;
    let y = area.y;
    if x < area.x + area.width {
        frame.set_cursor_position((x, y));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_state_defaults() {
        let state = RenderState::default();
        assert!(state.input_text.is_empty());
        assert_eq!(state.input_cursor, 0);
    }
}
