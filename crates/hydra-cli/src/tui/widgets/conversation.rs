mod render;
mod empty;

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

use crate::tui::app::{App, MessageRole};
use crate::tui::theme;

use render::render_rich_content_ex;
use empty::build_welcome_frame;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let inner = area;

    let mut lines: Vec<Line> = Vec::new();

    // Welcome frame always at top — scroll up to see it
    build_welcome_frame(app, inner.width as usize, &mut lines);
    lines.push(Line::default());

    // Build lines from messages — filter out internal noise
    let mut visible_idx: usize = 0;
    for msg in &app.messages {
        // Skip internal system messages that shouldn't be in conversation
        if msg.role == MessageRole::System {
            if let Some(ref phase) = msg.phase {
                if matches!(phase.as_str(), "Repair" | "Omniscience" | "Decide") {
                    let is_summary = msg.content.contains("complete")
                        || msg.content.contains("Complete")
                        || msg.content.contains("RISK");
                    if !is_summary { continue; }
                }
            }
            let content_lower = msg.content.to_lowercase();
            if content_lower.starts_with("sisters:")
                || content_lower.starts_with("{\"")
                || content_lower.starts_with("[{\"")
            {
                continue;
            }
        }

        // Hide internal cognitive phase tags
        {
            let c = &msg.content;
            if c.contains("[Think]") || c.contains("[Act]") || c.contains("[Learn")
                || c.contains("[Diagnostics]") || c.contains("[Think (Forge")
                || c.contains("Step 1 complete") || c.contains("Step 2 complete")
                || c.contains("Step 3 complete")
            { continue; }
            if c.contains("● Memory,") && c.contains("● Evolve") { continue; }
            if c.contains("1. Analyze request")
                && c.contains("2. Execute task")
                && c.contains("3. Verify outcome")
            { continue; }
        }

        if visible_idx > 0 { lines.push(Line::default()); }
        visible_idx += 1;

        match msg.role {
            MessageRole::User => {
                // Claude Code style: ❯ prompt with bold white text
                lines.push(Line::from(vec![
                    Span::styled("❯ ", Style::default().fg(theme::HYDRA_BLUE).add_modifier(Modifier::BOLD)),
                    Span::styled(msg.content.clone(), Style::default().add_modifier(Modifier::BOLD)),
                ]));
            }
            MessageRole::Hydra | MessageRole::System => {
                render_rich_content_ex(&msg.content, msg.role.clone(), &mut lines, app.tool_output_expanded);
            }
        }
        lines.push(Line::default());
    }

    // Running command indicator
    if app.running_cmd.is_some() {
        let spinner = match (app.tick_count / 2) % 4 {
            0 => "⠋", 1 => "⠙", 2 => "⠹", _ => "⠸",
        };
        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", spinner), Style::default().fg(theme::HYDRA_YELLOW)),
            Span::styled("Command running... (Ctrl+K to kill)", Style::default().fg(theme::HYDRA_YELLOW)),
        ]));
    }

    // Thinking indicator
    if app.is_thinking && app.running_cmd.is_none() {
        let spinners = ["◐", "◓", "◑", "◒"];
        let spinner = spinners[(app.tick_count / 3) as usize % 4];
        let status = if app.thinking_status.is_empty() {
            "Thinking...".to_string()
        } else {
            app.thinking_status.clone()
        };
        let elapsed = if app.thinking_elapsed_ms > 0 {
            format!("  ({:.1}s)", app.thinking_elapsed_ms as f64 / 1000.0)
        } else { String::new() };
        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", spinner), Style::default().fg(theme::HYDRA_CYAN)),
            Span::styled(status, Style::default().fg(theme::HYDRA_CYAN)),
            Span::styled(elapsed, Style::default().fg(theme::HYDRA_DIM)),
        ]));
    }

    // Scroll
    let visible_height = inner.height as usize;
    let total_lines = lines.len();
    let max_scroll = total_lines.saturating_sub(visible_height);
    let scroll_from_top = max_scroll.saturating_sub(app.scroll_offset.min(max_scroll));

    let para = Paragraph::new(lines)
        .scroll((scroll_from_top as u16, 0))
        .wrap(Wrap { trim: false });
    frame.render_widget(para, inner);

    // Scroll indicator — show when not pinned to bottom
    if app.scroll_offset > 0 && total_lines > visible_height {
        let indicator = " ↓ End to jump to latest ";
        let iw = indicator.len().min(inner.width as usize);
        let x = inner.x + inner.width.saturating_sub(iw as u16);
        let y = inner.y + inner.height.saturating_sub(1);
        let badge = Paragraph::new(Line::from(Span::styled(
            &indicator[..iw],
            Style::default().fg(theme::HYDRA_BG).bg(theme::HYDRA_CYAN),
        )));
        frame.render_widget(badge, Rect::new(x, y, iw as u16, 1));
    }
}
