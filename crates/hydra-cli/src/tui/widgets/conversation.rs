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
        // Show welcome text in conversation area
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

        // Message content — use terminal default color (works on both light and dark)
        let body_style = match msg.role {
            MessageRole::User => Style::default(),
            MessageRole::Hydra => Style::default(),
            MessageRole::System => Style::default().fg(theme::HYDRA_BLUE),
        };
        for content_line in msg.content.lines() {
            if content_line.is_empty() {
                lines.push(Line::default());
            } else {
                lines.push(Line::from(Span::styled(
                    format!("  {}", content_line),
                    body_style,
                )));
            }
        }

        // Blank line between messages
        lines.push(Line::default());
    }

    // Calculate scroll — show latest messages
    let visible_height = inner.height as usize;
    let total_lines = lines.len();
    let scroll = if total_lines > visible_height {
        total_lines - visible_height
    } else {
        0
    };

    let para = Paragraph::new(lines)
        .scroll((scroll as u16, 0))
        .wrap(Wrap { trim: false });

    frame.render_widget(para, inner);
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

    let lines = vec![
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
        Line::default(),
        boot_line,
        Line::default(),
        Line::from(Span::styled(
            "  Type your request below. Hydra will think, decide, and act.",
            theme::dim(),
        )),
        Line::from(Span::styled(
            "  Type /help for commands, or just describe what you need.",
            theme::dim(),
        )),
    ];

    let para = Paragraph::new(lines);
    frame.render_widget(para, area);
}
