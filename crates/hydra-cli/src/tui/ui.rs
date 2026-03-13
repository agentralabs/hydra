use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

use super::app::App;
use super::widgets;

/// Main render function — called every frame.
/// Layout: persistent header (1 line) + conversation body + input bar (3 lines).
pub fn render(frame: &mut Frame, app: &mut App) {
    let size = frame.area();

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Persistent header bar (always visible)
            Constraint::Min(8),    // Body (messages)
            Constraint::Length(3), // Input bar
        ])
        .split(size);

    widgets::header::render(frame, app, vertical[0]);
    widgets::conversation::render(frame, app, vertical[1]);
    widgets::input::render(frame, app, vertical[2]);
    widgets::dropdown::render(frame, app, vertical[2]);
}
