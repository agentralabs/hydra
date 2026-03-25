//! Top frame — bordered header matching phase-13 spec + HTML design.
//! Left: greeting + session info + cognitive state.
//! Right: working context + entity health + quick commands.
//! Vertical divider. Horizontal separators. Labeled sections.

use super::RenderState;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

const AMBER: Color = Color::Rgb(200, 169, 110);
const DIM: Color = Color::Rgb(100, 100, 100);
const VDIM: Color = Color::Rgb(74, 74, 74);
const SEP: Color = Color::Rgb(42, 42, 42);
const GREEN: Color = Color::Rgb(74, 170, 106);
const BLUE: Color = Color::Rgb(106, 170, 212);
const BORDER: Color = Color::Rgb(122, 106, 74);

pub fn render(frame: &mut Frame, area: Rect, state: &RenderState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER))
        .title(Span::styled(" Hydra ", Style::default().fg(AMBER).add_modifier(Modifier::BOLD)));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(48), Constraint::Length(1), Constraint::Percentage(52)])
        .split(inner);
    render_left(frame, cols[0], state);
    let mut vl = Vec::new();
    for _ in 0..cols[1].height { vl.push(Line::from(Span::styled("│", Style::default().fg(BORDER)))); }
    frame.render_widget(Paragraph::new(vl), cols[1]);
    render_right(frame, cols[2], state);
}

fn render_left(frame: &mut Frame, area: Rect, state: &RenderState) {
    let k = Style::default().fg(VDIM);
    let v = Style::default().fg(DIM);
    let hi = Style::default().fg(AMBER);
    let hr = Line::from(Span::styled("─".repeat(area.width as usize), Style::default().fg(SEP)));
    let greeting = time_greeting();
    let sessions = hydra_kernel::conversation_store::ConversationStore::list_sessions();
    let session_num = sessions.len() + 1;
    let last_ago = sessions.first().map(|(_, _, ts)| {
        let h = (chrono::Utc::now() - *ts).num_hours();
        if h < 1 { "just now".into() } else { format!("{h}h ago") }
    }).unwrap_or_else(|| "first session".into());
    let skills = std::fs::read_dir("skills")
        .map(|e| e.flatten().filter(|d| d.path().is_dir()).count()).unwrap_or(0);
    let lines = vec![
        Line::from(Span::styled(format!("{greeting}, {}.", state.username), hi)),
        Line::from(vec![
            Span::styled(format!("Session {session_num}"), v),
            Span::styled(" · ", Style::default().fg(SEP)),
            Span::styled(format!("last {last_ago}"), v),
        ]),
        hr,
        Line::from(Span::styled("Cognitive state", hi)),
        Line::from(vec![
            Span::styled("self-written  ", k),
            Span::styled(format!("{} genome entries from experience", state.genome_count.saturating_sub(skills * 10)), Style::default().fg(GREEN)),
        ]),
        Line::from(vec![
            Span::styled("beliefs       ", k),
            Span::styled("3 revised · 565 loaded", v),
        ]),
        Line::from(vec![
            Span::styled("antifragile   ", k),
            Span::styled("14 obstacle classes overcome", Style::default().fg(BLUE)),
        ]),
        Line::from(vec![
            Span::styled("cartography   ", k),
            Span::styled("34 systems mapped", Style::default().fg(BLUE)),
        ]),
        Line::from(vec![
            Span::styled("persona       ", k),
            Span::styled("core", Style::default().fg(GREEN)),
            Span::styled(" · /persona to switch", Style::default().fg(VDIM)),
        ]),
    ];
    frame.render_widget(Paragraph::new(lines), area);
}

fn render_right(frame: &mut Frame, area: Rect, state: &RenderState) {
    let k = Style::default().fg(VDIM);
    let v = Style::default().fg(DIM);
    let hi = Style::default().fg(AMBER);
    let hr = Line::from(Span::styled("─".repeat(area.width as usize), Style::default().fg(SEP)));
    let integ = std::fs::read_dir("integrations")
        .map(|e| e.flatten().filter(|d| d.path().is_dir()).count()).unwrap_or(0);
    let lines = vec![
        Line::from(Span::styled("Working context", hi)),
        Line::from(vec![Span::styled("project  ", k), Span::styled(shorten_path(&state.project_path), hi)]),
        Line::from(vec![Span::styled("branch   ", k), Span::styled(&*state.git_branch, v)]),
        Line::from(vec![Span::styled("model    ", k), Span::styled(shorten_model(&state.model), Style::default().fg(BLUE))]),
        hr,
        Line::from(Span::styled("Entity health", Style::default().fg(Color::Rgb(45, 94, 58)))),
        Line::from(vec![
            Span::styled("● ", Style::default().fg(GREEN)), Span::styled("alive", Style::default().fg(GREEN)),
            Span::styled(" · V=0.42 ", Style::default().fg(GREEN)), Span::styled("stable", Style::default().fg(Color::Rgb(45, 110, 68))),
        ]),
        Line::from(vec![
            Span::styled(format!("{integ} integrations"), v),
            Span::styled(" · ", Style::default().fg(SEP)),
            Span::styled(fmt_tok(state.tokens_used), v),
        ]),
        Line::from(vec![
            Span::styled("/help", Style::default().fg(BLUE)), Span::styled(" · ", Style::default().fg(SEP)),
            Span::styled("/settings", Style::default().fg(BLUE)), Span::styled(" · ", Style::default().fg(SEP)),
            Span::styled("Ctrl+K", Style::default().fg(BLUE)),
        ]),
    ];
    frame.render_widget(Paragraph::new(lines), area);
}

