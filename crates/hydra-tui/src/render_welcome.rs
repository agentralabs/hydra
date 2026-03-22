//! Welcome screen renderer — themed version.
//!
//! Uses theme::current() for all chrome colors.
//! Brand colors (AMBER, GOLD, etc.) come from the theme's accent fields.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, BorderType, Paragraph};
use ratatui::Frame;

use crate::theme;

// Brand colors that stay the same in both themes
const GOLD: Color = Color::Rgb(212, 170, 110);
const GOLD_BRIGHT: Color = Color::Rgb(232, 200, 122);
const CYAN: Color = Color::Rgb(106, 184, 212);

/// Welcome screen data collected at boot.
pub struct WelcomeData {
    pub lyapunov: f64,
    pub growth_rate: f64,
    pub morphic_depth: u64,
    pub genome_entries: usize,
    pub step_count: u64,
    pub version: String,
    pub beliefs_loaded: usize,
    pub skills_active: usize,
    pub antifragile_classes: usize,
    pub systems_mapped: usize,
    pub username: String,
}

impl Default for WelcomeData {
    fn default() -> Self {
        Self {
            lyapunov: 1.0,
            growth_rate: 0.003,
            morphic_depth: 0,
            genome_entries: 0,
            step_count: 0,
            version: "0.1.0".into(),
            beliefs_loaded: 0,
            skills_active: 0,
            antifragile_classes: 0,
            systems_mapped: 0,
            username: "operator".into(),
        }
    }
}

/// Render the full welcome screen.
pub fn render(f: &mut Frame, area: Rect, data: &WelcomeData) {
    let t = theme::current();

    if area.width < 80 || area.height < 24 {
        let msg = Paragraph::new("◈ HYDRA — Terminal too small (min 80×24)")
            .style(Style::default().fg(t.accent).bg(t.bg_primary));
        f.render_widget(msg, area);
        return;
    }

    let bg_block = Block::default().style(Style::default().bg(t.bg_primary));
    f.render_widget(bg_block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Min(12),
            Constraint::Length(1),
            Constraint::Min(8),
            Constraint::Length(1),
            Constraint::Length(2),
        ])
        .split(area);

    render_top_frame(f, chunks[0], data, &t);
    render_bottom_frame(f, chunks[2], data, &t);
    render_input_row(f, chunks[4], &t);
}

fn render_top_frame(f: &mut Frame, area: Rect, data: &WelcomeData, t: &theme::Theme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.frame_top))
        .style(Style::default().bg(t.bg_primary));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    render_top_left(f, cols[0], data, t);
    render_top_right(f, cols[1], data, t);
}

fn render_top_left(f: &mut Frame, area: Rect, data: &WelcomeData, t: &theme::Theme) {
    let bright = t.fg_primary;
    let dim = t.fg_secondary;
    let label_amber = t.label;

    let g1 = GOLD;
    let g2 = GOLD_BRIGHT;
    let gr = Color::Rgb(122, 200, 122);

    let mut lines = vec![
        // Diamond logo — 5 lines, matches HTML SVG diamond shape
        Line::from(vec![
            Span::styled("        ◆        ", Style::default().fg(g1)),
        ]),
        Line::from(vec![
            Span::styled("      ◇", Style::default().fg(g1)),
            Span::styled(" ◈ ", Style::default().fg(t.accent)),
            Span::styled("◇      ", Style::default().fg(gr)),
        ]),
        Line::from(vec![
            Span::styled("    ◇", Style::default().fg(g1)),
            Span::styled("  H Y D R A  ", Style::default().fg(g2)),
            Span::styled("◇    ", Style::default().fg(CYAN)),
        ]),
        Line::from(vec![
            Span::styled("      ◇", Style::default().fg(gr)),
            Span::styled("     ", Style::default().fg(t.accent)),
            Span::styled("◇      ", Style::default().fg(CYAN)),
        ]),
        Line::from(vec![
            Span::styled("        ◆        ", Style::default().fg(CYAN)),
        ]),
        // Status sub-row
        Line::from(vec![
            Span::styled("     ● ", Style::default().fg(t.alive)),
            Span::styled("alive", Style::default().fg(t.alive)),
            Span::styled(" │ ", Style::default().fg(dim)),
            Span::styled(format!("v{}", data.version), Style::default().fg(dim)),
            Span::styled(" │ ", Style::default().fg(dim)),
            Span::styled(
                format!("step {}", format_number(data.step_count)),
                Style::default().fg(dim),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            format!("  Good {}, {}.", time_of_day(), data.username),
            Style::default().fg(t.greeting),
        )),
        Line::from(""),
        Line::from(Span::styled("  COGNITIVE STATE", Style::default().fg(label_amber))),
        kv_line("  beliefs loaded", &data.beliefs_loaded.to_string(), t.accent, t),
        kv_line("  skills active", &data.skills_active.to_string(), bright, t),
        kv_line("  persona", "core", t.success, t),
        kv_line(
            "  antifragile",
            &format!("{} obstacle classes", data.antifragile_classes),
            CYAN,
            t,
        ),
        kv_line(
            "  cartography",
            &format!("{} systems mapped", data.systems_mapped),
            CYAN,
            t,
        ),
    ];
    lines.truncate(area.height as usize);

    let para = Paragraph::new(lines).style(Style::default().bg(t.bg_primary));
    f.render_widget(para, area);
}

fn render_top_right(f: &mut Frame, area: Rect, data: &WelcomeData, t: &theme::Theme) {
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "unknown".into());
    let short_cwd = if cwd.len() > 30 {
        format!("...{}", &cwd[cwd.len() - 27..])
    } else {
        cwd
    };

    let mut lines = vec![
        Line::from(Span::styled("  WORKING CONTEXT", Style::default().fg(t.label))),
        kv_line("  project", &short_cwd, t.accent, t),
        kv_line("  branch", "main · clean", t.fg_primary, t),
        kv_line("  phase", &format!("{} verified ✓", data.step_count), t.success, t),
        kv_line("  model", "claude-sonnet-4-20250514", t.fg_primary, t),
        Line::from(""),
        Line::from(Span::styled("  RECENT ACTIVITY", Style::default().fg(t.fg_muted))),
        activity_line("✓", t.alive, "kernel boot complete", t.fg_primary),
        activity_line("✓", t.alive, "constitution verified (7 laws)", t.fg_primary),
        activity_line(
            "◑",
            t.accent,
            &format!("genome: {} entries", data.genome_entries),
            t.fg_primary,
        ),
    ];
    lines.truncate(area.height as usize);

    let para = Paragraph::new(lines).style(Style::default().bg(t.bg_primary));
    f.render_widget(para, area);
}

