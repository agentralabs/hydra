//! Modal view — renders command palette, config editor, etc. as overlays.
//! Centered on screen with semi-transparent background.

use super::RenderState;
use crate::v2::modal::Modal;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};
use ratatui::Frame;

/// Render a modal overlay.
pub fn render(frame: &mut Frame, area: Rect, state: &RenderState) {
    let modal = match &state.modal {
        Some(m) => m,
        None => return,
    };

    match modal {
        Modal::CommandPalette { query, filtered, selected } => {
            // Claude Code style: inline dropdown above input, not centered overlay
            let max_visible = 12.min(area.height.saturating_sub(6) as usize);
            let palette_height = (max_visible + 3) as u16; // items + search + separator
            let palette_area = Rect {
                x: area.x + 2,
                y: area.y + area.height - palette_height - 3, // above input + status
                width: area.width.saturating_sub(4),
                height: palette_height,
            };
            frame.render_widget(Clear, palette_area);
            render_palette(frame, palette_area, query, filtered, selected, state);
        }
        Modal::ConfigEditor { entries, selected, editing, draft, error } => {
            let centered = centered_rect(60, 70, area);
            frame.render_widget(Clear, centered);
            render_config_editor(frame, centered, entries, *selected, *editing, draft, error.as_deref());
        }
        Modal::SessionList { sessions, selected } => {
            let centered = centered_rect(60, 70, area);
            frame.render_widget(Clear, centered);
            render_session_list(frame, centered, sessions, *selected);
        }
        Modal::Confirm { message, selected_yes } => {
            render_confirm(frame, area, message, *selected_yes);
        }
        _ => {}
    }
}

/// Claude Code style: simple list, no big border, anchored to input.
fn render_palette(
    frame: &mut Frame,
    area: Rect,
    query: &str,
    _filtered: &[usize],
    selected: &usize,
    _state: &RenderState,
) {
    // Simple dark background, thin border
    let bg = Style::default().bg(Color::Rgb(25, 25, 35));
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(60, 60, 80)))
        .style(bg);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Build filtered command list
    let registry = crate::v2::commands::build_registry();
    let results = registry.fuzzy_search(query);
    let max_items = inner.height as usize;

    let items: Vec<ListItem> = results
        .iter()
        .take(max_items)
        .enumerate()
        .map(|(i, (cmd_idx, _score))| {
            let cmd = &registry.all()[*cmd_idx];
            let is_sel = i == *selected;
            let style = if is_sel {
                Style::default().fg(Color::White).bg(Color::Rgb(50, 50, 80))
            } else {
                Style::default().fg(Color::Gray)
            };
            let line = Line::from(vec![
                Span::styled(
                    format!(" /{:<14}", cmd.name),
                    style.add_modifier(if is_sel { Modifier::BOLD } else { Modifier::empty() }),
                ),
                Span::styled(
                    cmd.description,
                    if is_sel { Style::default().fg(Color::Rgb(150, 150, 170)).bg(Color::Rgb(50, 50, 80)) }
                    else { Style::default().fg(Color::DarkGray) },
                ),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, inner);
}

fn render_config_editor(
    frame: &mut Frame,
    area: Rect,
    entries: &[crate::v2::modal::ConfigEntry],
    selected: usize,
    editing: bool,
    draft: &str,
    error: Option<&str>,
) {
    let block = Block::default()
        .title(" Settings ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .style(Style::default().bg(Color::Rgb(20, 20, 30)));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let items: Vec<ListItem> = entries
        .iter()
        .enumerate()
        .take(inner.height as usize)
        .map(|(i, entry)| {
            let is_selected = i == selected;
            let bg = if is_selected {
                Color::Rgb(50, 50, 80)
            } else {
                Color::Rgb(20, 20, 30)
            };

            let value_text = if is_selected && editing {
                format!("{draft}_")
            } else {
                entry.current_value.clone()
            };

            let line = Line::from(vec![
                Span::styled(
                    format!(" {:<24}", format!("{}.{}", entry.section, entry.key)),
                    Style::default().fg(Color::White).bg(bg),
                ),
                Span::styled(
                    format!(" {:<16}", value_text),
                    Style::default().fg(Color::Cyan).bg(bg),
                ),
                Span::styled(
                    &entry.description,
                    Style::default().fg(Color::DarkGray).bg(bg),
                ),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, inner);

    // Error message at bottom
    if let Some(err) = error {
        let err_area = Rect {
            y: inner.y + inner.height.saturating_sub(1),
            height: 1,
            ..inner
        };
        let err_p = Paragraph::new(format!(" Error: {err}"))
            .style(Style::default().fg(Color::Red));
        frame.render_widget(err_p, err_area);
    }
}

fn render_session_list(
    frame: &mut Frame,
    area: Rect,
    sessions: &[crate::v2::modal::SessionEntry],
    selected: usize,
) {
    let block = Block::default()
        .title(" Sessions ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta))
        .style(Style::default().bg(Color::Rgb(20, 20, 30)));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let items: Vec<ListItem> = sessions
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let style = if i == selected {
                Style::default().fg(Color::White).bg(Color::Rgb(50, 50, 80))
            } else {
                Style::default().fg(Color::Gray)
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!(" {:<12}", s.date), style),
                Span::styled(format!(" {} exchanges ", s.exchange_count), style),
                Span::styled(&s.preview, Style::default().fg(Color::DarkGray)),
            ]))
        })
        .collect();

    frame.render_widget(List::new(items), inner);
}

fn render_confirm(
    frame: &mut Frame,
    area: Rect,
    message: &str,
    selected_yes: bool,
) {
    let small = centered_rect(40, 20, area);
    let block = Block::default()
        .title(" Confirm ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .style(Style::default().bg(Color::Rgb(20, 20, 30)));

    let inner = block.inner(small);
    frame.render_widget(Clear, small);
    frame.render_widget(block, small);

    let lines = vec![
        Line::from(format!(" {message}")),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  [Yes]  ",
                if selected_yes {
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                },
            ),
            Span::styled(
                "  [No]  ",
                if !selected_yes {
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                },
            ),
        ]),
    ];
    frame.render_widget(Paragraph::new(lines), inner);
}

/// Calculate a centered rectangle within the given area.
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centered_rect_within_bounds() {
        let area = Rect::new(0, 0, 100, 50);
        let centered = centered_rect(60, 70, area);
        assert!(centered.x >= area.x);
        assert!(centered.y >= area.y);
        assert!(centered.x + centered.width <= area.x + area.width);
        assert!(centered.y + centered.height <= area.y + area.height);
    }
}
