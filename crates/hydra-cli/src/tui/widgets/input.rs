use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::tui::app::{App, InputMode};
use crate::tui::theme;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let mode_indicator = match app.input_mode {
        InputMode::Insert => Span::styled(" INSERT ", Style::default()
            .fg(theme::HYDRA_BG)
            .bg(theme::HYDRA_BLUE)
            .add_modifier(Modifier::BOLD)),
        InputMode::Normal => Span::styled(" NORMAL ", Style::default()
            .fg(theme::HYDRA_BG)
            .bg(theme::HYDRA_DIM)
            .add_modifier(Modifier::BOLD)),
    };

    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(theme::border());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Build input line
    let prompt_char = if app.input_mode == InputMode::Insert {
        "❯ "
    } else {
        "  "
    };

    // Input with cursor
    let input_line = Line::from(vec![
        Span::raw(" "),
        mode_indicator,
        Span::raw(" "),
        Span::styled(prompt_char, theme::prompt()),
        Span::styled(&app.input, theme::user_msg()),
    ]);

    let para = Paragraph::new(input_line);
    frame.render_widget(para, inner);

    // Set cursor position when in insert mode
    if app.input_mode == InputMode::Insert {
        // " INSERT  ❯ " = 13 chars before input
        let cursor_x = inner.x + 13 + app.cursor_pos as u16;
        let cursor_y = inner.y;
        if cursor_x < inner.x + inner.width {
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }
}