fn render_bottom_frame(f: &mut Frame, area: Rect, data: &WelcomeData, t: &theme::Theme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.frame_bottom))
        .style(Style::default().bg(t.bg_primary));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    render_bottom_left(f, cols[0], t);
    render_bottom_right(f, cols[1], data, t);
}

fn render_bottom_left(f: &mut Frame, area: Rect, t: &theme::Theme) {
    let mut lines = vec![
        Line::from(Span::styled(
            "  WHILE YOU WERE AWAY",
            Style::default().fg(t.label_green),
        )),
        Line::from(vec![
            Span::styled("  ○ ", Style::default().fg(t.fg_muted)),
            Span::styled("No pending briefings", Style::default().fg(t.fg_secondary)),
        ]),
    ];
    lines.truncate(area.height as usize);

    let para = Paragraph::new(lines).style(Style::default().bg(t.bg_primary));
    f.render_widget(para, area);
}

fn render_bottom_right(f: &mut Frame, area: Rect, data: &WelcomeData, t: &theme::Theme) {
    let green_dim = Color::Rgb(45, 94, 58);

    let mut lines = vec![
        Line::from(Span::styled("  ENTITY HEALTH", Style::default().fg(green_dim))),
        Line::from(vec![
            Span::styled("  V(Ψ) ", Style::default().fg(t.fg_muted)),
            Span::styled(format!("{:+.2}", data.lyapunov), Style::default().fg(t.success)),
            Span::styled("  stable", Style::default().fg(t.lyapunov_sub)),
        ]),
        Line::from(vec![
            Span::styled("  Γ̂(Ψ) ", Style::default().fg(t.fg_muted)),
            Span::styled(format!("{:+.3}", data.growth_rate), Style::default().fg(t.success)),
            Span::styled("  growing", Style::default().fg(t.lyapunov_sub)),
        ]),
        Line::from(vec![
            Span::styled("  depth ", Style::default().fg(t.fg_muted)),
            Span::styled(format_number(data.morphic_depth), Style::default().fg(t.identity_val)),
            Span::styled("  morphic events", Style::default().fg(t.identity_sub)),
        ]),
        Line::from(vec![
            Span::styled("  genome ", Style::default().fg(t.fg_muted)),
            Span::styled(
                format_number(data.genome_entries as u64),
                Style::default().fg(t.identity_val),
            ),
            Span::styled("  permanent entries", Style::default().fg(t.identity_sub)),
        ]),
    ];
    lines.truncate(area.height as usize);

    let para = Paragraph::new(lines).style(Style::default().bg(t.bg_primary));
    f.render_widget(para, area);
}

fn render_input_row(f: &mut Frame, area: Rect, t: &theme::Theme) {
    let lines = vec![
        Line::from(vec![
            Span::styled("  ◈  ", Style::default().fg(t.accent)),
            Span::styled("what are we building today?", Style::default().fg(t.fg_muted)),
            Span::styled("█", Style::default().fg(t.accent)),
        ]),
        Line::from(Span::styled(
            "  Ctrl+V voice  ·  /dream  ·  /digest  ·  /help",
            Style::default().fg(t.fg_ghost),
        )),
    ];
    let para = Paragraph::new(lines).style(Style::default().bg(t.bg_primary));
    f.render_widget(para, area);
}

// Helpers
fn kv_line(key: &str, value: &str, val_color: Color, t: &theme::Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{key:<20}"), Style::default().fg(t.fg_muted)),
        Span::styled(value.to_string(), Style::default().fg(val_color)),
    ])
}

fn activity_line(sym: &str, sym_color: Color, text: &str, text_color: Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {sym} "), Style::default().fg(sym_color)),
        Span::styled(text.to_string(), Style::default().fg(text_color)),
    ])
}

fn time_of_day() -> &'static str {
    let hour = chrono::Local::now().hour();
    match hour {
        5..=11 => "morning",
        12..=16 => "afternoon",
        _ => "evening",
    }
}

fn format_number(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{},{:03}", n / 1_000, n % 1_000)
    } else {
        n.to_string()
    }
}

use chrono::Timelike;
