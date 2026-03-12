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

    /// Poll for the next event. This is synchronous but uses crossterm's poll.
    pub async fn next(&self) -> io::Result<Event> {
        if event::poll(self.tick_rate)? {
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
    // Global keybindings (work in any mode)
    match (key.modifiers, key.code) {
        (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
            app.should_quit = true;
            return;
        }
        (KeyModifiers::CONTROL, KeyCode::Char('d')) => {
            // Exit session (Claude Code parity)
            app.should_quit = true;
            return;
        }
        (KeyModifiers::CONTROL, KeyCode::Char('k')) => {
            // Kill switch — stop current execution
            app.kill_current();
            return;
        }
        (KeyModifiers::CONTROL, KeyCode::Char('s')) => {
            // Toggle sidebar (Hydra-exclusive: Ctrl+S)
            app.sidebar_visible = !app.sidebar_visible;
            return;
        }
        (KeyModifiers::CONTROL, KeyCode::Char('b')) => {
            // Background current operation (CC §6.3: Ctrl+B)
            // If a command is running, it's already async; inform user
            if app.running_cmd.is_some() {
                app.messages.push(super::app::Message {
                    role: super::app::MessageRole::System,
                    content: "Operation running in background. Use /bashes to check status.".to_string(),
                    timestamp: String::new(),
                    phase: None,
                });
            }
            return;
        }
        (KeyModifiers::CONTROL, KeyCode::Char('e')) => {
            // Quick environment check (Hydra §6.5: Ctrl+E)
            let ts = String::new();
            app.slash_cmd_env("", &ts);
            return;
        }
        (KeyModifiers::CONTROL, KeyCode::Char('t')) => {
            // Toggle task list (CC §6.3: Ctrl+T)
            let ts = String::new();
            app.slash_cmd_tasks(&ts);
            return;
        }
        (KeyModifiers::CONTROL, KeyCode::Char('g')) => {
            // Open external editor for input (CC §6.4: Ctrl+G)
            app.messages.push(super::app::Message {
                role: super::app::MessageRole::System,
                content: "External editor: use /edit <file> to open $EDITOR.".to_string(),
                timestamp: String::new(),
                phase: None,
            });
            return;
        }
        (KeyModifiers::CONTROL, KeyCode::Char('f')) => {
            // Kill all background agents (CC §6.4: Ctrl+F)
            app.kill_current();
            app.messages.push(super::app::Message {
                role: super::app::MessageRole::System,
                content: "All background agents killed.".to_string(),
                timestamp: String::new(),
                phase: None,
            });
            return;
        }
        (KeyModifiers::CONTROL, KeyCode::Char('o')) => {
            // Toggle tool output expand/collapse (Visual Overhaul Rule 5: ctrl+o)
            app.tool_output_expanded = !app.tool_output_expanded;
            return;
        }
        (KeyModifiers::CONTROL, KeyCode::Char('l')) => {
            // Refresh
            app.refresh_status();
            return;
        }
        (KeyModifiers::SHIFT, KeyCode::BackTab) => {
            // Shift+Tab: Cycle permission mode (Normal → Auto-Accept → Plan)
            app.permission_mode = app.permission_mode.next();
            return;
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
            app.scroll_up();
            return;
        }
        (KeyModifiers::SHIFT, KeyCode::Down) => {
            app.scroll_down();
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
                    // Clear input (like Claude Code Escape behavior)
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
            // If dropdown is visible, select the highlighted command first
            if app.command_dropdown.visible {
                if let Some(name) = app.command_dropdown.selected_command() {
                    app.input = name.to_string();
                    app.cursor_pos = char_len(&app.input);
                    app.command_dropdown.close();
                }
                return;
            }
            if !app.input.is_empty() {
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
            app.cursor_pos = 0;
        }
        KeyCode::End => {
            app.cursor_pos = app.input.len();
        }
        KeyCode::Up => {
            if app.command_dropdown.visible {
                app.command_dropdown.select_prev();
            } else if app.input.is_empty() {
                // Empty input → scroll conversation (like Claude Code)
                app.scroll_up();
            } else {
                app.history_prev();
            }
        }
        KeyCode::Down => {
            if app.command_dropdown.visible {
                app.command_dropdown.select_next();
            } else if app.input.is_empty() {
                // Empty input → scroll conversation (like Claude Code)
                app.scroll_down();
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
