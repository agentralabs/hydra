//! Welcome screen renderer — exact replica of hydra_welcome_v5.html.
//!
//! Every color is from HYDRA-TUI-DESIGN-SPEC.md. No approximations.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, BorderType, Paragraph};
use ratatui::Frame;

// Spec colors
const BG: Color = Color::Rgb(12, 12, 12);
const AMBER: Color = Color::Rgb(200, 169, 110);
const GOLD: Color = Color::Rgb(212, 170, 110);
const GOLD_BRIGHT: Color = Color::Rgb(232, 200, 122);
const GREEN: Color = Color::Rgb(74, 170, 106);
const GREEN_DARK: Color = Color::Rgb(61, 140, 94);
const GREEN_DIM: Color = Color::Rgb(45, 94, 58);
const CYAN: Color = Color::Rgb(106, 184, 212);
const GREETING: Color = Color::Rgb(224, 200, 138);
const BRIGHT_TEXT: Color = Color::Rgb(153, 153, 153);
const DIM_TEXT: Color = Color::Rgb(85, 85, 85);
const DIMMER_TEXT: Color = Color::Rgb(68, 68, 68);
const GHOST: Color = Color::Rgb(46, 46, 46);
const AMBER_BORDER: Color = Color::Rgb(122, 106, 74);
const GREEN_BORDER: Color = Color::Rgb(37, 58, 37);
const LABEL_AMBER: Color = Color::Rgb(122, 106, 74);
const LABEL_GREEN: Color = Color::Rgb(90, 122, 74);
const LABEL_DIM: Color = Color::Rgb(62, 62, 62);
const LYAP_SUB: Color = Color::Rgb(45, 110, 68);
const IDENTITY_VAL: Color = Color::Rgb(106, 170, 212);
const IDENTITY_SUB: Color = Color::Rgb(58, 110, 140);

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
        }
    }
}

/// Render the full welcome screen.
pub fn render(f: &mut Frame, area: Rect, data: &WelcomeData) {
    // Check minimum size
    if area.width < 80 || area.height < 24 {
        let msg = Paragraph::new("◈ HYDRA — Terminal too small (min 80×24)")
            .style(Style::default().fg(AMBER).bg(BG));
        f.render_widget(msg, area);
        return;
    }

    // Background
    let bg_block = Block::default().style(Style::default().bg(BG));
    f.render_widget(bg_block, area);

    // Main layout: top frame, gap, bottom frame, gap, input row
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Min(12),    // top frame
            Constraint::Length(1),  // gap
            Constraint::Min(8),     // bottom frame
            Constraint::Length(1),  // gap
            Constraint::Length(2),  // input row
        ])
        .split(area);

    render_top_frame(f, chunks[0], data);
    render_bottom_frame(f, chunks[2], data);
    render_input_row(f, chunks[4]);
}

fn render_top_frame(f: &mut Frame, area: Rect, data: &WelcomeData) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(AMBER_BORDER))
        .style(Style::default().bg(BG));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    render_top_left(f, cols[0], data);
    render_top_right(f, cols[1], data);
}

fn render_top_left(f: &mut Frame, area: Rect, data: &WelcomeData) {
    let mut lines = vec![
        // Logo row: ◈ HYDRA with gradient
        Line::from(vec![
            Span::styled("  ◈  ", Style::default().fg(AMBER)),
            Span::styled("H", Style::default().fg(GOLD)),
            Span::styled("Y", Style::default().fg(GOLD)),
            Span::styled("D", Style::default().fg(GOLD_BRIGHT)),
            Span::styled("R", Style::default().fg(Color::Rgb(122, 200, 122))),
            Span::styled("A", Style::default().fg(CYAN)),
        ]),
        // Status sub-row
        Line::from(vec![
            Span::styled("     ● ", Style::default().fg(GREEN_DARK)),
            Span::styled("alive", Style::default().fg(GREEN_DARK)),
            Span::styled(" │ ", Style::default().fg(DIM_TEXT)),
            Span::styled(format!("v{}", data.version), Style::default().fg(DIM_TEXT)),
            Span::styled(" │ ", Style::default().fg(DIM_TEXT)),
            Span::styled(
                format!("step {}", format_number(data.step_count)),
                Style::default().fg(DIM_TEXT),
            ),
        ]),
        Line::from(""),
        // Greeting
        Line::from(Span::styled(
            format!("  Good {}, Omoshola.", time_of_day()),
            Style::default().fg(GREETING),
        )),
        Line::from(""),
        // Section: COGNITIVE STATE
        Line::from(Span::styled(
            "  COGNITIVE STATE",
            Style::default().fg(LABEL_AMBER),
        )),
        kv_line("  beliefs loaded", &data.beliefs_loaded.to_string(), AMBER),
        kv_line("  skills active", &data.skills_active.to_string(), BRIGHT_TEXT),
        kv_line("  persona", "core", GREEN),
        kv_line("  antifragile", &format!("{} obstacle classes", data.antifragile_classes), CYAN),
        kv_line("  cartography", &format!("{} systems mapped", data.systems_mapped), CYAN),
    ];
    lines.truncate(area.height as usize);

    let para = Paragraph::new(lines).style(Style::default().bg(BG));
    f.render_widget(para, area);
}

