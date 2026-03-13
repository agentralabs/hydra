mod render;
pub mod empty;

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

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let inner = area;

    let mut lines: Vec<Line> = Vec::new();

    // Build lines from messages — filter out internal noise
    let mut visible_idx: usize = 0;
    for msg in &app.messages {
        // Skip internal system messages that shouldn't be in conversation
        if msg.role == MessageRole::System {
            if let Some(ref phase) = msg.phase {
                // Hide ALL Repair/Omniscience/Decide internals — only show approval prompts
                if matches!(phase.as_str(), "Repair" | "Omniscience" | "Decide") {
                    if !msg.content.contains("RISK") { continue; }
                }
            }
            let c = &msg.content;
            let cl = c.to_lowercase();
            // Hide raw JSON, sister lists, diagnostics, spec dumps
            if cl.starts_with("sisters:") || cl.starts_with("{\"") || cl.starts_with("[{\"") { continue; }
            if c.contains("specs fully passing") || c.contains("/6 checks)") || c.contains("/4 checks)")
                || c.contains("/5 checks)") || c.contains("/2 checks)") { continue; }
            if c.contains("Self-repair diagnostics") { continue; }
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

    // Scroll — must count WRAPPED visual lines, not logical lines.
    // Ratatui wraps lines before scrolling, so a single logical line that
    // exceeds the terminal width becomes multiple visual rows.
    let visible_height = inner.height as usize;
    let w = inner.width.max(1) as usize;
    let total_lines: usize = lines.iter().map(|l| {
        let lw = l.width();
        if lw == 0 { 1 } else { (lw + w - 1) / w }
    }).sum();
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
