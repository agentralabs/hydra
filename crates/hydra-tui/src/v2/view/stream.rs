//! Stream view — renders conversation items, thinking indicator, scrollbar.
//! Claude Code style: clean, no borders, subtle separators.

use super::RenderState;
use crate::stream_types::StreamItem;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap};
use ratatui::Frame;

/// Render the conversation stream.
pub fn render(frame: &mut Frame, area: ratatui::layout::Rect, state: &RenderState) {

    // Convert stream items to lines (Fix 9: viewport-only rendering)
    let mut lines: Vec<Line> = Vec::new();
    let all_items: &[StreamItem] = &state.stream_items;

    // Estimate visible range: each item produces ~2-4 lines on average
    let viewport_lines = area.height as usize;
    let lines_per_item = 3usize; // conservative estimate
    let items_needed = (viewport_lines / lines_per_item) + 10; // +10 overscan
    let total = all_items.len();
    let end = total.saturating_sub(state.stream_scroll_offset);
    let start = end.saturating_sub(items_needed);
    let visible_items = &all_items[start..end];

    for item in visible_items {
        match item {
            StreamItem::UserMessage { text, .. } => {
                // Conversation text: black on light theme, white on dark theme
                let text_color = conversation_text_color(&state.theme);
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("  you: ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::styled(text.clone(), Style::default().fg(text_color)),
                ]));
            }
            StreamItem::AssistantText { text, .. } => {
                if text.is_empty() { continue; }
                let clean = strip_receipt_lines(text);
                if clean.trim().is_empty() { continue; }
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("  hydra: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                ]));
                // O22: Check for rich output structures before markdown rendering
                let rich = hydra_kernel::rich_output::classify_output(&clean);
                match &rich {
                    hydra_kernel::rich_output::RichOutput::Table { headers, rows } => {
                        for rl in crate::v2::view::rich::render_table(headers, rows, area.width) {
                            lines.push(Line::from(vec![Span::raw("  "), Span::styled(rl, Style::default().fg(Color::White))]));
                        }
                    }
                    hydra_kernel::rich_output::RichOutput::Chart { title, labels, values, unit } => {
                        for rl in crate::v2::view::rich::render_chart(title, labels, values, unit, area.width) {
                            lines.push(Line::from(vec![Span::raw("  "), Span::styled(rl, Style::default().fg(Color::Cyan))]));
                        }
                    }
                    hydra_kernel::rich_output::RichOutput::Timeline { events } => {
                        for rl in crate::v2::view::rich::render_timeline(events) {
                            lines.push(Line::from(vec![Span::raw("  "), Span::styled(rl, Style::default().fg(Color::Yellow))]));
                        }
                    }
                    hydra_kernel::rich_output::RichOutput::Progress { tasks } => {
                        for rl in crate::v2::view::rich::render_progress(tasks, area.width) {
                            lines.push(Line::from(vec![Span::raw("  "), Span::styled(rl, Style::default().fg(Color::Green))]));
                        }
                    }
                    hydra_kernel::rich_output::RichOutput::Diff { hunks } => {
                        for rl in crate::v2::view::rich::render_diff(hunks) {
                            let color = if rl.starts_with('+') { Color::Green }
                                else if rl.starts_with('-') { Color::Red }
                                else if rl.starts_with("@@") { Color::Cyan }
                                else { Color::DarkGray };
                            lines.push(Line::from(vec![Span::raw("  "), Span::styled(rl, Style::default().fg(color))]));
                        }
                    }
                    _ => {
                        // Default: render as markdown (existing path)
                        let md_lines = crate::render_markdown::render_assistant_text(&clean, &state.theme);
                        for ml in md_lines {
                            let mut spans = vec![Span::raw("  ")];
                            spans.extend(ml.spans);
                            lines.push(Line::from(spans));
                        }
                    }
                }
            }
            StreamItem::ToolDot { tool_name, kind, .. } => {
                let (symbol, color) = dot_style(&format!("{:?}", kind));
                // Compact: short name, very dim, minimal space
                let short = if tool_name.len() > 30 { format!("{}...", &tool_name[..27]) } else { tool_name.clone() };
                lines.push(Line::from(vec![
                    Span::styled(format!("  {symbol} "), Style::default().fg(color).add_modifier(Modifier::DIM)),
                    Span::styled(short, Style::default().fg(Color::Rgb(65, 65, 75)).add_modifier(Modifier::DIM)),
                ]));
            }
            StreamItem::ToolConnector { .. } => {
                // Skip connectors — too verbose. Dots are enough.
            }
            StreamItem::SystemNotification { content, .. } => {
                let is_box = content.starts_with('┌') || content.starts_with('└');
                let is_metadata = content.starts_with('[') && content.ends_with(']');
                let (color, dim) = if is_box {
                    (Color::Rgb(45, 94, 58), Modifier::empty()) // Green box borders
                } else if is_metadata {
                    (Color::Rgb(70, 70, 80), Modifier::DIM)
                } else {
                    (Color::Rgb(90, 90, 100), Modifier::empty())
                };
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(content.clone(), Style::default().fg(color).add_modifier(dim)),
                ]));
            }
            StreamItem::ThinkingPill { duration_secs } => {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        format!("Thought for {duration_secs:.1}s"),
                        Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM),
                    ),
                ]));
            }
            StreamItem::BeliefCitation { belief, confidence, .. } => {
                let border_color = if *confidence > 0.85 {
                    Color::Green
                } else if *confidence > 0.50 {
                    Color::Yellow
                } else {
                    Color::Red
                };
                let conf_pct = (confidence * 100.0) as u32;
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  ┌─ Belief ({conf_pct}%) "),
                        Style::default().fg(border_color),
                    ),
                    Span::styled("─".repeat(30), Style::default().fg(border_color)),
                    Span::styled("┐", Style::default().fg(border_color)),
                ]));
                for bl in belief.lines().take(3) {
                    lines.push(Line::from(vec![
                        Span::styled("  │ ", Style::default().fg(border_color)),
                        Span::raw(bl.to_string()),
                    ]));
                }
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  └{}┘", "─".repeat(40)),
                        Style::default().fg(border_color),
                    ),
                ]));
            }
            StreamItem::DreamNotification { content, .. } => {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("● ", Style::default().fg(Color::Rgb(100, 80, 180))),
                    Span::styled("[Dream] ", Style::default().fg(Color::Rgb(100, 80, 180)).add_modifier(Modifier::BOLD)),
                    Span::styled(content.clone(), Style::default().fg(Color::DarkGray)),
                ]));
            }
            StreamItem::BriefingItem { content, priority, .. } => {
                let (sym, color) = match priority {
                    crate::stream_types::BriefingPriority::Urgent => ("▲", Color::Red),
                    crate::stream_types::BriefingPriority::High => ("●", Color::Rgb(200, 169, 110)),
                    crate::stream_types::BriefingPriority::Normal => ("●", Color::Gray),
                    crate::stream_types::BriefingPriority::Low => ("○", Color::DarkGray),
                };
                // Inside bordered box — add │ side borders
                let border_color = Color::Rgb(45, 94, 58);
                lines.push(Line::from(vec![
                    Span::styled("  │ ", Style::default().fg(border_color)),
                    Span::styled(format!("{sym} "), Style::default().fg(color)),
                    Span::styled(content.clone(), Style::default().fg(color)),
                ]));
            }
            StreamItem::AgentStep { step_number, action, observation, is_complete, .. } => {
                let status = if *is_complete { "done" } else { "ok" };
                let color = if *is_complete { Color::Green } else { Color::Cyan };
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        format!("Step {step_number}"),
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!(" {action}"),
                        Style::default().fg(Color::White),
                    ),
                    Span::styled(
                        format!(" [{status}]"),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
                if !observation.is_empty() && observation != "Clicked" && observation != "Typed" {
                    lines.push(Line::from(vec![
                        Span::raw("    "),
                        Span::styled(observation.clone(), Style::default().fg(Color::DarkGray)),
                    ]));
                }
            }
            StreamItem::AlertFrame { title, lines: alert_lines, .. } => {
                let w = 54usize;
                let border = Style::default().fg(Color::Red);
                let title_pad = w.saturating_sub(title.len() + 6);
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    format!("  ┌─ ▲ {title} {}┐", "─".repeat(title_pad)), border)));
                lines.push(Line::from(Span::styled(
                    format!("  │{}│", " ".repeat(w)), border)));
                for al in alert_lines {
                    let pad = w.saturating_sub(al.len() + 2);
                    lines.push(Line::from(vec![
                        Span::styled("  │ ", border),
                        Span::styled(format!("▲ {al}"), Style::default().fg(Color::Red)),
                        Span::styled(format!("{:pad$}│", "", pad = pad.saturating_sub(2)), border),
                    ]));
                }
                lines.push(Line::from(Span::styled(
                    format!("  │{}│", " ".repeat(w)), border)));
                lines.push(Line::from(Span::styled(
                    format!("  └{}┘", "─".repeat(w)), border)));
                lines.push(Line::from(""));
            }
            StreamItem::Blank => {
                lines.push(Line::from(""));
            }
            _ => {}
        }
    }

    // Thinking indicator with spinner
    if state.is_thinking {
        let spinners = ['◌', '◐', '◑', '◒', '◓', '●'];
        let spinner = spinners[state.think_spinner_frame % spinners.len()];
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("{spinner} "),
                Style::default().fg(state.thinking_color),
            ),
            Span::styled(
                &*state.thinking_verb,
                Style::default().fg(state.thinking_color).add_modifier(Modifier::ITALIC),
            ),
            Span::styled("...", Style::default().fg(Color::DarkGray)),
        ]));
    }

    // Calculate scroll — estimate visual lines including wrapping
    let width = area.width.max(1) as usize;
    let estimated_total: usize = lines.iter()
        .map(|line| {
            let line_width: usize = line.spans.iter().map(|s| s.content.len()).sum();
            (line_width / width).max(1) // at least 1 visual line per logical line
        })
        .sum();
    let visible_height = area.height as usize;
    let max_scroll = estimated_total.saturating_sub(visible_height);
    let scroll_offset = if state.stream_scroll_offset == 0 {
        // Auto-scroll: only scroll when content exceeds viewport
        // This keeps briefing visible until conversation fills the screen
        max_scroll
    } else {
        max_scroll.saturating_sub(state.stream_scroll_offset)
    };

    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((scroll_offset as u16, 0));

    frame.render_widget(paragraph, area);

    // "▼ more below" indicator when scrolled up
    if state.stream_scroll_offset > 0 {
        let indicator_area = Rect {
            y: area.y + area.height.saturating_sub(1),
            height: 1,
            ..area
        };
        let label = if state.new_while_scrolled > 0 {
            format!("  ↓ {} new (PageDown)", state.new_while_scrolled)
        } else {
            "  ↓ more below (PageDown)".into()
        };
        let badge_color = if state.new_while_scrolled > 0 { Color::Cyan } else { Color::Rgb(60, 60, 70) };
        let indicator = Paragraph::new(Line::from(vec![
            Span::styled(label, Style::default().fg(badge_color).add_modifier(Modifier::DIM)),
        ]));
        frame.render_widget(indicator, indicator_area);
    }

    // Scrollbar (only if content overflows)
    if estimated_total > visible_height {
        let mut scrollbar_state = ScrollbarState::new(estimated_total)
            .position(scroll_offset)
            .viewport_content_length(visible_height);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None);
        frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}

