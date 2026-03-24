//! Status bar — spec format: ◈ Hydra session:Xm tasks:N V=0.42 tokens:12k ◑ Verb
//! Left: entity + session + tasks + lyapunov. Right: tokens + thinking verb.

use super::RenderState;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub fn render(frame: &mut Frame, area: Rect, state: &RenderState) {
    let dim = Style::default().fg(Color::DarkGray);
    let sep = Span::styled("  ", dim);

    // ── LEFT SECTION: context ──
    let mut left = vec![
        Span::styled(" ◈ ", Style::default().fg(Color::Rgb(200, 169, 110))),
        Span::styled("Hydra", Style::default().fg(Color::Rgb(200, 169, 110))),
        sep.clone(),
        Span::styled(format!("session:{}m", state.session_minutes), dim),
        sep.clone(),
        Span::styled(format!("tasks:{}", state.task_count), dim),
        sep.clone(),
        Span::styled(format!("genome:{}", state.genome_count), dim),
        sep.clone(),
    ];

    // Lyapunov stability — color-coded
    let (v_color, v_label) = if state.lyapunov > 0.3 {
        (Color::Green, "")
    } else if state.lyapunov > 0.0 {
        (Color::Yellow, " watch")
    } else {
        (Color::Red, " intervention")
    };
    left.push(Span::styled(
        format!("V={:.2}{v_label}", state.lyapunov),
        Style::default().fg(v_color),
    ));

    // Computer-use indicators
    if state.shell_mode {
        left.push(sep.clone());
        left.push(Span::styled("SHELL", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)));
    }
    if state.agent_active {
        left.push(sep.clone());
        left.push(Span::styled("AGENT", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)));
    }
    if let Some(remaining) = state.vision_budget_remaining {
        left.push(sep.clone());
        let color = if remaining > 50 { Color::DarkGray } else if remaining > 10 { Color::Yellow } else { Color::Red };
        left.push(Span::styled(format!("vision:{remaining}"), Style::default().fg(color)));
    }

    // O16: Monitor indicators
    if state.monitor_count > 0 {
        left.push(sep.clone());
        left.push(Span::styled(format!("monitors:{}", state.monitor_count), dim));
        if state.alert_count > 0 {
            left.push(sep.clone());
            left.push(Span::styled(
                format!("alerts:{}", state.alert_count),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ));
        }
    }

    // O17: Voice presence state indicator
    if let Some(ref voice_state) = state.voice_state {
        left.push(sep.clone());
        let (color, label) = match voice_state.as_str() {
            "dormant" => (Color::DarkGray, "MIC"),
            "listening" => (Color::Green, "LISTENING"),
            "processing" => (Color::Yellow, "VOICE"),
            "speaking" => (Color::Cyan, "SPEAKING"),
            _ => (Color::DarkGray, "MIC"),
        };
        left.push(Span::styled(label, Style::default().fg(color).add_modifier(Modifier::BOLD)));
    }

    // Session 22: Alive signal
    if let Some(ref msg) = state.alive_message {
        left.push(sep.clone());
        left.push(Span::styled(format!("◌ {msg}"), Style::default().fg(Color::DarkGray)));
    }

    // ── RIGHT SECTION: current operation ──
    let mut right = vec![];

    // Token count
    right.push(Span::styled(fmt_tok(state.tokens_used), dim));

    // Context warning at 80%
    if state.tokens_used > 80_000 {
        right.push(Span::styled(
            "  ● 80%",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ));
    }

    // Thinking verb + spinner (only when active)
    if state.is_thinking {
        let spinners = ['◌', '◐', '◑', '◒', '◓', '●'];
        let sp = spinners[state.think_spinner_frame % spinners.len()];
        right.push(Span::styled(
            format!("  {sp} {}", state.thinking_verb),
            Style::default().fg(Color::Rgb(200, 169, 110)).add_modifier(Modifier::ITALIC),
        ));
    }

    // Compose: left + padding + right
    let left_width: usize = left.iter().map(|s| s.content.len()).sum();
    let right_width: usize = right.iter().map(|s| s.content.len()).sum();
    let padding = (area.width as usize).saturating_sub(left_width + right_width + 1);

    let mut spans = left;
    spans.push(Span::raw(" ".repeat(padding)));
    spans.extend(right);
    spans.push(Span::raw(" "));

    let p = Paragraph::new(vec![Line::from(spans)])
        .style(Style::default().bg(Color::Rgb(25, 25, 35)));
    frame.render_widget(p, area);
}

fn fmt_tok(t: u64) -> String {
    if t >= 1_000_000 { format!("tokens:{:.1}M", t as f64 / 1_000_000.0) }
    else if t >= 1_000 { format!("tokens:{:.1}k", t as f64 / 1_000.0) }
    else if t > 0 { format!("tokens:{t}") }
    else { "tokens:0".into() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn tok_fmt() { assert_eq!(fmt_tok(1500), "tokens:1.5k"); assert_eq!(fmt_tok(42), "tokens:42"); }
}
