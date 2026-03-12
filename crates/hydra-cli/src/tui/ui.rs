use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

use super::app::App;
use super::widgets;

/// Main render function — called every frame.
///
/// Layout (clean, full-width — matches Claude Code):
/// ```text
/// ── Hydra v1.1.0 · project · model · tools ● online ──────────
///
///   > user input text here
///
///   Hydra response text flows full width...
///
///   ● Read src/auth.rs (287 lines)
///   ● Bash(cargo check) (exit 0)
///     └  Finished `dev` profile
///
/// > _
/// ```
pub fn render(frame: &mut Frame, app: &mut App) {
    let size = frame.area();

    // Vertical: Header | Body | Input
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),  // Header
            Constraint::Min(8),    // Body (conversation — full width)
            Constraint::Length(2), // Input bar
        ])
        .split(size);

    // Render header
    widgets::header::render(frame, app, vertical[0]);

    // Full-width conversation — no sidebar by default (status via /status command)
    if app.sidebar_visible && vertical[1].width > 50 {
        let sidebar_width = if vertical[1].width > 100 { 26 } else { 22 };
        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(30),                       // Conversation
                Constraint::Length(sidebar_width),         // Sidebar
            ])
            .split(vertical[1]);

        widgets::conversation::render(frame, app, horizontal[0]);
        widgets::sidebar::render(frame, app, horizontal[1]);
    } else {
        // Full width conversation (default)
        widgets::conversation::render(frame, app, vertical[1]);
    }

    // Render input bar
    widgets::input::render(frame, app, vertical[2]);

    // Render command dropdown overlay (above input bar)
    widgets::dropdown::render(frame, app, vertical[2]);
}
