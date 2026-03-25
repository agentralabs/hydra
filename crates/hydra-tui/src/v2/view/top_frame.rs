//! Top frame — clean adaptive header like Claude Code.
//! Full terminal width. No breaks. Green dots for active systems.
//! Left: ◈ Hydra + model. Right: status indicators.

use super::RenderState;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

const AMBER: Color = Color::Rgb(200, 169, 110);
const GREEN: Color = Color::Rgb(74, 170, 106);
const DIM: Color = Color::Rgb(100, 100, 100);
const BORDER: Color = Color::Rgb(60, 60, 60);

pub fn render(frame: &mut Frame, area: Rect, state: &RenderState) {
    if area.height < 3 { return; } // too small

    // Outer block — full-width adaptive border
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Build the single status line that fills the width
    let mut spans = Vec::new();

    // Left: ◈ Hydra + model
    spans.push(Span::styled(" ◈ ", Style::default().fg(AMBER).add_modifier(Modifier::BOLD)));
    spans.push(Span::styled("Hydra", Style::default().fg(AMBER).add_modifier(Modifier::BOLD)));
    spans.push(Span::styled("  ", Style::default()));

    // Model
    spans.push(Span::styled(state.model.to_string(), Style::default().fg(DIM)));
    spans.push(Span::styled("  ", Style::default()));

    // Active system indicators — green dot = active, dim dot = inactive
    let indicators = [
        ("genome", state.genome_count > 0),
        ("memory", true), // always active
        ("web", true),
        ("vision", state.agent_active),
        ("voice", state.voice_state.is_some()),
    ];

    for (name, active) in &indicators {
        let dot_color = if *active { GREEN } else { DIM };
        spans.push(Span::styled("● ", Style::default().fg(dot_color)));
        spans.push(Span::styled(format!("{name} "), Style::default().fg(if *active { GREEN } else { DIM })));
    }

    // Right: session info
    let right = format!(
        " genome:{} session:{}m ",
        state.genome_count, state.session_minutes,
    );
    // Pad to fill width
    let used: usize = spans.iter().map(|s| s.content.len()).sum::<usize>();
    let total = inner.width as usize;
    let pad = total.saturating_sub(used + right.len());
    spans.push(Span::styled(" ".repeat(pad), Style::default()));
    spans.push(Span::styled(right, Style::default().fg(DIM)));

    let line = Line::from(spans);
    frame.render_widget(Paragraph::new(vec![line]), inner);
}

// Items for stream greeting (called once at boot)
pub fn greeting_items(user: &str, genome_count: usize, session_num: usize) -> Vec<crate::stream_types::StreamItem> {
    use crate::stream_types::StreamItem;
    use chrono::Utc;
    use uuid::Uuid;

    let hour = chrono::Local::now().hour();
    let greeting = if hour < 12 { "Good morning" }
        else if hour < 17 { "Good afternoon" }
        else { "Good evening" };

    vec![
        StreamItem::SystemNotification {
            id: Uuid::new_v4(),
            content: format!("{greeting}, {user}! Session #{session_num}. Genome: {genome_count} entries."),
            timestamp: Utc::now(),
        },
    ]
}

use chrono::Timelike;
