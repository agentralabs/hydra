//! ConversationStream — the scrollable stream of items in the cockpit.
//!
//! Multi-line rendering for belief boxes, dream notifications, and tool dots.
//! Go reference template style: ❯ You, ⏵ Tool ▸ action, ○ Thought.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use std::sync::Arc;

use crate::constants;
use crate::stream_types::{BriefingPriority, CompanionStatus, StreamItem};
use crate::theme;

/// The conversation stream buffer with scrolling support.
/// Items stored in Arc for zero-copy snapshot to render state (Fix 1).
#[derive(Debug, Clone)]
pub struct ConversationStream {
    items: Arc<Vec<StreamItem>>,
    scroll_offset: usize,
    /// Auto-scroll: when true, new items keep the view at the bottom.
    auto_scroll: bool,
    /// Generation counter — increments on every mutation.
    pub generation: u64,
    /// Items added while user is scrolled up (for "↓ N new" badge).
    new_while_scrolled: usize,
}

impl ConversationStream {
    pub fn new() -> Self {
        Self { items: Arc::new(Vec::new()), scroll_offset: 0, auto_scroll: true, generation: 0, new_while_scrolled: 0 }
    }

    pub fn push(&mut self, item: StreamItem) {
        Arc::make_mut(&mut self.items).push(item);
        self.generation = self.generation.wrapping_add(1);
        if !self.auto_scroll { self.new_while_scrolled += 1; }
        // Chunked eviction: drain 500 at once instead of 1-at-a-time (Fix 10)
        if self.items.len() > constants::MAX_STREAM_BUFFER {
            let drain_count = constants::MAX_STREAM_BUFFER / 10;
            Arc::make_mut(&mut self.items).drain(..drain_count);
            self.scroll_offset = self.scroll_offset.saturating_sub(drain_count);
        }
    }

    pub fn scroll_up(&mut self, amount: usize) {
        let max_offset = self.items.len().saturating_sub(1);
        self.scroll_offset = (self.scroll_offset + amount).min(max_offset);
        self.auto_scroll = false;
        self.generation = self.generation.wrapping_add(1);
    }

    pub fn scroll_down(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
        if self.scroll_offset == 0 { self.auto_scroll = true; }
        self.generation = self.generation.wrapping_add(1);
    }

    pub fn scroll_to_bottom(&mut self) { self.scroll_offset = 0; self.auto_scroll = true; self.new_while_scrolled = 0; }
    pub fn scroll_offset(&self) -> usize { self.scroll_offset }
    pub fn len(&self) -> usize { self.items.len() }
    pub fn is_empty(&self) -> bool { self.items.is_empty() }
    pub fn items(&self) -> &[StreamItem] { &self.items }
    /// Zero-copy Arc clone for render state snapshot (Fix 1).
    pub fn items_shared(&self) -> Arc<Vec<StreamItem>> { Arc::clone(&self.items) }
    pub fn is_auto_scroll(&self) -> bool { self.auto_scroll }
    pub fn new_while_scrolled(&self) -> usize { self.new_while_scrolled }

    pub fn to_lines(&self, viewport_height: usize) -> Vec<Line<'static>> {
        if self.items.is_empty() { return vec![Line::from("")]; }
        let total = self.items.len();
        let end = total.saturating_sub(self.scroll_offset);
        let start = end.saturating_sub(viewport_height);

        let mut lines: Vec<Line<'static>> = self.items[start..end]
            .iter()
            .flat_map(render_item)
            .collect();

        if lines.len() > viewport_height {
            let skip = lines.len() - viewport_height;
            lines = lines[skip..].to_vec();
        }
        lines
    }

    pub fn update_last_text(&mut self, new_text: &str) {
        if let Some(StreamItem::AssistantText { text, .. }) = Arc::make_mut(&mut self.items).last_mut() {
            *text = new_text.to_string();
            self.generation = self.generation.wrapping_add(1);
        }
    }

    pub fn clear(&mut self) {
        Arc::make_mut(&mut self.items).clear();
        self.scroll_offset = 0;
        self.auto_scroll = true;
        self.generation = self.generation.wrapping_add(1);
    }
}

impl Default for ConversationStream {
    fn default() -> Self { Self::new() }
}

