//! View layer — pure rendering functions. No mutation. No side effects.
//! render(frame, state) is the only entry point.

pub mod input;
pub mod rich;
pub mod modal;
pub mod status_bar;
pub mod stream;
pub mod top_frame;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};

/// Top-level render — Claude Code style layout with top frame.
/// Top frame (5 lines) → Stream fills middle → Input at bottom → Status bar.
pub fn render(frame: &mut Frame, state: &RenderState) {
    let area = frame.area();

    // Calculate input height (1-5 lines based on content)
    let input_lines = state.input_line_count.clamp(1, 5) as u16;
    let input_height = input_lines + 1;

    // Greeting is now part of the stream (scrolls naturally like Claude Code)
    let top_frame_height = 0u16;

    // Slash menu: when input starts with "/", show inline command list below input
    let slash_active = state.input_text.starts_with('/') && !state.is_thinking;
    let slash_height = if slash_active { 10u16 } else { 0 };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(top_frame_height),  // top frame
            Constraint::Min(3),                    // stream (fills everything)
            Constraint::Length(input_height),       // input
            Constraint::Length(slash_height),       // slash menu (0 when hidden)
            Constraint::Length(1),                 // status bar
        ])
        .split(area);

    stream::render(frame, chunks[1], state);
    input::render(frame, chunks[2], state);
    if slash_active {
        render_slash_menu(frame, chunks[3], state);
    }
    status_bar::render(frame, chunks[4], state);

    // Modal overlay (config editor, session list — NOT slash menu)
    if state.modal_active && !slash_active {
        modal::render(frame, area, state);
    }
}

/// Render state — immutable snapshot for the view layer.
/// Built from AppState before each render call. No mutation allowed.
#[derive(Debug)]
pub struct RenderState {
    // Stream (Arc for zero-copy from AppState — Fix 1)
    pub stream_items: std::sync::Arc<Vec<crate::stream_types::StreamItem>>,
    pub stream_scroll_offset: usize,
    pub is_thinking: bool,
    pub thinking_verb: std::sync::Arc<str>,
    pub thinking_color: ratatui::style::Color,
    pub think_spinner_frame: usize,

    // Input (these change per-keystroke, keep as String)
    pub input_text: String,
    pub input_cursor: usize,
    pub input_line_count: usize,
    pub input_placeholder: std::sync::Arc<str>,
    pub is_searching: bool,
    pub search_query: String,

    // Status bar
    pub genome_count: usize,
    pub memory_size_kb: u64,
    pub middleware_count: usize,
    pub provider: std::sync::Arc<str>,
    pub model: std::sync::Arc<str>,
    pub session_minutes: u64,
    pub tokens_used: u64,
    pub mode: std::sync::Arc<str>,

    // Status bar spec fields
    pub lyapunov: f64,
    pub task_count: usize,

    // Slash menu
    pub slash_selected: usize,

    // Top frame
    pub show_top_frame: bool,
    pub username: std::sync::Arc<str>,
    pub project_path: std::sync::Arc<str>,
    pub git_branch: std::sync::Arc<str>,

    // Modal
    pub modal_active: bool,
    pub modal: Option<crate::v2::modal::Modal>,

    // Computer use
    pub vision_budget_remaining: Option<u32>,
    pub shell_mode: bool,
    pub agent_active: bool,

    // Theme
    pub theme: crate::theme::Theme,

    // Voice presence (O17)
    pub voice_state: Option<String>,
    // Omniscient Monitor (O16)
    pub monitor_count: usize,
    pub alert_count: usize,
    // Alive signal (Session 22)
    pub alive_message: Option<String>,
    // Spatial Presence (O19)
    pub presence_state: Option<String>,
    // Scroll badge
    pub new_while_scrolled: usize,
}

impl Default for RenderState {
    fn default() -> Self {
        Self {
            stream_items: std::sync::Arc::new(Vec::new()),
            stream_scroll_offset: 0,
            is_thinking: false,
            thinking_verb: "".into(),
            thinking_color: ratatui::style::Color::White,
            think_spinner_frame: 0,
            input_text: String::new(),
            input_cursor: 0,
            input_line_count: 1,
            input_placeholder: "What are we building today?".into(),
            is_searching: false,
            search_query: String::new(),
            genome_count: 0,
            memory_size_kb: 0,
            middleware_count: 0,
            provider: "anthropic".into(),
            model: "sonnet".into(),
            session_minutes: 0,
            tokens_used: 0,
            mode: "local".into(),
            lyapunov: 0.42,
            task_count: 0,
            slash_selected: 0,
            show_top_frame: true,
            username: whoami::username().into(),
            project_path: std::env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_default().into(),
            git_branch: "".into(),
            modal_active: false,
            modal: None,
            vision_budget_remaining: None,
            shell_mode: false,
            agent_active: false,
            theme: crate::theme::Theme::dark(),
            voice_state: None,
            monitor_count: 0,
            alert_count: 0,
            alive_message: None,
            presence_state: None,
            new_while_scrolled: 0,
        }
    }
}

/// Claude Code style inline slash menu — plain list, no border, below input.
fn render_slash_menu(frame: &mut Frame, area: ratatui::layout::Rect, state: &RenderState) {
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{List, ListItem, Paragraph};

    // Thin separator line
    let sep_area = ratatui::layout::Rect { height: 1, ..area };
    let sep = Paragraph::new("─".repeat(area.width as usize))
        .style(Style::default().fg(Color::Rgb(60, 60, 80)));
    frame.render_widget(sep, sep_area);

    // Command list area (below separator)
    let list_area = ratatui::layout::Rect {
        y: area.y + 1,
        height: area.height.saturating_sub(1),
        ..area
    };

    let query = state.input_text.trim_start_matches('/');
    let registry = crate::v2::commands::build_registry();
    let results = registry.fuzzy_search(query);

    let items: Vec<ListItem> = results
        .iter()
        .take(list_area.height as usize)
        .enumerate()
        .map(|(i, (cmd_idx, _))| {
            let cmd = &registry.all()[*cmd_idx];
            let is_first = i == state.slash_selected;
            let style = if is_first {
                Style::default().fg(Color::White).bg(Color::Rgb(50, 50, 80))
            } else {
                Style::default().fg(Color::Gray)
            };
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("  /{:<18}", cmd.name),
                    style.add_modifier(if is_first { Modifier::BOLD } else { Modifier::empty() }),
                ),
                Span::styled(
                    cmd.description,
                    if is_first {
                        Style::default().fg(Color::Rgb(150, 150, 180)).bg(Color::Rgb(50, 50, 80))
                    } else {
                        Style::default().fg(Color::DarkGray)
                    },
                ),
            ]))
        })
        .collect();

    frame.render_widget(List::new(items), list_area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_render_state() {
        let state = RenderState::default();
        assert!(!state.is_thinking);
        assert!(!state.modal_active);
        assert_eq!(state.input_line_count, 1);
    }
}