/// Get symbol and color for a tool dot kind.
fn dot_style(kind: &str) -> (&'static str, Color) {
    match kind {
        "read" | "Read" => ("◉", Color::Cyan),
        "write" | "Write" => ("◆", Color::Yellow),
        "cognitive" | "Cognitive" => ("◆", Color::Magenta),
        "narration" | "Narration" => ("●", Color::Blue),
        "workflow" | "Workflow" => ("▲", Color::Green),
        "security" | "Security" => ("▲", Color::Red),
        "system" | "System" => ("●", Color::White),
        _ => ("●", Color::DarkGray),
    }
}

/// Get the conversation text color based on theme.
/// Light theme: black text. Dark theme: white text.
fn conversation_text_color(theme: &crate::theme::Theme) -> Color {
    if theme.name() == "light" {
        Color::Rgb(10, 10, 10)
    } else {
        Color::Rgb(240, 240, 240)
    }
}

/// Strip emoji characters from text. Terminal TUI should be text-only.
fn strip_emojis(line: &str) -> String {
    line.chars().filter(|c| {
        let cp = *c as u32;
        // Keep ASCII + Latin + box drawing, strip emoji ranges (U+1F000+)
        cp < 0x1F000 && !(0x2600..0x2700).contains(&cp)
    }).collect::<String>().replace("  ", " ")
}

