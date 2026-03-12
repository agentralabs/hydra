//! Navigation and scroll helpers — extracted from app.rs for file size.
//! Contains scroll, history, focus, dropdown, and tab completion methods.

use super::app::{App, FocusArea};

impl App {
    // Scroll — line-based offset from the bottom.
    // 0 = pinned to bottom (auto-scroll), >0 = scrolled up by N lines.
    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset += 1;
    }

    /// Scroll by N lines (for keyboard: 3 lines per arrow key).
    pub fn scroll_down_n(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
    }

    pub fn scroll_up_n(&mut self, n: usize) {
        self.scroll_offset += n;
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = usize::MAX / 2;
    }

    pub fn page_up(&mut self) {
        self.scroll_offset += 20;
    }

    pub fn page_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(20);
    }

    pub fn is_at_bottom(&self) -> bool {
        self.scroll_offset == 0
    }

    // History navigation
    pub fn history_prev(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let idx = match self.history_index {
            Some(i) => if i > 0 { i - 1 } else { 0 },
            None => self.history.len() - 1,
        };
        self.history_index = Some(idx);
        self.input = self.history[idx].clone();
        self.cursor_pos = self.input.len();
    }

    pub fn history_next(&mut self) {
        if let Some(idx) = self.history_index {
            if idx + 1 < self.history.len() {
                self.history_index = Some(idx + 1);
                self.input = self.history[idx + 1].clone();
                self.cursor_pos = self.input.len();
            } else {
                self.history_index = None;
                self.input.clear();
                self.cursor_pos = 0;
            }
        }
    }

    pub fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            FocusArea::Conversation => FocusArea::Sidebar,
            FocusArea::Sidebar => FocusArea::Conversation,
        };
    }

    /// Update the command dropdown filter based on current input.
    pub fn update_dropdown(&mut self) {
        self.command_dropdown.update_filter(&self.input);
    }

    /// Tab completion — if dropdown is visible, select the highlighted command.
    /// Otherwise fall back to cycling through matches.
    pub fn tab_complete(&mut self) {
        // If dropdown is showing, accept the selected command
        if self.command_dropdown.visible {
            if let Some(name) = self.command_dropdown.selected_command() {
                self.input = name.to_string();
                self.cursor_pos = self.input.len();
                self.command_dropdown.close();
            }
            return;
        }

        if self.input.is_empty() {
            return;
        }

        // Legacy cycle-through for non-dropdown cases
        if self.completions.is_empty() {
            use crate::tui::commands::COMMANDS;
            self.completions = COMMANDS
                .iter()
                .filter(|c| c.name.starts_with(&self.input))
                .map(|c| c.name.to_string())
                .collect();
            self.completion_index = 0;
        }

        if !self.completions.is_empty() {
            let completion = self.completions[self.completion_index].clone();
            self.input = completion;
            self.cursor_pos = self.input.len();
            self.completion_index = (self.completion_index + 1) % self.completions.len();
        }
    }
}
