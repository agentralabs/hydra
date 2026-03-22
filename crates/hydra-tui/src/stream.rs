//! ConversationStream — the scrollable stream of items in the cockpit.
//!
//! Rendering logic for each `StreamItem` variant.

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
    pub fn to_lines(&self, viewport_height: usize) -> Vec<Line<'static>> {
        if self.items.is_empty() {
            return vec![Line::from("")];
        }

        let total = self.items.len();
        let end = total.saturating_sub(self.scroll_offset);
        let start = end.saturating_sub(viewport_height);

        self.items[start..end]
            .iter()
            .map(|item| render_item(item))
            .collect()
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

/// Render a single stream item to a ratatui Line.
fn render_item(item: &StreamItem) -> Line<'static> {
    match item {
        StreamItem::UserMessage { text, .. } => {
            let (r, g, b) = constants::USER_MESSAGE_COLOR;
            Line::from(vec![
                Span::styled("▶ ", Style::default().fg(Color::Rgb(r, g, b))),
                Span::styled(text.clone(), Style::default().fg(Color::Rgb(r, g, b))),
            ])
        }

        StreamItem::AssistantText { text, .. } => {
            let (r, g, b) = constants::ASSISTANT_TEXT_COLOR;
            Line::from(Span::styled(
                text.clone(),
                Style::default().fg(Color::Rgb(r, g, b)),
            ))
        }

        StreamItem::ToolDot {
            tool_name, kind, ..
        } => {
            let color = kind.color();
            Line::from(vec![
                Span::styled(kind.symbol().to_string(), Style::default().fg(color)),
                Span::raw(format!(" {tool_name}")),
            ])
        }

        StreamItem::ToolConnector { label, .. } => Line::from(Span::styled(
            format!("  │ {label}"),
            Style::default().fg(Color::DarkGray),
        )),

        StreamItem::Truncation {
            chars_truncated, ..
        } => Line::from(Span::styled(
            format!("  ... ({chars_truncated} chars truncated)"),
            Style::default().fg(Color::DarkGray),
        )),

        StreamItem::BeliefCitation {
            belief, confidence, ..
        } => {
            let conf_pct = (confidence * 100.0) as u8;
            Line::from(Span::styled(
                format!("  ⟨belief: {belief} ({conf_pct}%)⟩"),
                Style::default().fg(Color::Rgb(167, 139, 250)),
            ))
        }

        StreamItem::CompanionTask {
            description,
            status,
            ..
        } => {
            let (r, g, b) = constants::DOT_COLOR_COMPANION;
            let status_str = companion_status_symbol(status);
            Line::from(vec![
                Span::styled(
                    format!("{status_str} "),
                    Style::default().fg(Color::Rgb(r, g, b)),
                ),
                Span::raw(description.clone()),
                Span::styled(
                    format!(" [{}]", status),
                    Style::default().fg(Color::DarkGray),
                ),
            ])
        }

        StreamItem::BriefingItem {
            content, priority, ..
        } => {
            let color = briefing_priority_color(priority);
            let prefix = match priority {
                BriefingPriority::Urgent => "⚡",
                BriefingPriority::High => "▲",
                BriefingPriority::Normal => "•",
                BriefingPriority::Low => "·",
            };
            Line::from(vec![
                Span::styled(format!("{prefix} "), Style::default().fg(color)),
                Span::styled(content.clone(), Style::default().fg(color)),
            ])
        }

        StreamItem::DreamNotification { content, .. } => {
            let (r, g, b) = constants::VERB_COLOR_DREAM;
            Line::from(Span::styled(
                format!("  ☽ {content}"),
                Style::default().fg(Color::Rgb(r, g, b)),
            ))
        }

        StreamItem::SystemNotification { content, .. } => {
            let (r, g, b) = constants::SYSTEM_NOTIFICATION_COLOR;
            Line::from(Span::styled(
                format!("  ℹ {content}"),
                Style::default().fg(Color::Rgb(r, g, b)),
            ))
        }

        StreamItem::Blank => Line::from(""),
    }
}

/// Return a status symbol for companion task status.
fn companion_status_symbol(status: &CompanionStatus) -> &'static str {
    match status {
        CompanionStatus::Pending => "◇",
        CompanionStatus::Running => "◈",
        CompanionStatus::Complete => "◆",
        CompanionStatus::Failed => "◇",
        CompanionStatus::Cancelled => "◇",
    }
}

/// Return a ratatui color for a briefing priority.
fn briefing_priority_color(priority: &BriefingPriority) -> Color {
    match priority {
        BriefingPriority::Low => Color::DarkGray,
        BriefingPriority::Normal => {
            let (r, g, b) = constants::ASSISTANT_TEXT_COLOR;
            Color::Rgb(r, g, b)
        }
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
