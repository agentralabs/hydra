use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

use super::app::App;
use super::widgets;

/// Main render function — called every frame.
/// Layout: conversation body + input bar. Welcome frame is part of conversation
/// and always scrollable at the top.
pub fn render(frame: &mut Frame, app: &mut App) {
    let size = frame.area();

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),    // Body (welcome frame + messages)
            Constraint::Length(3), // Input bar
        ])
        .split(size);

    widgets::conversation::render(frame, app, vertical[0]);
    widgets::input::render(frame, app, vertical[1]);
    widgets::dropdown::render(frame, app, vertical[1]);
}
