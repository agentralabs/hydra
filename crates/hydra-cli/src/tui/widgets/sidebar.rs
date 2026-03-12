use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

use crate::tui::app::{App, FocusArea, TaskStatus};
use crate::tui::theme;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let border_style = if app.focus == FocusArea::Sidebar {
        theme::border_active()
    } else {
        theme::border()
    };

    // NO BORDERS — Visual Overhaul Rule 1. Clean padding only.
    let _ = border_style; // suppress unused warning
    let block = Block::default()
        .title(Span::styled(
            "Status",
            Style::default()
                .fg(theme::HYDRA_BLUE)
                .add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split sidebar into sections
    let has_project = app.project_info.is_some();
    let project_height = if has_project { 5 } else { 0 };

    let chunks = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(project_height), // Project info
            Constraint::Length(4),  // Sisters summary
            Constraint::Length(8),  // Metrics
            Constraint::Length(1),  // Separator
            Constraint::Min(4),    // Recent tasks
            Constraint::Length(2), // Phase indicator
        ])
        .split(inner);

    if has_project {
        render_project_info(frame, app, chunks[0]);
    }
    render_sisters_summary(frame, app, chunks[1]);
    render_metrics(frame, app, chunks[2]);
    render_separator(frame, chunks[3]);
    render_recent_tasks(frame, app, chunks[4]);
    render_phase(frame, app, chunks[5]);
}

