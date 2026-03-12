use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::tui::app::{App, BootState};
use crate::tui::theme;

/// Render the welcome screen — shown when no messages exist.
/// Matches Claude Code's clean centered layout.
pub fn render_empty_state(frame: &mut Frame, app: &App, area: Rect) {
    let version = env!("CARGO_PKG_VERSION");

    let boot_status = match app.boot_state {
        BootState::Booting => Line::from(vec![
            Span::styled("  ⠋ ", Style::default().fg(theme::HYDRA_YELLOW)),
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

    // Build left+right column lines (Claude Code style welcome)
    let pad_r = "                ";
    let mut all_lines = vec![
        Line::default(),
        Line::from(vec![
            Span::styled("        Welcome back ", theme::dim()),
            Span::styled(
                &app.user_name,
                Style::default()
                    .fg(theme::HYDRA_CYAN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("!", theme::dim()),
            Span::styled(pad_r, Style::default()),
            Span::styled("Tips for getting started", Style::default().fg(theme::HYDRA_BLUE).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::raw("                                                "),
            Span::styled("Run /init to create a HYDRA.md", theme::dim()),
        ]),
        Line::from(vec![
            Span::raw("                                                "),
            Span::styled("file with project instructions.", theme::dim()),
        ]),
        Line::from(vec![
            Span::raw("                                                "),
            Span::styled("──────────────────────────────", theme::dim()),
        ]),
        Line::from(vec![
            Span::raw("                                                "),
            Span::styled("Recent activity", Style::default().fg(theme::HYDRA_BLUE).add_modifier(Modifier::BOLD)),
        ]),
    ];
    // Show recent tasks or placeholder
    if app.recent_tasks.is_empty() {
        all_lines.push(Line::from(vec![
            Span::raw("                                                  "),
            Span::styled("No recent activity", theme::dim()),
        ]));
    } else {
        for task in app.recent_tasks.iter().take(3) {
            all_lines.push(Line::from(vec![
                Span::raw("                                                  "),
                Span::styled(format!("  {}", task.summary), theme::dim()),
            ]));
        }
    }
    all_lines.push(Line::default());
    all_lines.push(Line::from(Span::styled("                ◉", Style::default().fg(theme::HYDRA_CYAN))));
    all_lines.push(Line::from(Span::styled("              ╱   ╲", Style::default().fg(theme::HYDRA_BLUE))));
    all_lines.push(Line::from(Span::styled("             ◉─────◉", Style::default().fg(theme::HYDRA_BLUE))));
    all_lines.push(Line::from(Span::styled("              ╲   ╱", Style::default().fg(theme::HYDRA_BLUE))));
    all_lines.push(Line::from(Span::styled("                ◉", Style::default().fg(theme::HYDRA_CYAN))));
    all_lines.push(Line::default());
    all_lines.push(Line::from(vec![
        Span::styled("        ", Style::default()),
        Span::styled(&app.model_name, Style::default().fg(theme::HYDRA_PURPLE)),
        Span::styled(" · ", theme::dim()),
        Span::styled(format!("v{}", version), theme::dim()),
    ]));
    all_lines.push(Line::from(Span::styled(
        format!("        {}", app.working_dir), theme::dim(),
    )));

    // Project info
    if let Some(ref info) = app.project_info {
        all_lines.push(Line::default());
        all_lines.push(Line::from(vec![
            Span::styled("        ", Style::default()),
            Span::styled(
                format!("{} {}", info.kind.icon(), info.name),
                Style::default()
                    .fg(theme::HYDRA_BLUE)
                    .add_modifier(Modifier::BOLD),
            ),
            if let Some(count) = info.crate_count {
                Span::styled(format!(" ({} crates)", count), theme::dim())
            } else {
                Span::raw("")
            },
        ]));
        if let Some(ref branch) = info.git_branch {
            all_lines.push(Line::from(vec![
                Span::styled("        Git: ", theme::dim()),
                Span::styled(branch.clone(), Style::default().fg(theme::HYDRA_GREEN)),
            ]));
        }
    }

    all_lines.push(Line::default());
    all_lines.push(boot_status);
    all_lines.push(Line::default());

    let para = Paragraph::new(all_lines);
    frame.render_widget(para, area);
}
