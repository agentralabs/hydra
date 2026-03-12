mod render;
mod empty;

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

use crate::tui::app::{App, MessageRole};
use crate::tui::theme;

use render::render_rich_content_ex;
use empty::render_empty_state;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    // No borders — full width, clean like Claude Code
    let inner = area;

    if app.messages.is_empty() {
        render_empty_state(frame, app, inner);
        return;
    }

    // Build lines from messages — filter out internal noise
    let mut lines: Vec<Line> = Vec::new();
    let mut visible_idx: usize = 0;
    for msg in &app.messages {
        // Skip internal system messages that shouldn't be in conversation
        if msg.role == MessageRole::System {
            if let Some(ref phase) = msg.phase {
                // Filter internal phases that are just noise
                if matches!(phase.as_str(),
                    "Repair" | "Omniscience" | "Decide"
                ) {
                    // Still show repair/omniscience completion summaries (short messages)
                    let is_summary = msg.content.contains("complete")
                        || msg.content.contains("Complete")
                        || msg.content.contains("RISK");
                    if !is_summary {
                        continue;
                    }
                }
            }
            // Filter system messages that dump sister lists or raw JSON
            let content_lower = msg.content.to_lowercase();
            if content_lower.starts_with("sisters:") || content_lower.starts_with("{\"")
                || content_lower.starts_with("[{\"")
            {
                continue;
            }
        }

        // Phase 2, Bug Fix 0C: Hide internal cognitive phase tags from conversation
        {
            let c = &msg.content;
            if c.contains("[Think]") || c.contains("[Act]") || c.contains("[Learn")
                || c.contains("[Diagnostics]") || c.contains("[Think (Forge")
                || c.contains("Step 1 complete") || c.contains("Step 2 complete")
                || c.contains("Step 3 complete")
            {
                continue;
            }
            // Hide sister list dumps (e.g., "● Memory, ● Identity, ... ● Evolve")
            if c.contains("● Memory,") && c.contains("● Evolve") {
                continue;
            }
            // Hide generic auto-generated plans
            if c.contains("1. Analyze request")
                && c.contains("2. Execute task")
                && c.contains("3. Verify outcome")
            {
                continue;
            }
        }

        // Blank line between messages (no separators — just whitespace)
        if visible_idx > 0 {
            lines.push(Line::default());
        }
        visible_idx += 1;

        // Message rendering — clean, Claude Code style
        match msg.role {
            MessageRole::User => {
                // User messages: "> text" — simple prefix, no label/timestamp
                lines.push(Line::from(vec![
                    Span::styled("> ", theme::prompt()),
                    Span::styled(msg.content.clone(), theme::user_msg()),
                ]));
            }
            MessageRole::Hydra => {
                // Hydra responses: just flowing text, no label prefix
                render_rich_content_ex(&msg.content, msg.role.clone(), &mut lines, app.tool_output_expanded);
            }
            MessageRole::System => {
                // System messages: render content with rich formatting
                render_rich_content_ex(&msg.content, msg.role.clone(), &mut lines, app.tool_output_expanded);
            }
        }

        // Blank line after each message
        lines.push(Line::default());
    }

    // Show running command indicator
    if app.running_cmd.is_some() {
        let spinner = match (app.tick_count / 2) % 4 {
            0 => "⠋",
            1 => "⠙",
            2 => "⠹",
            _ => "⠸",
        };
        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", spinner), Style::default().fg(theme::HYDRA_YELLOW)),
            Span::styled("Command running... (Ctrl+K to kill)", Style::default().fg(theme::HYDRA_YELLOW)),
        ]));
    }

    // Phase 3, C5.3: Meaningful loading states with phase-specific messages
    if app.is_thinking && app.running_cmd.is_none() {
        // Rotating spinner with 4 distinct frames for visual progress
        let spinners = ["◐", "◓", "◑", "◒"];
        let spinner = spinners[(app.tick_count / 3) as usize % 4];
        let status = if app.thinking_status.is_empty() {
            "Thinking...".to_string()
        } else {
            app.thinking_status.clone()
        };
        // Show elapsed time for long-running phases
        let elapsed = if app.thinking_elapsed_ms > 0 {
            format!("  ({:.1}s)", app.thinking_elapsed_ms as f64 / 1000.0)
        } else {
            String::new()
        };
        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", spinner), Style::default().fg(theme::HYDRA_CYAN)),
            Span::styled(status, Style::default().fg(theme::HYDRA_CYAN)),
            Span::styled(elapsed, Style::default().fg(theme::HYDRA_DIM)),
        ]));
    }

    // Calculate scroll — scroll_offset is "lines from bottom" (0 = pinned to bottom)
    let visible_height = inner.height as usize;
    let total_lines = lines.len();
    let max_scroll = total_lines.saturating_sub(visible_height);
    let scroll_from_top = max_scroll.saturating_sub(app.scroll_offset.min(max_scroll));

    let para = Paragraph::new(lines)
        .scroll((scroll_from_top as u16, 0))
        .wrap(Wrap { trim: false });

    frame.render_widget(para, inner);

    // Show scroll indicator when not at bottom
    if app.scroll_offset > 0 && total_lines > visible_height {
        let lines_above = app.scroll_offset.min(max_scroll);
        let indicator = format!(" ▼ {} more below — Shift+Down or PageDown ", lines_above);
        let indicator_width = indicator.len().min(inner.width as usize);
        let x = inner.x + inner.width.saturating_sub(indicator_width as u16);
        let y = inner.y + inner.height.saturating_sub(1);
        let indicator_area = Rect::new(x, y, indicator_width as u16, 1);
        let badge = Paragraph::new(Line::from(Span::styled(
            &indicator[..indicator_width],
            Style::default()
                .fg(theme::HYDRA_BG)
                .bg(theme::HYDRA_YELLOW),
        )));
        frame.render_widget(badge, indicator_area);
    }
}