/// Render a stream item to one or more ratatui Lines.
fn render_item(item: &StreamItem) -> Vec<Line<'static>> {
    let t = theme::current();
    let (br, bg, bb) = constants::HYDRA_BLUE;
    let blue = Color::Rgb(br, bg, bb);

    match item {
        // ❯ You — user message in HYDRA_BLUE bold
        StreamItem::UserMessage { text, .. } => {
            let mut lines = vec![Line::from("")]; // blank before user msg
            lines.push(Line::from(vec![
                Span::styled("  ❯ ", Style::default().fg(blue).add_modifier(Modifier::BOLD)),
                Span::styled("You", Style::default().fg(blue).add_modifier(Modifier::BOLD)),
            ]));
            for line in text.lines() {
                lines.push(Line::from(Span::styled(
                    format!("  {line}"),
                    Style::default().fg(t.fg_primary),
                )));
            }
            lines
        }

        StreamItem::AssistantText { text, .. } => {
            let mut lines = vec![Line::from("")];
            lines.extend(crate::render_markdown::render_assistant_text(text, &t));
            lines
        }

        // ⏵ Tool ▸ action — Go style tool dot
        StreamItem::ToolDot { tool_name, kind, .. } => {
            let color = kind.color();
            vec![Line::from(vec![
                Span::styled("  ⏵ ", Style::default().fg(blue)),
                Span::styled(
                    kind_label(kind),
                    Style::default().fg(blue),
                ),
                Span::styled(" ▸ ", Style::default().fg(t.dim)),
                Span::styled(tool_name.clone(), Style::default().fg(color)),
            ])]
        }

        // └ connector in DIM
        StreamItem::ToolConnector { label, .. } => vec![Line::from(Span::styled(
            format!("    └ {label}"),
            Style::default().fg(t.dim),
        ))],

        StreamItem::Truncation { chars_truncated, .. } => vec![Line::from(Span::styled(
            format!("  ... ({chars_truncated} chars truncated, Ctrl+O to expand)"),
            Style::default().fg(t.dim),
        ))],

        StreamItem::BeliefCitation { belief, confidence, .. } => {
            render_belief_box(belief, *confidence)
        }

        StreamItem::CompanionTask { description, status, .. } => {
            let (or, og, ob) = constants::HYDRA_ORANGE;
            let sym = companion_status_symbol(status);
            vec![Line::from(vec![
                Span::styled(format!("  {sym} "), Style::default().fg(Color::Rgb(or, og, ob))),
                Span::raw(format!("Companion ▸ {description}")),
                Span::styled(format!(" [{}]", status), Style::default().fg(t.dim)),
            ])]
        }

        // Briefing items with Go-style priority indicators
        StreamItem::BriefingItem { content, priority, .. } => {
            let (prefix, color) = match priority {
                BriefingPriority::Urgent => ("▲", Color::Rgb(constants::HYDRA_RED.0, constants::HYDRA_RED.1, constants::HYDRA_RED.2)),
                BriefingPriority::High => ("●", Color::Rgb(constants::HYDRA_YELLOW.0, constants::HYDRA_YELLOW.1, constants::HYDRA_YELLOW.2)),
                BriefingPriority::Normal => ("○", t.fg_primary),
                BriefingPriority::Low => ("○", Color::Rgb(constants::HYDRA_DIM.0, constants::HYDRA_DIM.1, constants::HYDRA_DIM.2)),
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

        // ○ Thought for Xs — in DIM
        StreamItem::ThinkingPill { duration_secs } => {
            let label = if *duration_secs < 1.0 {
                format!("Thought for {:.0}ms", duration_secs * 1000.0)
            } else {
                format!("Thought for {:.1}s", duration_secs)
            };
            vec![Line::from(Span::styled(
                format!("  ○ {label}"),
                Style::default().fg(t.dim),
            ))]
        }

        StreamItem::AgentStep { step_number, action, is_complete, .. } => {
            let status = if *is_complete { "done" } else { "ok" };
            let color = if *is_complete { Color::Green } else { Color::Cyan };
            vec![Line::from(vec![
                Span::raw("  "),
                Span::styled(format!("Step {step_number}"), Style::default().fg(color).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" {action}"), Style::default().fg(t.fg_primary)),
                Span::styled(format!(" [{status}]"), Style::default().fg(Color::DarkGray)),
            ])]
        }
        StreamItem::AlertFrame { title, lines: alert_lines, .. } => {
            let mut out = vec![Line::from("")];
            out.push(Line::from(Span::styled(format!("  ┌─ ▲ {title} ─────────────────────────┐"), Style::default().fg(Color::Red))));
            for al in alert_lines {
                out.push(Line::from(Span::styled(format!("  │ ▲ {al}"), Style::default().fg(Color::Red))));
            }
            out.push(Line::from(Span::styled("  └──────────────────────────────────────┘", Style::default().fg(Color::Red))));
            out
        }
        StreamItem::Blank => vec![Line::from("")],
    }
}

/// Map DotKind to a human-readable subsystem label.
fn kind_label(kind: &crate::dot::DotKind) -> String {
    use crate::dot::DotKind;
    match kind {
        DotKind::Read => "Codebase".into(),
        DotKind::Cognitive => "Memory".into(),
        DotKind::Companion => "Companion".into(),
        DotKind::Active => "Working".into(),
        DotKind::Success => "Complete".into(),
        DotKind::Error => "Error".into(),
        DotKind::Narration => "System".into(),
    }
}

/// Render a belief citation as a bordered box with confidence color.
fn render_belief_box(belief: &str, confidence: f64) -> Vec<Line<'static>> {
    let border_color = theme::Theme::confidence_color(confidence);
    let conf_pct = (confidence * 100.0) as u8;
    let bs = Style::default().fg(border_color);
    let ts = Style::default().fg(Color::Rgb(180, 180, 180));

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

fn render_dream_notification(content: &str) -> Vec<Line<'static>> {
    let (r, g, b) = constants::VERB_COLOR_DREAM;
    let ds = Style::default().fg(Color::Rgb(r, g, b));

    let mut lines = vec![Line::from(Span::styled(format!("  ☽ {content}"), ds))];
    for line in content.lines().skip(1) {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            let t = theme::current();
            lines.push(Line::from(Span::styled(
                format!("    └ {trimmed}"),
                Style::default().fg(t.dim),
            )));
        }
    }
    lines
}

fn companion_status_symbol(status: &CompanionStatus) -> &'static str {
    match status {
        CompanionStatus::Pending => "◇",
        CompanionStatus::Running => "⏵",
        CompanionStatus::Complete => "✓",
        CompanionStatus::Failed => "✗",
        CompanionStatus::Cancelled => "◇",
    }
}
