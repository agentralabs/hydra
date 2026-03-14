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
use crate::tui::app_helpers::TIPS;
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

        // Hide internal cognitive phase tags — ONLY for System messages
        if msg.role == MessageRole::System {
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
                // ❯ prompt with bold white text
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

    // ── Agent tree ──
    if !app.running_sub_agents.is_empty() {
        let agent_count = app.running_sub_agents.len();
        render_agent_tree(app, agent_count, &mut lines);
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

    // ── Thinking indicator ──
    if app.is_thinking && app.running_cmd.is_none() {
        // Asterisk spinner cycle
        let spinners = ["·", "✢", "✳", "∗", "✻", "✽"];
        let spinner = spinners[(app.tick_count / 3) as usize % spinners.len()];
        let status = if app.thinking_status.is_empty() {
            "Thinking...".to_string()
        } else {
            app.thinking_status.clone()
        };
        // Elapsed time: use task start tick for accurate per-task timing
        let elapsed_ticks = app.tick_count.saturating_sub(app.task_stats.start_tick);
        let elapsed = format!("{}s", elapsed_ticks / 20);
        let task_tokens = app.tokens_received.saturating_sub(app.task_stats.tokens_start);
        let tokens = format_tokens(task_tokens);
        let meta = format!("({} · ↓ {} tokens · {})", elapsed, tokens, status);

        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", spinner), Style::default().fg(theme::HYDRA_BLUE)),
            Span::styled(status.clone(), Style::default().fg(theme::HYDRA_BLUE).add_modifier(Modifier::BOLD)),
            Span::styled(format!("  {}", meta), Style::default().fg(theme::HYDRA_DIM)),
        ]));

        // Show a tip during thinking
        let tip_idx = (app.tick_count / 200) as usize % TIPS.len();
        lines.push(Line::from(vec![
            Span::styled("    └ ", Style::default().fg(theme::HYDRA_DIM)),
            Span::styled(
                format!("Tip: {}", TIPS[tip_idx]),
                Style::default().fg(theme::HYDRA_DIM),
            ),
        ]));
    }

    // Scroll — count wrapped visual lines
    let visible_height = inner.height as usize;
    let w = inner.width.max(1) as usize;
    let total_lines: usize = lines.iter().map(|l| {
        let lw = l.width();
        if lw <= w { 1 } else { (lw + w - 1) / w + 1 }
    }).sum();
    let max_scroll = total_lines.saturating_sub(visible_height);

    // Dual scroll model:
    // - scroll_pinned_top = Some(row) → use absolute row from top (stable when content grows)
    // - scroll_pinned_top = None → auto-scroll to bottom
    let scroll_from_top = if let Some(pinned) = app.scroll_pinned_top {
        if pinned == usize::MAX {
            // Sentinel: user just started scrolling up — compute from current bottom position
            max_scroll.saturating_sub(app.scroll_offset.min(max_scroll))
        } else {
            pinned.min(max_scroll)
        }
    } else {
        max_scroll // at bottom
    };

    // Clamp to u16 range for Ratatui (unlikely to exceed but safe)
    let scroll_y = scroll_from_top.min(u16::MAX as usize) as u16;
    let para = Paragraph::new(lines)
        .scroll((scroll_y, 0))
        .wrap(Wrap { trim: false });
    frame.render_widget(para, inner);

    // Scroll indicators
    let is_scrolled = app.scroll_pinned_top.is_some();
    if is_scrolled {
        // Show "↑ Home to jump to top" when not at the very top
        if scroll_from_top > 0 {
            let top_hint = " ↑ Home to scroll to top ";
            let tw = top_hint.len().min(inner.width as usize);
            let badge = Paragraph::new(Line::from(Span::styled(
                &top_hint[..tw],
                Style::default().fg(theme::HYDRA_BG).bg(theme::HYDRA_BLUE),
            )));
            frame.render_widget(badge, Rect::new(inner.x + inner.width.saturating_sub(tw as u16), inner.y, tw as u16, 1));
        }
        // Show "↓ End to jump to latest" at bottom
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

/// Render the agent tree with branches and stats.
fn render_agent_tree(app: &App, count: usize, lines: &mut Vec<Line<'static>>) {
    // Header: ● Running N agents... (ctrl+o to expand)
    lines.push(Line::from(vec![
        Span::styled("  ● ", Style::default().fg(theme::HYDRA_BLUE)),
        Span::styled(
            format!("Running {} agent{}...", count, if count > 1 { "s" } else { "" }),
            Style::default().fg(theme::HYDRA_BLUE).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  (ctrl+o to expand)", Style::default().fg(theme::HYDRA_DIM)),
    ]));

    // Tree branches for each sub-agent
    for (i, agent) in app.running_sub_agents.iter().enumerate() {
        let is_last = i == count - 1;
        let branch = if is_last { "└─" } else { "├─" };
        let desc = if agent.description.is_empty() {
            format!("Agent {}", agent.id)
        } else {
            agent.description.clone()
        };
        // Agent line: ├─ Description · N tool uses
        let stats = if agent.tool_uses > 0 {
            format!(" · {} tool use{}", agent.tool_uses, if agent.tool_uses > 1 { "s" } else { "" })
        } else { String::new() };
        let tokens = if agent.tokens > 0 {
            format!(" · {} tokens", format_tokens(agent.tokens))
        } else { String::new() };

        lines.push(Line::from(vec![
            Span::styled(format!("    {} ", branch), Style::default().fg(theme::HYDRA_DIM)),
            Span::styled(desc, Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(format!("{}{}", stats, tokens), Style::default().fg(theme::HYDRA_DIM)),
        ]));

        // Activity detail line
        if !agent.activity.is_empty() {
            let indent = if is_last { "      " } else { "    │ " };
            lines.push(Line::from(vec![
                Span::styled(format!("{}└ ", indent), Style::default().fg(theme::HYDRA_DIM)),
                Span::styled(
                    agent.activity.clone(),
                    Style::default().fg(theme::HYDRA_DIM),
                ),
            ]));
        }
    }

    // Hint at bottom of tree
    lines.push(Line::from(Span::styled(
        "    ctrl+b to run in background",
        Style::default().fg(theme::HYDRA_DIM),
    )));
}

/// Format token count for display: 1234 → "1.2k", 12345 → "12.3k"
fn format_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}k", tokens as f64 / 1_000.0)
    } else {
        format!("{}", tokens)
    }
}
