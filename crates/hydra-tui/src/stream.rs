//! ConversationStream — the scrollable stream of items in the cockpit.
//!
//! Multi-line rendering for belief boxes, dream notifications, and tool dots.

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use crate::constants;
use crate::stream_types::{BriefingPriority, CompanionStatus, StreamItem};

/// The conversation stream buffer with scrolling support.
#[derive(Debug, Clone)]
pub struct ConversationStream {
    /// All items in the stream.
    items: Vec<StreamItem>,
    /// Current scroll offset (0 = bottom / most recent).
    scroll_offset: usize,
}

impl ConversationStream {
    /// Create a new empty conversation stream.
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            scroll_offset: 0,
        }
    }

    /// Push a new item to the stream, evicting old items if over capacity.
    pub fn push(&mut self, item: StreamItem) {
        self.items.push(item);
        if self.items.len() > constants::MAX_STREAM_BUFFER {
            let excess = self.items.len() - constants::MAX_STREAM_BUFFER;
            self.items.drain(..excess);
            self.scroll_offset = self.scroll_offset.saturating_sub(excess);
        }
    }

    /// Scroll up by the given number of items.
    pub fn scroll_up(&mut self, amount: usize) {
        let max_offset = self.items.len().saturating_sub(1);
        self.scroll_offset = (self.scroll_offset + amount).min(max_offset);
    }

    /// Scroll down by the given number of items (toward newest).
    pub fn scroll_down(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    /// Scroll to the bottom (newest items).
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }

    /// Return the current scroll offset.
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Return the number of items in the stream.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Return whether the stream is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Return a reference to all items.
    pub fn items(&self) -> &[StreamItem] {
        &self.items
    }

    /// Render visible items as ratatui Lines for a given viewport height.
    /// Items may render as multiple lines (belief boxes, dream notifications).
    pub fn to_lines(&self, viewport_height: usize) -> Vec<Line<'static>> {
        if self.items.is_empty() {
            return vec![Line::from("")];
        }

        let total = self.items.len();
        let end = total.saturating_sub(self.scroll_offset);
        let start = end.saturating_sub(viewport_height);

        let mut lines: Vec<Line<'static>> = self.items[start..end]
            .iter()
            .flat_map(render_item)
            .collect();

        // Trim to viewport
        if lines.len() > viewport_height {
            let skip = lines.len() - viewport_height;
            lines = lines[skip..].to_vec();
        }

        lines
    }

    /// Update the text of the last AssistantText item (for streaming).
    pub fn update_last_text(&mut self, new_text: &str) {
        if let Some(StreamItem::AssistantText { text, .. }) = self.items.last_mut() {
            *text = new_text.to_string();
        }
    }

    /// Clear all items from the stream.
    pub fn clear(&mut self) {
        self.items.clear();
        self.scroll_offset = 0;
    }
}

impl Default for ConversationStream {
    fn default() -> Self {
        Self::new()
    }
}

/// Render a stream item to one or more ratatui Lines.
fn render_item(item: &StreamItem) -> Vec<Line<'static>> {
    let t = crate::theme::current();

    match item {
        StreamItem::UserMessage { text, .. } => vec![Line::from(vec![
            Span::styled("▶ ", Style::default().fg(t.user_message)),
            Span::styled(text.clone(), Style::default().fg(t.user_message)),
        ])],

        StreamItem::AssistantText { text, .. } => {
            crate::render_markdown::render_assistant_text(text, &t)
        }

        StreamItem::ToolDot {
            tool_name, kind, ..
        } => {
            let color = kind.color();
            vec![Line::from(vec![
                Span::styled(kind.symbol().to_string(), Style::default().fg(color)),
                Span::raw(format!(" {tool_name}")),
            ])]
        }

        StreamItem::ToolConnector { label, .. } => vec![Line::from(Span::styled(
            format!("  └ {label}"),
            Style::default().fg(Color::DarkGray),
        ))],

        StreamItem::Truncation {
            chars_truncated, ..
        } => vec![Line::from(Span::styled(
            format!("  ... ({chars_truncated} chars truncated, Ctrl+O to expand)"),
            Style::default().fg(Color::DarkGray),
        ))],

        // Belief citation box — multi-line bordered rendering
        StreamItem::BeliefCitation {
            belief, confidence, ..
        } => render_belief_box(belief, *confidence),

        StreamItem::CompanionTask {
            description,
            status,
            ..
        } => {
            let (r, g, b) = constants::DOT_COLOR_COMPANION;
            let sym = companion_status_symbol(status);
            vec![Line::from(vec![
                Span::styled(format!("{sym} "), Style::default().fg(Color::Rgb(r, g, b))),
                Span::raw(format!("Companion ▸ {description}")),
                Span::styled(
                    format!(" [{}]", status),
                    Style::default().fg(Color::DarkGray),
                ),
            ])]
        }

        StreamItem::BriefingItem {
            content, priority, ..
        } => {
            let color = briefing_priority_color(priority);
            let prefix = match priority {
                BriefingPriority::Urgent => "▲",
                BriefingPriority::High => "●",
                BriefingPriority::Normal => "○",
                BriefingPriority::Low => "·",
            };
            vec![Line::from(vec![
                Span::styled(format!("  {prefix} "), Style::default().fg(color)),
                Span::styled(content.clone(), Style::default().fg(color)),
            ])]
        }

        StreamItem::DreamNotification { content, .. } => {
            render_dream_notification(content)
        }

        StreamItem::SystemNotification { content, .. } => vec![Line::from(Span::styled(
            format!("  ℹ {content}"),
            Style::default().fg(t.system_notification),
        ))],

        StreamItem::Blank => vec![Line::from("")],
    }
}

