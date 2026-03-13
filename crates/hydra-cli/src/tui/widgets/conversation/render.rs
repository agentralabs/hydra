use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use crate::tui::app::MessageRole;
use crate::tui::theme;

/// Render message content with rich formatting: code blocks, diffs, tool use, etc.
/// Render message content with rich formatting.
/// `expand_tools` controls whether tool output is collapsed or expanded (Ctrl+O toggle).
pub fn render_rich_content_ex(content: &str, role: MessageRole, lines: &mut Vec<Line<'static>>, expand_tools: bool) {
    render_rich_content_inner(content, role, lines, expand_tools);
}

pub fn render_rich_content(content: &str, role: MessageRole, lines: &mut Vec<Line<'static>>) {
    render_rich_content_inner(content, role, lines, false);
}

fn render_rich_content_inner(content: &str, role: MessageRole, lines: &mut Vec<Line<'static>>, expand_tools: bool) {
    let content_lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < content_lines.len() {
        let line = content_lines[i];

        // ── Markdown headers: ### Title → bold colored text ──
        if line.starts_with("### ") {
            let header_text = line.trim_start_matches('#').trim();
            let clean = strip_markdown_bold(header_text);
            lines.push(Line::from(Span::styled(
                format!("  {}", clean),
                Style::default()
                    .fg(theme::HYDRA_BLUE)
                    .add_modifier(Modifier::BOLD),
            )));
            i += 1;
            continue;
        }
        if line.starts_with("## ") {
            let header_text = line.trim_start_matches('#').trim();
            let clean = strip_markdown_bold(header_text);
            lines.push(Line::from(Span::styled(
                format!("  {}", clean),
                Style::default()
                    .fg(theme::HYDRA_CYAN)
                    .add_modifier(Modifier::BOLD),
            )));
            i += 1;
            continue;
        }

        // Detect diff lines (starts with + or - in a diff context)
        if line.starts_with('+') && !line.starts_with("+++") {
            lines.push(Line::from(Span::styled(
                format!("  {}", line),
                Style::default().fg(theme::HYDRA_GREEN),
            )));
            i += 1;
            continue;
        }
        if line.starts_with('-') && !line.starts_with("---") {
            lines.push(Line::from(Span::styled(
                format!("  {}", line),
                Style::default().fg(theme::HYDRA_RED),
            )));
            i += 1;
            continue;
        }

        // Detect diff header lines
        if line.starts_with("diff ") || line.starts_with("@@") {
            lines.push(Line::from(Span::styled(
                format!("  {}", line),
                Style::default().fg(theme::HYDRA_PURPLE),
            )));
            i += 1;
            continue;
        }

        // Detect file headers in /open output: "--- path (lang, N lines) ---"
        if line.starts_with("--- ") && line.ends_with(" ---") {
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(
                    line.to_string(),
                    Style::default()
                        .fg(theme::HYDRA_BLUE)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            i += 1;
            continue;
        }

        // Detect line-numbered code (from /open): "   N | code"
        if line.len() > 6 {
            let trimmed = line.trim_start();
            if let Some(pipe_pos) = trimmed.find(" | ") {
                let num_part = &trimmed[..pipe_pos];
                if num_part.chars().all(|c| c.is_ascii_digit()) {
                    lines.push(Line::from(vec![
                        Span::styled("  ", Style::default()),
                        Span::styled(
                            format!("{} ", num_part),
                            Style::default().fg(theme::HYDRA_DIM),
                        ),
                        Span::styled(
                            format!("| {}", &trimmed[pipe_pos + 3..]),
                            Style::default(), // terminal default for code
                        ),
                    ]));
                    i += 1;
                    continue;
                }
            }
        }

        // Phase 3, C5.2: Collapsible tool results — collapse long tool output to one-liner
        // When expand_tools is true (Ctrl+O toggled), skip collapsing
        if !expand_tools {
        if let Some(summary) = should_collapse_tool_result(line) {
            lines.push(Line::from(vec![
                Span::styled("  ● ", Style::default().fg(theme::HYDRA_GREEN)),
                Span::styled(summary, Style::default().fg(theme::HYDRA_DIM)),
            ]));
            i += 1;
            while i < content_lines.len() {
                let next = content_lines[i];
                if next.is_empty() || next.starts_with("Using ") || next.starts_with("●") || next.starts_with("## ") { break; }
                i += 1;
            }
            continue;
        }
        } // end !expand_tools

        // Claude Code-style tool action: "● ToolName(args)" header line
        if line.starts_with("● ") {
            let tool_text = line.trim_start_matches("● ");
            lines.push(Line::from(vec![
                Span::styled("  ● ", Style::default().fg(theme::HYDRA_GREEN)),
                Span::styled(
                    tool_text.to_string(),
                    Style::default().fg(theme::HYDRA_GREEN).add_modifier(Modifier::BOLD),
                ),
            ]));
            i += 1;
            // Collect result lines: "  └ result" or "  ✗ error"
            while i < content_lines.len() {
                let next = content_lines[i];
                if next.trim_start().starts_with("└ ") {
                    let result_text = next.trim_start().trim_start_matches("└ ");
                    lines.push(Line::from(vec![
                        Span::styled("    └ ", Style::default().fg(theme::HYDRA_DIM)),
                        Span::styled(result_text.to_string(), Style::default().fg(theme::HYDRA_DIM)),
                    ]));
                    i += 1;
                } else if next.trim_start().starts_with("✗ ") {
                    let err_text = next.trim_start().trim_start_matches("✗ ");
                    lines.push(Line::from(vec![
                        Span::styled("    ✗ ", Style::default().fg(theme::HYDRA_RED)),
                        Span::styled(err_text.to_string(), Style::default().fg(theme::HYDRA_RED)),
                    ]));
                    i += 1;
                } else { break; }
            }
            continue;
        }

        // Legacy tool use: "Using Sister: tool(args)" — reformat as ● dot style
        if line.starts_with("Using ") && line.contains(": ") {
            // Extract sister and tool from "Using Sister: tool(args)"
            if let Some(colon_pos) = line.find(": ") {
                let sister = &line[6..colon_pos]; // skip "Using "
                let tool_part = &line[colon_pos + 2..];
                lines.push(Line::from(vec![
                    Span::styled("  ● ", Style::default().fg(theme::HYDRA_GREEN)),
                    Span::styled(format!("{}", tool_part), Style::default().fg(theme::HYDRA_GREEN).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("  ({})", sister), Style::default().fg(theme::HYDRA_DIM)),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled("  ● ", Style::default().fg(theme::HYDRA_GREEN)),
                    Span::styled(line.to_string(), Style::default().fg(theme::HYDRA_GREEN)),
                ]));
            }
            i += 1;
            while i < content_lines.len() {
                let next = content_lines[i];
                if next.is_empty() || next.starts_with("Using ") || next.starts_with("●") || next.starts_with("## ") { break; }
                let trimmed = next.trim();
                if !trimmed.is_empty() && (next.starts_with("  ") || next.starts_with("\t")) {
                    lines.push(Line::from(vec![
                        Span::styled("    └ ", Style::default().fg(theme::HYDRA_DIM)),
                        Span::styled(trimmed.to_string(), Style::default().fg(theme::HYDRA_DIM)),
                    ]));
                    i += 1;
                } else { break; }
            }
            continue;
        }

        // Detect sisters line: "Sisters: X, Y, Z"
        if line.starts_with("Sisters: ") && role == MessageRole::System {
            let sisters_str = &line[9..];
            let mut spans = vec![
                Span::styled("  ", Style::default()),
            ];
            for (j, sister) in sisters_str.split(", ").enumerate() {
                if j > 0 {
                    spans.push(Span::styled(", ", theme::dim()));
                }
                spans.push(Span::styled(format!("◉ {}", sister), Style::default().fg(theme::HYDRA_CYAN)));
            }
            lines.push(Line::from(spans));
            i += 1;
            continue;
        }

        // Detect command output: "$ command"
        if line.starts_with("$ ") {
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(
                    line.to_string(),
                    Style::default()
                        .fg(theme::HYDRA_GREEN)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            i += 1;
            continue;
        }

        // Detect thinking blocks: "[thinking...]" or "<thinking>" (§3.4)
        if line.trim_start().starts_with("[thinking") || line.trim_start().starts_with("<thinking>") {
            let ts = Style::default().fg(theme::HYDRA_DIM);
            let bs = Style::default().fg(theme::HYDRA_BLUE);
            lines.push(Line::from(vec![
                Span::styled("  ┌ ", bs),
                Span::styled("Thinking ", bs.add_modifier(Modifier::BOLD)),
                Span::styled("─".repeat(40), bs), Span::styled("┐", bs),
            ]));
            i += 1;
            while i < content_lines.len() {
                let tl = content_lines[i].trim_start();
                if tl.starts_with("[/thinking") || tl.starts_with("</thinking>") || tl.is_empty() {
                    i += 1; break;
                }
                lines.push(Line::from(vec![Span::styled("  │ ", bs), Span::styled(tl.trim().to_string(), ts)]));
                i += 1;
            }
            lines.push(Line::from(vec![Span::styled("  └", bs), Span::styled("─".repeat(44), bs), Span::styled("┘", bs)]));
            continue;
        }

        // Detect approval prompts
        if line.contains("Approve? [y/n]") || line.contains("Approve? (y/n)") {
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(
                    line.to_string(),
                    Style::default()
                        .fg(theme::HYDRA_YELLOW)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            i += 1;
            continue;
        }

        // Risk labels
        if line.starts_with("[HIGH RISK]") || line.starts_with("[CRITICAL RISK]") {
            lines.push(Line::from(Span::styled(format!("  {}", line), Style::default().fg(theme::HYDRA_RED).add_modifier(Modifier::BOLD))));
            i += 1; continue;
        }
        if line.starts_with("[MEDIUM RISK]") {
            lines.push(Line::from(Span::styled(format!("  {}", line), Style::default().fg(theme::HYDRA_YELLOW).add_modifier(Modifier::BOLD))));
            i += 1; continue;
        }

        // Box-drawing / tree / file-tree lines
        if line.contains('┌') || line.contains('├') || line.contains('│') || line.contains('─')
            || line.contains("├─") || line.contains("└─") {
            lines.push(Line::from(Span::styled(format!("  {}", line), Style::default().fg(theme::HYDRA_DIM))));
            i += 1; continue;
        }
        if line.contains("📁 ") {
            lines.push(Line::from(Span::styled(format!("  {}", line), Style::default().fg(theme::HYDRA_BLUE))));
            i += 1; continue;
        }

        // Detect search result lines: "path:N:content"
        if role == MessageRole::System && line.trim_start().contains(':') {
            let trimmed = line.trim_start();
            // Check if it looks like "file.rs:42:content"
            let parts: Vec<&str> = trimmed.splitn(3, ':').collect();
            if parts.len() == 3 {
                if let Ok(_line_num) = parts[1].parse::<usize>() {
                    if parts[0].contains('.') {
                        lines.push(Line::from(vec![
                            Span::styled("  ", Style::default()),
                            Span::styled(
                                format!("{}:{}", parts[0], parts[1]),
                                Style::default().fg(theme::HYDRA_BLUE),
                            ),
                            Span::styled(
                                format!(":{}", parts[2]),
                                Style::default(),
                            ),
                        ]));
                        i += 1;
                        continue;
                    }
                }
            }
        }

        // Detect status messages
        if line.contains("completed successfully") || line.contains("failed with exit code") {
            let color = if line.contains("failed") { theme::HYDRA_RED } else { theme::HYDRA_GREEN };
            lines.push(Line::from(Span::styled(format!("  {}", line), Style::default().fg(color))));
            i += 1;
            continue;
        }

        // Default: regular text — strip markdown bold markers
        if line.is_empty() {
            lines.push(Line::default());
        } else {
            let clean = strip_markdown_bold(line);
            let body_style = match role {
                MessageRole::System => Style::default().fg(theme::HYDRA_BLUE),
                _ => Style::default(),
            };
            lines.push(Line::from(Span::styled(
                format!("  {}", clean),
                body_style,
            )));
        }
        i += 1;
    }
}

/// Strip markdown bold markers: "**text**" → "text", "*text*" → "text"
pub fn strip_markdown_bold(s: &str) -> String {
    s.replace("**", "").replace("__", "")
}

/// Phase 3, C5.2: Detect tool result blocks that should be collapsed to a one-liner.
/// Returns Some(summary) if the line starts a collapsible block.
pub fn should_collapse_tool_result(line: &str) -> Option<String> {
    let trimmed = line.trim();

    // Pattern: "Found N references in M files" or similar search results
    if trimmed.contains("Found") && (trimmed.contains("reference") || trimmed.contains("match")) {
        return Some(format!("Searched codebase — {}", trimmed));
    }

    // Pattern: "N files analyzed" or "Scanned N files"
    if (trimmed.contains("files analyzed") || trimmed.contains("Scanned"))
        && trimmed.chars().any(|c| c.is_ascii_digit())
    {
        return Some(format!("Analysis complete — {}", trimmed));
    }

    // Pattern: build/compile success lines
    if (trimmed.contains("Compiling") || trimmed.contains("Finished"))
        && (trimmed.contains("target") || trimmed.contains("release") || trimmed.contains("debug"))
    {
        return Some("Build successful".to_string());
    }

    // Pattern: "Generated blueprint" or similar forge output
    if trimmed.contains("Generated") && trimmed.contains("blueprint") {
        return Some("Blueprint generated".to_string());
    }

    None
}
