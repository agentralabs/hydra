use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

use super::app::App;
use super::widgets;

/// Main render function — called every frame.
pub fn render(frame: &mut Frame, app: &mut App) {
    let size = frame.area();

    // Header bar shows after first input. Before that, info is in the welcome frame.
    if app.welcome_dismissed {
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),  // Header
                Constraint::Min(8),    // Body
                Constraint::Length(3), // Input bar
            ])
            .split(size);

        widgets::header::render(frame, app, vertical[0]);

        if app.sidebar_visible && vertical[1].width > 50 {
            let sw = if vertical[1].width > 100 { 26 } else { 22 };
            let h = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(30), Constraint::Length(sw)])
                .split(vertical[1]);
            widgets::conversation::render(frame, app, h[0]);
            widgets::sidebar::render(frame, app, h[1]);
        } else {
            widgets::conversation::render(frame, app, vertical[1]);
        }

        widgets::input::render(frame, app, vertical[2]);
        widgets::dropdown::render(frame, app, vertical[2]);
    } else {
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
}