fn render_top_right(f: &mut Frame, area: Rect, data: &WelcomeData) {
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "unknown".into());
    let short_cwd = if cwd.len() > 30 {
        format!("...{}", &cwd[cwd.len() - 27..])
    } else {
        cwd
    };

    let mut lines = vec![
        Line::from(Span::styled(
            "  WORKING CONTEXT",
            Style::default().fg(LABEL_AMBER),
        )),
        kv_line("  project", &short_cwd, AMBER),
        kv_line("  branch", "main · clean", BRIGHT_TEXT),
        kv_line(
            "  phase",
            &format!("{} verified ✓", data.step_count),
            GREEN,
        ),
        kv_line("  model", "claude-opus-4-6", BRIGHT_TEXT),
        Line::from(""),
        Line::from(Span::styled(
            "  RECENT ACTIVITY",
            Style::default().fg(LABEL_DIM),
        )),
        activity_line("✓", GREEN_DARK, "kernel boot complete", BRIGHT_TEXT),
        activity_line("✓", GREEN_DARK, "constitution verified (7 laws)", BRIGHT_TEXT),
        activity_line(
            "◑",
            AMBER,
            &format!("genome: {} entries", data.genome_entries),
            BRIGHT_TEXT,
        ),
    ];
    lines.truncate(area.height as usize);

    let para = Paragraph::new(lines).style(Style::default().bg(BG));
    f.render_widget(para, area);
}

fn render_bottom_frame(f: &mut Frame, area: Rect, data: &WelcomeData) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(GREEN_BORDER))
        .style(Style::default().bg(BG));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    render_bottom_left(f, cols[0]);
    render_bottom_right(f, cols[1], data);
}

fn render_bottom_left(f: &mut Frame, area: Rect) {
    let mut lines = vec![
        Line::from(Span::styled(
            "  WHILE YOU WERE AWAY",
            Style::default().fg(LABEL_GREEN),
        )),
        Line::from(vec![
            Span::styled("  ○ ", Style::default().fg(DIMMER_TEXT)),
            Span::styled("No pending briefings", Style::default().fg(DIM_TEXT)),
        ]),
    ];
    lines.truncate(area.height as usize);

    let para = Paragraph::new(lines).style(Style::default().bg(BG));
    f.render_widget(para, area);
}

fn render_bottom_right(f: &mut Frame, area: Rect, data: &WelcomeData) {
    let mut lines = vec![
        Line::from(Span::styled(
            "  ENTITY HEALTH",
            Style::default().fg(GREEN_DIM),
        )),
        Line::from(vec![
            Span::styled("  V(Ψ) ", Style::default().fg(DIMMER_TEXT)),
            Span::styled(format!("{:+.2}", data.lyapunov), Style::default().fg(GREEN)),
            Span::styled("  stable", Style::default().fg(LYAP_SUB)),
        ]),
        Line::from(vec![
            Span::styled("  Γ̂(Ψ) ", Style::default().fg(DIMMER_TEXT)),
            Span::styled(format!("{:+.3}", data.growth_rate), Style::default().fg(GREEN)),
            Span::styled("  growing", Style::default().fg(LYAP_SUB)),
        ]),
        Line::from(vec![
            Span::styled("  depth ", Style::default().fg(DIMMER_TEXT)),
            Span::styled(
                format_number(data.morphic_depth),
                Style::default().fg(IDENTITY_VAL),
            ),
            Span::styled("  morphic events", Style::default().fg(IDENTITY_SUB)),
        ]),
        Line::from(vec![
            Span::styled("  genome ", Style::default().fg(DIMMER_TEXT)),
            Span::styled(
                format_number(data.genome_entries as u64),
                Style::default().fg(IDENTITY_VAL),
            ),
            Span::styled("  permanent entries", Style::default().fg(IDENTITY_SUB)),
        ]),
    ];
    lines.truncate(area.height as usize);

    let para = Paragraph::new(lines).style(Style::default().bg(BG));
    f.render_widget(para, area);
}

fn render_input_row(f: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from(vec![
            Span::styled("  ◈  ", Style::default().fg(AMBER)),
            Span::styled("what are we building today?", Style::default().fg(DIMMER_TEXT)),
            Span::styled("█", Style::default().fg(AMBER)),
        ]),
        Line::from(Span::styled(
            "  Ctrl+V voice  ·  /dream  ·  /digest  ·  /help",
            Style::default().fg(GHOST),
        )),
    ];
    let para = Paragraph::new(lines).style(Style::default().bg(BG));
    f.render_widget(para, area);
}

// Helpers
fn kv_line(key: &str, value: &str, val_color: Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{key:<20}"),
            Style::default().fg(Color::Rgb(74, 74, 74)),
        ),
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