/// Strip ALL audit/receipt/metadata from LLM output. Aggressive filter.
fn strip_receipt_lines(text: &str) -> String {
    text.lines()
        .filter(|line| {
            let t = line.trim();
            let tl = t.to_lowercase();
            // Receipt lines (all forms: receipt, receipted, Receipt:, *Receipt:)
            if tl.contains("receipt") { return false; }
            // Confidence/evidence/attribution metadata
            if tl.contains("evidence_basis") || tl.contains("constitutional_compliance") { return false; }
            if tl.contains("confidence:") && t.len() < 80 { return false; }
            if tl.starts_with("action:") || tl.starts_with("attribution:") { return false; }
            if t.starts_with("*Action:") || t.starts_with("*Attribution:") || t.starts_with("*Confidence:") { return false; }
            if t.starts_with("*Receipt:") || t.starts_with("**Receipt:") { return false; }
            // Constitutional/operational markers
            if tl.contains("[operational protocol]") || tl.contains("[evidence_basis]") { return false; }
            if tl.contains("constitutional compliance") || tl.contains("capability overreach") { return false; }
            if t.starts_with("[Constitutional") || t.starts_with("[constitutional") { return false; }
            // Horizontal rules / separators (Unicode box drawing + markdown HR)
            if t.chars().all(|c| "─—_━▬═-*".contains(c)) && t.len() > 3 { return false; }
            if t.starts_with("---") || t.starts_with("___") || t.starts_with("***") { return false; }
            true
        })
        // Strip emojis from the entire text
        .map(strip_emojis)
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dot_style_read() {
        let (sym, color) = dot_style("Read");
        assert_eq!(sym, "◉");
        assert_eq!(color, Color::Cyan);
    }

    #[test]
    fn dot_style_unknown() {
        let (sym, _) = dot_style("unknown");
        assert_eq!(sym, "●");
    }
}