fn render_project_info(frame: &mut Frame, app: &App, area: Rect) {
    if let Some(ref info) = app.project_info {
        let mut lines = vec![
            Line::from(vec![
                Span::styled("Project  ", theme::sidebar_label()),
                Span::styled(
                    &info.name,
                    Style::default()
                        .fg(theme::HYDRA_BLUE)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Type     ", theme::sidebar_label()),
                Span::styled(
                    format!("{} {}", info.kind.icon(), info.kind.label()),
                    theme::sidebar_value(),
                ),
                if let Some(count) = info.crate_count {
                    Span::styled(format!(" ({})", count), theme::dim())
                } else {
                    Span::raw("")
                },
            ]),
        ];

        if let Some(ref branch) = info.git_branch {
            let mut git_spans = vec![
                Span::styled("Git      ", theme::sidebar_label()),
                Span::styled(branch.clone(), Style::default().fg(theme::HYDRA_GREEN)),
            ];
            match (info.git_ahead, info.git_behind) {
                (Some(a), _) if a > 0 => {
                    git_spans.push(Span::styled(format!(" +{}", a), theme::dim()));
                }
                _ => {}
            }
            lines.push(Line::from(git_spans));
        }

        // Blank line separator — NO horizontal rules (Visual Overhaul Rule 3)
        lines.push(Line::default());

        let para = Paragraph::new(lines);
        frame.render_widget(para, area);
    }
}

fn render_sisters_summary(frame: &mut Frame, app: &App, area: Rect) {
    let sisters_line = Line::from(vec![
        Span::styled("Sisters  ", theme::sidebar_label()),
        Span::styled(
            format!("{}/{}", app.connected_count, app.total_sisters),
            if app.connected_count == app.total_sisters {
                theme::status_ok()
            } else if app.connected_count > 0 {
                theme::status_warn()
            } else {
                theme::status_err()
            },
        ),
    ]);

    let all_status = if app.connected_count == app.total_sisters {
        Line::from(Span::styled("         All Online", theme::status_ok()))
    } else if app.connected_count > 0 {
        Line::from(Span::styled(
            format!(
                "         {} offline",
                app.total_sisters - app.connected_count
            ),
            theme::status_warn(),
        ))
    } else {
        Line::from(Span::styled("         All Offline", theme::status_err()))
    };

    // Sister dots - compact grid
    let mut dot_spans: Vec<Span> = vec![Span::raw("         ")];
    for (i, s) in app.sisters.iter().enumerate() {
        let dot = if s.connected {
            Span::styled("●", theme::status_ok())
        } else {
            Span::styled("○", theme::dim())
        };
        dot_spans.push(dot);
        if i < app.sisters.len() - 1 {
            dot_spans.push(Span::raw(" "));
        }
    }
    let dots_line = Line::from(dot_spans);

    let text = vec![sisters_line, all_status, dots_line];
    let para = Paragraph::new(text);
    frame.render_widget(para, area);
}

fn render_metrics(frame: &mut Frame, app: &App, area: Rect) {
    let health_style = if app.health_pct >= 90 {
        theme::status_ok()
    } else if app.health_pct >= 50 {
        theme::status_warn()
    } else {
        theme::status_err()
    };

    let lines = vec![
        Line::from(vec![
            Span::styled("Health   ", theme::sidebar_label()),
            Span::styled(format!("{}%", app.health_pct), health_style),
        ]),
        Line::from(vec![
            Span::styled("Trust    ", theme::sidebar_label()),
            Span::styled(&app.trust_level, theme::sidebar_value()),
        ]),
        Line::from(vec![
            Span::styled("Memory   ", theme::sidebar_label()),
            Span::styled(
                format!("{} facts", app.memory_facts),
                theme::sidebar_value(),
            ),
        ]),
        Line::from(vec![
            Span::styled("Tokens   ", theme::sidebar_label()),
            Span::styled(
                if app.token_avg > 0 {
                    format!("~{} avg", app.token_avg)
                } else {
                    "—".to_string()
                },
                theme::sidebar_value(),
            ),
        ]),
        Line::from(vec![
            Span::styled("Receipts ", theme::sidebar_label()),
            Span::styled(format!("{}", app.receipt_count), theme::sidebar_value()),
        ]),
        Line::default(),
        Line::from(vec![
            Span::styled("Mode     ", theme::sidebar_label()),
            if app.sisters_handle.is_some() {
                Span::styled("● Local", theme::status_ok())
            } else if app.server_online {
                Span::styled("● Server", theme::status_ok())
            } else {
                Span::styled("○ Offline", theme::status_err())
            },
        ]),
    ];

    let para = Paragraph::new(lines);
    frame.render_widget(para, area);
}

fn render_separator(frame: &mut Frame, area: Rect) {
    // Blank line separator — NO horizontal rules (Visual Overhaul Rule 3)
    let para = Paragraph::new(Line::default());
    frame.render_widget(para, area);
}

fn render_recent_tasks(frame: &mut Frame, app: &App, area: Rect) {
    let mut lines = vec![Line::from(Span::styled(
        "Recent",
        Style::default()
            .fg(theme::HYDRA_DIM)
            .add_modifier(Modifier::BOLD),
    ))];

    if app.recent_tasks.is_empty() {
        lines.push(Line::from(Span::styled("  No recent tasks", theme::dim())));
    } else {
        for task in app.recent_tasks.iter().take(area.height.saturating_sub(1) as usize) {
            let icon = match task.status {
                TaskStatus::Complete => Span::styled("✓ ", theme::status_ok()),
                TaskStatus::Running => Span::styled("● ", theme::status_warn()),
                TaskStatus::Failed => Span::styled("✗ ", theme::status_err()),
            };
            let max_len = area.width.saturating_sub(4) as usize;
            let char_count = task.summary.chars().count();
            let summary = if char_count > max_len {
                let truncated: String = task.summary.chars().take(max_len.saturating_sub(1)).collect();
                format!("{}…", truncated)
            } else {
                task.summary.clone()
            };
            lines.push(Line::from(vec![
                Span::raw("  "),
                icon,
                Span::styled(summary, theme::sidebar_value()),
            ]));
        }
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, area);
}

fn render_phase(frame: &mut Frame, app: &App, area: Rect) {
    let line = if let Some(ref phase) = app.current_phase {
        let spinner = match (app.tick_count / 2) % 4 {
            0 => "⠋",
            1 => "⠙",
            2 => "⠹",
            3 => "⠸",
            _ => "⠋",
        };
        Line::from(vec![
            Span::styled(format!("{} ", spinner), theme::phase_color(phase)),
            Span::styled(phase, theme::phase_color(phase)),
        ])
    } else {
        Line::from(Span::styled("Idle", theme::dim()))
    };

    let para = Paragraph::new(line);
    frame.render_widget(para, area);
}
