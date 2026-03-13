use std::io;
use std::time::Duration;

use crossterm::event::{self, Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers, MouseEvent};

use super::app::{App, InputMode};

/// Convert char index to byte index (clamped to string length).
fn char_to_byte(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

fn char_len(s: &str) -> usize {
    s.chars().count()
}

/// Find word boundary going backward from char position.
fn word_boundary_back(s: &str, pos: usize) -> usize {
    let chars: Vec<char> = s.chars().collect();
    let mut i = pos;
    while i > 0 && !chars[i - 1].is_alphanumeric() { i -= 1; }
    while i > 0 && chars[i - 1].is_alphanumeric() { i -= 1; }
    i
}

/// Find word boundary going forward from char position.
fn word_boundary_fwd(s: &str, pos: usize) -> usize {
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    let mut i = pos;
    while i < len && !chars[i].is_alphanumeric() { i += 1; }
    while i < len && chars[i].is_alphanumeric() { i += 1; }
    i
}

/// Terminal events.
#[derive(Debug)]
pub enum Event {
    /// Terminal tick (periodic refresh).
    Tick,
    /// Key press.
    Key(KeyEvent),
    /// Mouse event.
    Mouse(MouseEvent),
    /// Terminal resize.
    Resize(u16, u16),
}

/// Handles terminal events using crossterm polling.
pub struct EventHandler {
    tick_rate: Duration,
}

impl EventHandler {
    pub fn new(tick_rate_ms: u64) -> Self {
        Self {
            tick_rate: Duration::from_millis(tick_rate_ms),
        }
    }

    /// Poll for the next event with a short timeout for responsiveness.
    /// Uses a 16ms poll (≈60fps) so cognitive updates are processed quickly.
    /// The tick_rate controls the minimum interval between tick callbacks.
    pub async fn next(&self) -> io::Result<Event> {
        // Use short poll (16ms) so cognitive updates are visible in real-time.
        // Tick events fire at tick_rate intervals even if no terminal events occur.
        let poll_timeout = Duration::from_millis(16);
        if event::poll(poll_timeout)? {
            match event::read()? {
                CrosstermEvent::Key(key) => Ok(Event::Key(key)),
                CrosstermEvent::Mouse(mouse) => Ok(Event::Mouse(mouse)),
                CrosstermEvent::Resize(w, h) => Ok(Event::Resize(w, h)),
                _ => Ok(Event::Tick),
            }
        } else {
            Ok(Event::Tick)
        }
    }
}

/// Handle a key event and update app state.
pub fn handle_key_event(app: &mut App, key: KeyEvent) {
    // Ctrl+C must ALWAYS work regardless of event kind (Press/Release/Repeat)
    if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('c') {
        if app.is_thinking || app.running_cmd.is_some() {
            app.kill_current();
            app.is_thinking = false;
        } else {
            app.should_quit = true;
        }
        return;
    }
    // Only handle key press events — ignore Release/Repeat to prevent double-fire
    if key.kind != crossterm::event::KeyEventKind::Press {
        return;
    }
    // Global keybindings (work in any mode)
    match (key.modifiers, key.code) {
        // ── Readline shortcuts ──
        (KeyModifiers::CONTROL, KeyCode::Char('a')) => {
            app.cursor_pos = 0; return;
        }
        (KeyModifiers::CONTROL, KeyCode::Char('e')) => {
            app.cursor_pos = char_len(&app.input); return;
        }
        (KeyModifiers::CONTROL, KeyCode::Char('k')) => {
            // Delete from cursor to end → kill ring
            let byte_idx = char_to_byte(&app.input, app.cursor_pos);
            app.kill_ring = app.input[byte_idx..].to_string();
            app.input.truncate(byte_idx);
            return;
        }
        (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
            // Delete entire line → kill ring
            app.kill_ring = app.input.clone();
            app.input.clear();
            app.cursor_pos = 0;
            return;
        }
        (KeyModifiers::CONTROL, KeyCode::Char('w')) => {
            // Delete word backward
            if app.cursor_pos > 0 {
                let old_pos = app.cursor_pos;
                let new_pos = word_boundary_back(&app.input, app.cursor_pos);
                let b0 = char_to_byte(&app.input, new_pos);
                let b1 = char_to_byte(&app.input, old_pos);
                app.kill_ring = app.input[b0..b1].to_string();
                app.input.replace_range(b0..b1, "");
                app.cursor_pos = new_pos;
            }
            return;
        }
        (KeyModifiers::CONTROL, KeyCode::Char('y')) => {
            // Yank (paste from kill ring)
            if !app.kill_ring.is_empty() {
                let byte_idx = char_to_byte(&app.input, app.cursor_pos);
                app.input.insert_str(byte_idx, &app.kill_ring.clone());
                app.cursor_pos += app.kill_ring.chars().count();
            }
            return;
        }
        (KeyModifiers::ALT, KeyCode::Char('b')) => {
            app.cursor_pos = word_boundary_back(&app.input, app.cursor_pos); return;
        }
        (KeyModifiers::ALT, KeyCode::Char('f')) => {
            app.cursor_pos = word_boundary_fwd(&app.input, app.cursor_pos); return;
        }
        // ── Session controls ──
        (KeyModifiers::CONTROL, KeyCode::Char('d')) => {
            if !app.input.is_empty() { app.input.clear(); app.cursor_pos = 0; }
            return;
        }
        (KeyModifiers::CONTROL, KeyCode::Char('l')) => {
            // Clear conversation
            app.messages.clear();
            app.scroll_offset = 0;
            return;
        }
        (KeyModifiers::CONTROL, KeyCode::Char('r')) => {
            app.search_mode = true; app.search_query.clear(); return;
        }
        (KeyModifiers::CONTROL, KeyCode::Char('o')) => {
            app.tool_output_expanded = !app.tool_output_expanded; return;
        }
        (KeyModifiers::CONTROL, KeyCode::Char('t')) => {
            let ts = String::new(); app.slash_cmd_tasks(&ts); return;
        }
        (KeyModifiers::CONTROL, KeyCode::Char('s')) => {
            app.sidebar_visible = !app.sidebar_visible; return;
        }
        (KeyModifiers::SHIFT, KeyCode::BackTab) => {
            app.permission_mode = app.permission_mode.next(); return;
        }
        // Scroll works in ANY mode — no need to switch to Normal
        (_, KeyCode::PageUp) => {
            app.page_up();
            return;
        }
        (_, KeyCode::PageDown) => {
            app.page_down();
            return;
        }
        (KeyModifiers::SHIFT, KeyCode::Up) => {
            app.scroll_up_n(3);
            return;
        }
        (KeyModifiers::SHIFT, KeyCode::Down) => {
            app.scroll_down_n(3);
            return;
        }
        _ => {}
    }

    match app.input_mode {
        InputMode::Normal => handle_normal_mode(app, key),
        InputMode::Insert => handle_insert_mode(app, key),
    }
}

fn handle_normal_mode(app: &mut App, key: KeyEvent) {
    // No separate Normal mode — always redirect to Insert mode
    app.input_mode = InputMode::Insert;
    handle_insert_mode(app, key);
}

fn handle_insert_mode(app: &mut App, key: KeyEvent) {
    // Reverse search mode (Ctrl+R)
    if app.search_mode {
        match key.code {
            KeyCode::Esc => {
                app.search_mode = false;
                app.search_query.clear();
            }
            KeyCode::Enter => {
                // Accept the found result
                app.search_mode = false;
                app.search_query.clear();
            }
            KeyCode::Backspace => {
                app.search_query.pop();
                reverse_search_update(app);
            }
            KeyCode::Char(c) => {
                app.search_query.push(c);
                reverse_search_update(app);
            }
            _ => {}
        }
        return;
    }
    match key.code {
        KeyCode::Esc => {
            if app.command_dropdown.visible {
                app.command_dropdown.close();
            } else {
                // Esc+Esc detection (§6.1: double-Esc opens rewind menu)
                let now = app.tick_count;
                if now.saturating_sub(app.last_esc_tick) < 4 && app.input.is_empty() {
                    // Double Esc — show rewind menu
                    let ts = String::new();
                    app.messages.push(super::app::Message {
                        role: super::app::MessageRole::System,
                        content: "Rewind Menu\n\n\
                                 1. Rewind everything (conversation + code)\n\
                                 2. Rewind conversation only (keep code changes)\n\
                                 3. Rewind code only (keep conversation)\n\
                                 4. Cancel\n\n\
                                 Type 1-4 to select:".to_string(),
                        timestamp: ts,
                        phase: None,
                    });
                    app.last_esc_tick = 0;
                } else if !app.input.is_empty() {
                    // Clear input on Escape
                    app.input.clear();
                    app.cursor_pos = 0;
                    app.completions.clear();
                    app.last_esc_tick = 0;
                } else {
                    app.last_esc_tick = now;
                }
            }
        }
        KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => {
            // Shift+Enter: insert newline for multiline input (§4.1)
            let byte_idx = char_to_byte(&app.input, app.cursor_pos);
            app.input.insert(byte_idx, '\n');
            app.cursor_pos += 1;
        }
        KeyCode::Enter => {
            // If dropdown is visible, select the highlighted command
            if app.command_dropdown.visible {
                if let Some(name) = app.command_dropdown.selected_command() {
                    app.input = name.to_string();
                    app.cursor_pos = char_len(&app.input);
                    app.command_dropdown.close();
                }
                return;
            }
            // Backslash-Enter: replace trailing \ with newline
            if app.cursor_pos > 0 {
                let prev_byte = char_to_byte(&app.input, app.cursor_pos - 1);
                if app.input.as_bytes().get(prev_byte) == Some(&b'\\') {
                    app.input.replace_range(prev_byte..prev_byte + 1, "\n");
                    return;
                }
            }
            // Debounce: ignore Enter if fired within 3 ticks of last submit
            let now = app.tick_count;
            if now.saturating_sub(app.last_submit_tick) < 3 { return; }
            if !app.input.is_empty() {
                app.last_submit_tick = now;
                let input = app.input.clone();
                app.input.clear();
                app.cursor_pos = 0;
                app.completions.clear();
                app.submit_input(&input);
            }
        }
        KeyCode::Char(c) => {
            let byte_idx = char_to_byte(&app.input, app.cursor_pos);
            app.input.insert(byte_idx, c);
            app.cursor_pos += 1;
            app.completions.clear();
            app.update_dropdown();
        }
        KeyCode::Backspace => {
            if app.cursor_pos > 0 {
                app.cursor_pos -= 1;
                let byte_idx = char_to_byte(&app.input, app.cursor_pos);
                app.input.remove(byte_idx);
                app.completions.clear();
                app.update_dropdown();
            }
        }
        KeyCode::Delete => {
            if app.cursor_pos < char_len(&app.input) {
                let byte_idx = char_to_byte(&app.input, app.cursor_pos);
                app.input.remove(byte_idx);
                app.completions.clear();
                app.update_dropdown();
            }
        }
        KeyCode::Left => {
            if app.cursor_pos > 0 {
                app.cursor_pos -= 1;
            }
        }
        KeyCode::Right => {
            if app.cursor_pos < char_len(&app.input) {
                app.cursor_pos += 1;
            }
        }
        KeyCode::Home => {
            if app.input.is_empty() {
                app.scroll_to_top();
            } else {
                app.cursor_pos = 0;
            }
        }
        KeyCode::End => {
            if app.input.is_empty() {
                app.scroll_to_bottom();
            } else {
                app.cursor_pos = app.input.len();
            }
        }
        KeyCode::Up => {
            if app.command_dropdown.visible {
                app.command_dropdown.select_prev();
            } else if app.input.is_empty() {
                app.scroll_up_n(3);
            } else {
                app.history_prev();
            }
        }
        KeyCode::Down => {
            if app.command_dropdown.visible {
                app.command_dropdown.select_next();
            } else if app.input.is_empty() {
                app.scroll_down_n(3);
            } else {
                app.history_next();
            }
        }
        KeyCode::Tab => {
            app.tab_complete();
        }
        _ => {}
    }
}

/// Update input from reverse search through history.
fn reverse_search_update(app: &mut App) {
    if app.search_query.is_empty() { return; }
    let q = app.search_query.to_lowercase();
    for entry in app.history.iter().rev() {
        if entry.to_lowercase().contains(&q) {
            app.input = entry.clone();
            app.cursor_pos = char_len(&app.input);
            return;
        }
    }
}
