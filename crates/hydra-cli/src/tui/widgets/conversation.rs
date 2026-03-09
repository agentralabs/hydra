use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::tui::app::{App, BootState, FocusArea, MessageRole};
use crate::tui::theme;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let border_style = if app.focus == FocusArea::Conversation {
        theme::border_active()
    } else {
        theme::border()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.messages.is_empty() {
        render_empty_state(frame, app, inner);
        return;
    }

    // Build lines from messages
    let mut lines: Vec<Line> = Vec::new();
    for msg in &app.messages {
        // Message header
        let (prefix, prefix_style) = match msg.role {
            MessageRole::User => ("❯ ", theme::prompt()),
            MessageRole::Hydra => ("◉ ", theme::hydra_msg()),
            MessageRole::System => ("ℹ ", theme::dim()),
        };

        let mut header_spans = vec![
            Span::styled(prefix, prefix_style),
            Span::styled(
                match msg.role {
                    MessageRole::User => "you",
                    MessageRole::Hydra => "hydra",
                    MessageRole::System => "system",
                },
                Style::default()
                    .fg(match msg.role {
                        MessageRole::User => theme::HYDRA_BLUE,
                        MessageRole::Hydra => theme::HYDRA_CYAN,
                        MessageRole::System => theme::HYDRA_DIM,
                    })
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("  {}", msg.timestamp), theme::dim()),
        ];

        if let Some(ref phase) = msg.phase {
            header_spans.push(Span::styled(format!("  [{}]", phase), theme::phase_color(phase)));
        }

        lines.push(Line::from(header_spans));

        // Rich content rendering — detect code blocks, diffs, tool use, command output
        render_rich_content(&msg.content, msg.role.clone(), &mut lines);

        // Blank line between messages
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

    // Show thinking indicator
    if app.is_thinking && app.running_cmd.is_none() {
        let spinner = match (app.tick_count / 2) % 4 {
            0 => "⠋",
            1 => "⠙",
            2 => "⠹",
            _ => "⠸",
        };
        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", spinner), Style::default().fg(theme::HYDRA_CYAN)),
            Span::styled("Thinking...", Style::default().fg(theme::HYDRA_CYAN)),
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

/// Render message content with rich formatting: code blocks, diffs, tool use, etc.
fn render_rich_content(content: &str, role: MessageRole, lines: &mut Vec<Line<'static>>) {
    let content_lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < content_lines.len() {
        let line = content_lines[i];

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

        // Detect tool use lines: "Using Sister: tool(args)"
        if line.starts_with("Using ") && line.contains(": ") {
            lines.push(Line::from(vec![
                Span::styled("  ◉ ", Style::default().fg(theme::HYDRA_CYAN)),
                Span::styled(
                    line.to_string(),
                    Style::default().fg(theme::HYDRA_CYAN),
                ),
            ]));
            i += 1;
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

        // Detect risk labels
        if line.starts_with("[HIGH RISK]") || line.starts_with("[CRITICAL RISK]") {
            lines.push(Line::from(Span::styled(
                format!("  {}", line),
                Style::default().fg(theme::HYDRA_RED).add_modifier(Modifier::BOLD),
            )));
            i += 1;
            continue;
        }
        if line.starts_with("[MEDIUM RISK]") {
            lines.push(Line::from(Span::styled(
                format!("  {}", line),
                Style::default().fg(theme::HYDRA_YELLOW).add_modifier(Modifier::BOLD),
            )));
            i += 1;
            continue;
        }

        // Detect table lines (box-drawing chars)
        if line.contains('┌') || line.contains('├') || line.contains('└')
            || line.contains('│') || line.contains('─')
        {
            lines.push(Line::from(Span::styled(
                format!("  {}", line),
                Style::default().fg(theme::HYDRA_DIM),
            )));
            i += 1;
            continue;
        }

        // Detect tree lines (from /health, /config)
        if line.contains("├─") || line.contains("└─") {
            lines.push(Line::from(Span::styled(
                format!("  {}", line),
                Style::default().fg(theme::HYDRA_DIM),
            )));
            i += 1;
            continue;
        }

        // Detect file tree entries (from /files)
        if line.contains("📁 ") {
            lines.push(Line::from(Span::styled(
                format!("  {}", line),
                Style::default().fg(theme::HYDRA_BLUE),
            )));
            i += 1;
            continue;
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

        // Detect status messages: "completed successfully", "failed with"
        if line.contains("completed successfully") {
            lines.push(Line::from(Span::styled(
                format!("  {}", line),
                Style::default().fg(theme::HYDRA_GREEN),
            )));
            i += 1;
            continue;
        }
        if line.contains("failed with exit code") {
            lines.push(Line::from(Span::styled(
                format!("  {}", line),
                Style::default().fg(theme::HYDRA_RED),
            )));
            i += 1;
            continue;
        }

        // Default: regular text
        if line.is_empty() {
            lines.push(Line::default());
        } else {
            let body_style = match role {
                MessageRole::System => Style::default().fg(theme::HYDRA_BLUE),
                _ => Style::default(),
            };
            lines.push(Line::from(Span::styled(
                format!("  {}", line),
                body_style,
            )));
        }
        i += 1;
    }
}

fn render_empty_state(frame: &mut Frame, app: &App, area: Rect) {
    let version = env!("CARGO_PKG_VERSION");

    let boot_line = match app.boot_state {
        BootState::Booting => Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("⠋ ", Style::default().fg(theme::HYDRA_YELLOW)),
            Span::styled(
                "Spawning sisters...",
                Style::default().fg(theme::HYDRA_YELLOW),
            ),
        ]),
        BootState::Ready => Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                format!("{}/{} sisters · {} tools · ready",
                    app.connected_count, app.total_sisters, app.tool_count),
                theme::status_ok(),
            ),
        ]),
    };

    // Project info lines
    let project_lines: Vec<Line> = if let Some(ref info) = app.project_info {
        let mut pl = vec![
            Line::default(),
            Line::from(vec![
                Span::styled("  Project: ", theme::dim()),
                Span::styled(
                    &info.name,
                    Style::default()
                        .fg(theme::HYDRA_BLUE)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("  Type:    ", theme::dim()),
                Span::styled(
                    format!("{} {}", info.kind.icon(), info.kind.label()),
                    Style::default(),
                ),
                if let Some(count) = info.crate_count {
                    Span::styled(format!(" ({} crates)", count), theme::dim())
                } else {
                    Span::raw("")
                },
            ]),
        ];
        if let Some(ref branch) = info.git_branch {
            let mut git_spans = vec![
                Span::styled("  Git:     ", theme::dim()),
                Span::styled(branch.clone(), Style::default().fg(theme::HYDRA_GREEN)),
            ];
            match (info.git_ahead, info.git_behind) {
                (Some(a), _) if a > 0 => {
                    git_spans.push(Span::styled(format!(" ({} ahead)", a), theme::dim()));
                }
                _ => {}
            }
            pl.push(Line::from(git_spans));
        }
        pl
    } else {
        vec![]
    };

    let mut all_lines = vec![
        Line::default(),
        Line::from(vec![
            Span::styled("  Welcome back, ", theme::dim()),
            Span::styled(
                &app.user_name,
                Style::default()
                    .fg(theme::HYDRA_CYAN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("!", theme::dim()),
        ]),
        Line::default(),
        Line::from(Span::styled(
            "          ◉",
            Style::default().fg(theme::HYDRA_CYAN),
        )),
        Line::from(Span::styled(
            "        ╱   ╲",
            Style::default().fg(theme::HYDRA_BLUE),
        )),
        Line::from(Span::styled(
            "       ◉─────◉",
            Style::default().fg(theme::HYDRA_BLUE),
        )),
        Line::from(Span::styled(
            "        ╲   ╱",
            Style::default().fg(theme::HYDRA_BLUE),
        )),
        Line::from(Span::styled(
            "          ◉",
            Style::default().fg(theme::HYDRA_CYAN),
        )),
        Line::default(),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                &app.model_name,
                Style::default().fg(theme::HYDRA_PURPLE),
            ),
            Span::styled(" · ", theme::dim()),
            Span::styled(
                format!("v{}", version),
                theme::dim(),
            ),
        ]),
        Line::from(Span::styled(
            format!("  {}", app.working_dir),
            theme::dim(),
        )),
    ];

    all_lines.extend(project_lines);

    all_lines.push(Line::default());
    all_lines.push(boot_line);
    all_lines.push(Line::default());
    all_lines.push(Line::from(Span::styled(
        "  Type your request below, or use /files, /test, /build, /search.",
        theme::dim(),
    )));
    all_lines.push(Line::from(Span::styled(
        "  Type /help for all commands. 14 sisters at your service.",
        theme::dim(),
    )));

    let para = Paragraph::new(all_lines);
    frame.render_widget(para, area);
}
