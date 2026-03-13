use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

use super::app::App;
use super::widgets;

/// Main render function — called every frame.
/// Layout: fixed header (welcome frame) + scrollable conversation + input bar.
/// The header never scrolls away — always pinned to top like Claude Code.
pub fn render(frame: &mut Frame, app: &mut App) {
    let size = frame.area();

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(17), // Fixed header (welcome frame — never scrolls)
            Constraint::Min(5),    // Scrollable conversation body
            Constraint::Length(3), // Input bar
        ])
        .split(size);

    widgets::header::render(frame, app, vertical[0]);
    widgets::conversation::render(frame, app, vertical[1]);
    widgets::input::render(frame, app, vertical[2]);
    widgets::dropdown::render(frame, app, vertical[2]);
}
