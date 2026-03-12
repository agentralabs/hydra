use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use crate::tui::app::App;
use crate::tui::theme;

const FC: ratatui::style::Color = theme::HYDRA_BLUE;

/// Build a row: │ left_content (padded) │ right_content (padded) │
fn row(left: Vec<Span<'static>>, right: Vec<Span<'static>>,
       w: usize, sp: usize) -> Line<'static> {
    let bs = Style::default().fg(FC);
    let ll: usize = left.iter().map(|s| s.content.chars().count()).sum();
    let rl: usize = right.iter().map(|s| s.content.chars().count()).sum();
    let lpad = sp.saturating_sub(ll + 1);
    let rpad = w.saturating_sub(sp + rl + 2);
    let mut v = vec![Span::styled("│", bs)];
    v.extend(left);
    v.push(Span::raw(" ".repeat(lpad)));
    v.push(Span::styled("│", bs));
    v.extend(right);
    v.push(Span::raw(" ".repeat(rpad)));
    v.push(Span::styled("│", bs));
    Line::from(v)
}

/// Build the framed welcome screen and append lines to `out`.
pub fn build_welcome_frame(app: &App, width: usize, out: &mut Vec<Line<'static>>) {
    let version = env!("CARGO_PKG_VERSION");
    let w = width;
    // Left column gets 45%, right gets 55% — room for metrics
    let sp = w * 45 / 100;

    let bs = Style::default().fg(FC);
    let bb = Style::default().fg(FC).add_modifier(Modifier::BOLD);
    let cb = Style::default().fg(theme::HYDRA_CYAN).add_modifier(Modifier::BOLD);
    let dim = theme::dim();
    let lb = Style::default().fg(theme::HYDRA_BLUE);
    let lc = Style::default().fg(theme::HYDRA_CYAN);
    let green = Style::default().fg(theme::HYDRA_GREEN);
    let yellow = Style::default().fg(theme::HYDRA_YELLOW);
    let purple = Style::default().fg(theme::HYDRA_PURPLE);

    // ┌─── Hydra v0.1.0 ──────────────────────────┐
    let title = format!(" Hydra v{} ", version);
    let inner_w = w.saturating_sub(2); // minus ┌ and ┐
    let ld = 3usize;
    let rd = inner_w.saturating_sub(ld + title.len());
    out.push(Line::from(vec![
        Span::styled("┌", bs),
        Span::styled("─".repeat(ld), bs),
        Span::styled(title, bb),
        Span::styled("─".repeat(rd), bs),
        Span::styled("┐", bs),
    ]));

    // Empty row
    out.push(row(vec![], vec![], w, sp));

    // Welcome | Tips header
    out.push(row(
        vec![
            Span::styled("      Welcome back ", dim),
            Span::styled(app.user_name.clone(), cb),
            Span::styled("!", dim),
        ],
        vec![Span::styled(" Tips for getting started", bb)],
        w, sp,
    ));

    // (empty) | tip text
    out.push(row(vec![], vec![
        Span::styled(" Run /init to create a HYDRA.md file with", dim),
    ], w, sp));
    out.push(row(vec![], vec![
        Span::styled(" instructions for Hydra.", dim),
    ], w, sp));

    // Logo ◉ | separator
    let sep_w = w.saturating_sub(sp + 4).min(45);
    out.push(row(
        vec![Span::styled("           ◉", lc)],
        vec![Span::styled(format!(" {}", "─".repeat(sep_w)), dim)],
        w, sp,
    ));

    // Logo ╱╲ | Recent activity
    out.push(row(
        vec![Span::styled("         ╱   ╲", lb)],
        vec![Span::styled(" Recent activity", bb)],
        w, sp,
    ));

    // Logo ◉──◉ | activity items
    let act1 = if app.recent_tasks.is_empty() {
        Span::styled(" No recent activity", dim)
    } else {
        Span::styled(format!("   {}", app.recent_tasks[0].summary), dim)
    };
    out.push(row(
        vec![Span::styled("        ◉─────◉", lb)],
        vec![act1], w, sp,
    ));

    // Logo ╲╱ | activity 2
    let act2 = if app.recent_tasks.len() > 1 {
        Span::styled(format!("   {}", app.recent_tasks[1].summary), dim)
    } else { Span::raw("") };
    out.push(row(
        vec![Span::styled("         ╲   ╱", lb)],
        vec![act2], w, sp,
    ));

    // Logo ◉ | separator
    out.push(row(
        vec![Span::styled("           ◉", lc)],
        vec![Span::styled(format!(" {}", "─".repeat(sep_w)), dim)],
        w, sp,
    ));

    // Empty | System info header
    out.push(row(vec![], vec![
        Span::styled(" System", bb),
    ], w, sp));

    // Model + Org | Sisters
    let sister_style = if app.connected_count == app.total_sisters { green } else { yellow };
    out.push(row(
        vec![
            Span::styled("  ", Style::default()),
            Span::styled(app.model_name.clone(), purple),
            Span::styled(" · ", dim),
            Span::styled("Agentra Labs", dim),
        ],
        vec![
            Span::styled(" Sisters    ", dim),
            Span::styled(format!("{}/{} connected", app.connected_count, app.total_sisters), sister_style),
        ],
        w, sp,
    ));

    // Working dir | Tools
    let short_dir = shorten_path(&app.working_dir, sp.saturating_sub(5));
    out.push(row(
        vec![Span::styled(format!("  {}", short_dir), dim)],
        vec![
            Span::styled(" Tools      ", dim),
            Span::styled(format!("{}+", app.tool_count), dim),
        ],
        w, sp,
    ));

    // Project info | Health
    let health_style = if app.health_pct >= 90 { green }
        else if app.health_pct >= 50 { yellow }
        else { Style::default().fg(theme::HYDRA_RED) };
    if let Some(ref info) = app.project_info {
        let proj = if let Some(count) = info.crate_count {
            format!("  {} ({} crates)", info.name, count)
        } else {
            format!("  {}", info.name)
        };
        out.push(row(
            vec![Span::styled(proj, bb)],
            vec![
                Span::styled(" Health     ", dim),
                Span::styled(format!("{}%", app.health_pct), health_style),
            ],
            w, sp,
        ));
    } else {
        out.push(row(vec![], vec![
            Span::styled(" Health     ", dim),
            Span::styled(format!("{}%", app.health_pct), health_style),
        ], w, sp));
    }

    // Mode row
    let (status_dot, status_label) = if app.sisters_handle.is_some() && app.connected_count > 0 {
        ("●", "Local")
    } else if app.server_online {
        ("●", "Server")
    } else {
        ("●", "Offline")
    };
    let dot_style = if status_label == "Offline" {
        Style::default().fg(theme::HYDRA_RED)
    } else { green };

    out.push(row(
        vec![],
        vec![
            Span::styled(" Mode       ", dim),
            Span::styled(status_dot, dot_style),
            Span::styled(format!(" {}", status_label), dim),
        ],
        w, sp,
    ));

    // Empty row
    out.push(row(vec![], vec![], w, sp));

    // Bottom border
    let dashes = w.saturating_sub(2);
    out.push(Line::from(vec![
        Span::styled("└", bs),
        Span::styled("─".repeat(dashes), bs),
        Span::styled("┘", bs),
    ]));
}

fn shorten_path(path: &str, max: usize) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let s = if !home.is_empty() && path.starts_with(&home) {
        format!("~{}", &path[home.len()..])
    } else { path.to_string() };
    if s.len() <= max { s } else {
        format!("...{}", &s[s.len().saturating_sub(max.saturating_sub(3))..])
    }
}