/// Render a belief citation as a bordered box.
/// Border color: GREEN (>0.85), YELLOW (0.50-0.85), RED (<0.50).
fn render_belief_box(belief: &str, confidence: f64) -> Vec<Line<'static>> {
    let border_color = if confidence > 0.85 {
        Color::Rgb(74, 222, 128) // green
    } else if confidence >= 0.50 {
        Color::Rgb(251, 191, 36) // yellow
    } else {
        Color::Rgb(248, 113, 113) // red
    };
    let conf_pct = (confidence * 100.0) as u8;
    let bs = Style::default().fg(border_color);
    let ts = Style::default().fg(Color::Rgb(180, 180, 180));

    // Measure content width (capped at 60 chars for the belief text)
    let display_belief: String = belief.chars().take(60).collect();
    let header = format!(" Belief ({conf_pct}%) ");
    let inner_width = display_belief.len().max(header.len()) + 2;
    let top_pad = "─".repeat(inner_width.saturating_sub(header.len()));
    let bot_line = "─".repeat(inner_width + 2);

    vec![
        Line::from(vec![
            Span::styled("  ┌─", bs),
            Span::styled(header, bs),
            Span::styled(format!("{top_pad}┐"), bs),
        ]),
        Line::from(vec![
            Span::styled("  │ ", bs),
            Span::styled(format!("\"{display_belief}\""), ts),
            Span::styled(" │", bs),
        ]),
        Line::from(Span::styled(format!("  └{bot_line}┘"), bs)),
    ]
}

/// Render dream notification with numbered discoveries.
fn render_dream_notification(content: &str) -> Vec<Line<'static>> {
    let (r, g, b) = constants::VERB_COLOR_DREAM;
    let dream_color = Color::Rgb(r, g, b);
    let ds = Style::default().fg(dream_color);

    let mut lines = vec![Line::from(Span::styled(
        format!("  ☽ {content}"),
        ds,
    ))];

    // If content has numbered items separated by newlines, render as connectors
    for line in content.lines().skip(1) {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("    └ {trimmed}"),
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    lines
}

/// Return a status symbol for companion task status.
fn companion_status_symbol(status: &CompanionStatus) -> &'static str {
    match status {
        CompanionStatus::Pending => "◇",
        CompanionStatus::Running => "⏵",
        CompanionStatus::Complete => "✓",
        CompanionStatus::Failed => "✗",
        CompanionStatus::Cancelled => "◇",
    }
}

/// Return a ratatui color for a briefing priority.
fn briefing_priority_color(priority: &BriefingPriority) -> Color {
    match priority {
        BriefingPriority::Low => Color::DarkGray,
        BriefingPriority::Normal => Color::Rgb(180, 180, 180),
        BriefingPriority::High => {
            let (r, g, b) = constants::VERB_COLOR_GENERAL;
            Color::Rgb(r, g, b)
        }
        BriefingPriority::Urgent => {
            let (r, g, b) = constants::DOT_COLOR_ERROR;
            Color::Rgb(r, g, b)
        }
    }
}