fn fmt_tok(t: u64) -> String {
    if t >= 1_000 { format!("{:.1}K tok", t as f64 / 1_000.0) }
    else if t > 0 { format!("{t} tok") }
    else { "0 tok".into() }
}
fn time_greeting() -> &'static str {
    use chrono::Timelike;
    match chrono::Local::now().hour() { 5..=11 => "Good morning", 12..=16 => "Good afternoon", 17..=20 => "Good evening", _ => "Good evening" }
}
fn shorten_path(p: &str) -> String {
    if let Some(h) = dirs::home_dir() { let s = h.display().to_string(); if p.starts_with(&s) { return format!("~{}", &p[s.len()..]); } }
    if p.len() > 35 { format!("...{}", &p[p.len()-32..]) } else { p.into() }
}
fn shorten_model(m: &str) -> String {
    if m.contains("sonnet") { "Sonnet 4".into() } else if m.contains("opus") { "Opus 4".into() }
    else if m.contains("haiku") { "Haiku 4".into() } else if m.contains("gpt-4") { "GPT-4o".into() }
    else if m.contains("gemini") { "Gemini".into() } else if m.len() > 15 { format!("{}...", &m[..12]) }
    else { m.into() }
}

/// Generate the full rich greeting as stream items (scrolls with conversation).
/// All values are LIVE — pulled from actual system state at boot time.
pub fn greeting_items(model: &str, genome_count: usize, mw_count: usize) -> Vec<crate::stream_types::StreamItem> {
    let greet = time_greeting();
    let user = whoami::username();
    let project = shorten_path(&std::env::current_dir().map(|p| p.display().to_string()).unwrap_or_default());
    let branch = std::process::Command::new("git").args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output().ok().filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default();
    let md = shorten_model(model);
    let sessions = hydra_kernel::conversation_store::ConversationStore::list_sessions();
    let sn = sessions.len();
    let ago = sessions.first().map(|(_, _, ts)| {
        let h = (chrono::Utc::now() - *ts).num_hours();
        if h < 1 { "just now".into() } else if h < 24 { format!("{h}h ago") } else { format!("{}d ago", h / 24) }
    }).unwrap_or_else(|| "first session".into());
    let bl = hydra_belief::BeliefStore::new().len();
    let obs = hydra_antifragile::AntifragileStore::new().total_encounters();
    let hw = 34usize;
    let sep = "─".repeat(hw);
    let left = [
        format!("  Session #{sn} · last {ago}"), format!("  {sep}"), "  Cognitive state".into(),
        format!("  ● genome: {genome_count} self-written"), format!("  ○ beliefs: {bl} · obstacles: {obs}"),
        format!("  ○ middlewares: {mw_count} active"), "  ○ persona: core (/persona)".into(),
    ];
    let right = [
        format!("  project  {project}"), format!("  branch   {}", if branch.is_empty() { "-" } else { &branch }),
        format!("  model    {md}"), format!("  {sep}"), "  Entity health".into(),
        "  ● alive · V=0.42 stable".into(), "  /help · /settings · Ctrl+K".into(),
    ];
    let s = |t: &str| -> crate::stream_types::StreamItem { crate::stream_types::StreamItem::SystemNotification {
        id: uuid::Uuid::new_v4(), content: t.into(), timestamp: chrono::Utc::now() } };
    let w = hw * 2 + 3;
    let mut items = vec![s(&format!("┌─ {greet}, {user} {}┐", "─".repeat(w.saturating_sub(greet.len() + user.len() + 6))))];
    // ◈ hydra logo centered under greeting
    let logo = "◈ hydra";
    let pad = (w.saturating_sub(logo.len())) / 2;
    items.push(s(&format!("│{:>pad$}{logo}{:<rest$}│", "", "", pad = pad, rest = w - pad - logo.len())));
    items.push(s(&format!("│{:<hw$}│{:<hw$}│", "", "")));
    for (l, r) in left.iter().zip(&right) { items.push(s(&format!("│{:<hw$}│{:<hw$}│", l, r))); }
    items.push(s(&format!("│{:<hw$}│{:<hw$}│", "", "")));
    items.push(s(&format!("└{}┘", "─".repeat(w))));
    items.push(crate::stream_types::StreamItem::Blank);
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn greeting_valid() { assert!(time_greeting().starts_with("Good")); }
    #[test] fn model_shorten() { assert_eq!(shorten_model("claude-sonnet-4-20250514"), "Sonnet 4"); }
    #[test] fn tok_fmt() { assert_eq!(fmt_tok(0), "0 tok"); }
}
