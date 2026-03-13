use ratatui::{
    layout::Rect,
    text::Line,
    widgets::Paragraph,
    Frame,
};

use crate::tui::app::App;
use super::conversation::empty::build_welcome_frame;

/// Fixed header — always visible at top of screen, never scrolls.
/// Renders the full welcome frame (version, logo, metrics, tips).
pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();
    build_welcome_frame(app, area.width as usize, &mut lines);
    let para = Paragraph::new(lines);
    frame.render_widget(para, area);
}
