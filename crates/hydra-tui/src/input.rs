//! Input box — text entry with cursor, history, yank, and word movement.

/// Maximum number of inputs stored in history.
const MAX_HISTORY: usize = 100;

use crate::input_search::SearchState;

/// A text input box with cursor support, history, yank buffer, and search.
#[derive(Debug, Clone)]
pub struct InputBox {
    /// The current text content.
    buffer: String,
    /// Cursor position (byte offset in the buffer).
    cursor: usize,
    /// Input history (most recent last).
    history: Vec<String>,
    /// Current position in history (None = editing new input).
    history_index: Option<usize>,
    /// Stashed current input when browsing history.
    stashed: String,
    /// Yank buffer for Ctrl+K/Ctrl+U/Ctrl+Y.
    yank_buffer: String,
    /// Reverse search state (Ctrl+R).
    search: SearchState,
}

impl InputBox {
    /// Create a new empty input box.
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            cursor: 0,
            history: Vec::new(),
            history_index: None,
            stashed: String::new(),
            yank_buffer: String::new(),
            search: SearchState::default(),
        }
    }

    /// Return the current text content.
    pub fn text(&self) -> &str {
        &self.buffer
    }

    /// Return the cursor position.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Return true if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Insert a character at the cursor position.
    pub fn insert(&mut self, ch: char) {
        self.buffer.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
    }

    /// Delete the character before the cursor (backspace).
    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            let prev = self.buffer[..self.cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.buffer.remove(prev);
            self.cursor = prev;
        }
    }

    /// Delete the character at the cursor position.
    pub fn delete(&mut self) {
        if self.cursor < self.buffer.len() {
            self.buffer.remove(self.cursor);
        }
    }

    /// Move cursor left by one character.
    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor = self.buffer[..self.cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }

    /// Move cursor right by one character.
    pub fn move_right(&mut self) {
        if self.cursor < self.buffer.len() {
            self.cursor = self.buffer[self.cursor..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor + i)
                .unwrap_or(self.buffer.len());
        }
    }

    /// Move cursor to the beginning (Ctrl+A / Home).
    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    /// Move cursor to the end (Ctrl+E / End).
    pub fn move_end(&mut self) {
        self.cursor = self.buffer.len();
    }

    /// Move cursor backward one word (Alt+B).
    pub fn move_word_backward(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let before = &self.buffer[..self.cursor];
        // Skip trailing whitespace, then skip word chars
        let trimmed = before.trim_end();
        if trimmed.is_empty() {
            self.cursor = 0;
            return;
        }
        let word_end = trimmed.len();
        let word_start = trimmed
            .rfind(|c: char| c.is_whitespace())
            .map(|i| i + 1)
            .unwrap_or(0);
        // Use byte offset directly since we're within valid UTF-8
        self.cursor = word_start.min(word_end);
    }

    /// Move cursor forward one word (Alt+F).
    pub fn move_word_forward(&mut self) {
        if self.cursor >= self.buffer.len() {
            return;
        }
        let after = &self.buffer[self.cursor..];
        // Skip current word chars, then skip whitespace
        let skip_word = after
            .find(|c: char| c.is_whitespace())
            .unwrap_or(after.len());
        let remaining = &after[skip_word..];
        let skip_space = remaining
            .find(|c: char| !c.is_whitespace())
            .unwrap_or(remaining.len());
        self.cursor += skip_word + skip_space;
    }

    /// Delete from cursor to end of line, store in yank buffer (Ctrl+K).
    pub fn kill_to_end(&mut self) {
        if self.cursor < self.buffer.len() {
            self.yank_buffer = self.buffer[self.cursor..].to_string();
            self.buffer.truncate(self.cursor);
        }
    }

    /// Delete entire line, store in yank buffer (Ctrl+U).
    pub fn kill_line(&mut self) {
        if !self.buffer.is_empty() {
            self.yank_buffer = std::mem::take(&mut self.buffer);
            self.cursor = 0;
        }
    }

    /// Paste yank buffer at cursor (Ctrl+Y).
    pub fn yank(&mut self) {
        if !self.yank_buffer.is_empty() {
            let yanked = self.yank_buffer.clone();
            self.buffer.insert_str(self.cursor, &yanked);
            self.cursor += yanked.len();
        }
    }

    /// Delete word backward (Ctrl+W).
    pub fn delete_word_backward(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let before = &self.buffer[..self.cursor];
        let trimmed = before.trim_end();
        let new_end = if trimmed.is_empty() {
            0
        } else {
            trimmed
                .rfind(|c: char| c.is_whitespace())
                .map(|i| i + 1)
                .unwrap_or(0)
        };
        let deleted = self.buffer[new_end..self.cursor].to_string();
        self.yank_buffer = deleted;
        self.buffer.replace_range(new_end..self.cursor, "");
        self.cursor = new_end;
    }

    /// Submit the current text, add to history, and clear. Returns submitted text.
    pub fn submit(&mut self) -> String {
        let text = std::mem::take(&mut self.buffer);
        self.cursor = 0;
        self.history_index = None;
        self.stashed.clear();

        if !text.is_empty() {
            self.history.push(text.clone());
            if self.history.len() > MAX_HISTORY {
                self.history.remove(0);
            }
        }

        text
    }

    /// Navigate up in history. Returns true if navigated.
    pub fn history_up(&mut self) -> bool {
        if self.history.is_empty() {
            return false;
        }
        match self.history_index {
            None => {
                // First press — stash current input, show most recent
                self.stashed = self.buffer.clone();
                let idx = self.history.len() - 1;
                self.history_index = Some(idx);
                self.buffer = self.history[idx].clone();
                self.cursor = self.buffer.len();
                true
            }
            Some(idx) if idx > 0 => {
                let new_idx = idx - 1;
                self.history_index = Some(new_idx);
                self.buffer = self.history[new_idx].clone();
                self.cursor = self.buffer.len();
                true
            }
            _ => false, // at oldest entry
        }
    }

    /// Navigate down in history. Returns true if navigated.
    pub fn history_down(&mut self) -> bool {
        match self.history_index {
            Some(idx) if idx + 1 < self.history.len() => {
                let new_idx = idx + 1;
                self.history_index = Some(new_idx);
                self.buffer = self.history[new_idx].clone();
                self.cursor = self.buffer.len();
                true
            }
            Some(_) => {
                // Past most recent — restore stashed input
                self.history_index = None;
                self.buffer = std::mem::take(&mut self.stashed);
                self.cursor = self.buffer.len();
                true
            }
            None => false, // not in history mode
        }
    }

    /// Enter reverse search mode (Ctrl+R).
    pub fn start_search(&mut self) {
        self.search.start();
        self.stashed = self.buffer.clone();
    }

    /// Whether in search mode.
    pub fn is_searching(&self) -> bool {
        self.search.active
    }

    /// Get the search prompt text for display.
    pub fn search_prompt(&self) -> String {
        let match_text = self.search.match_index
            .and_then(|i| self.history.get(i))
            .map(|s| s.as_str())
            .unwrap_or("");
        self.search.prompt(match_text)
    }

    /// Add a character to the search query and update match.
    pub fn search_insert(&mut self, ch: char) {
        self.search.insert(ch);
        self.search.match_index = self.search.find_match(&self.history, None);
        if let Some(idx) = self.search.match_index {
            self.buffer = self.history[idx].clone();
            self.cursor = self.buffer.len();
        }
    }

    /// Remove last character from search query.
    pub fn search_backspace(&mut self) {
        self.search.backspace();
        self.search.match_index = self.search.find_match(&self.history, None);
        if let Some(idx) = self.search.match_index {
            self.buffer = self.history[idx].clone();
            self.cursor = self.buffer.len();
        }
    }

    /// Cycle to next match (Ctrl+R again while searching).
    pub fn search_next(&mut self) {
        let from = self.search.match_index.map(|i| i.wrapping_sub(1));
        self.search.match_index = self.search.find_match(&self.history, from);
        if let Some(idx) = self.search.match_index {
            self.buffer = self.history[idx].clone();
            self.cursor = self.buffer.len();
        }
    }

    /// Accept the current search match and exit search mode.
    pub fn search_accept(&mut self) {
        if let Some(idx) = self.search.match_index {
            self.buffer = self.history[idx].clone();
            self.cursor = self.buffer.len();
        }
        self.search.stop();
    }

    /// Cancel search and restore original input.
    pub fn search_cancel(&mut self) {
        self.buffer = std::mem::take(&mut self.stashed);
        self.cursor = self.buffer.len();
        self.search.stop();
    }

    /// Clear the buffer without returning the text.
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.cursor = 0;
    }
}

impl Default for InputBox {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_navigation() {
        let mut input = InputBox::new();
        input.buffer = "first".into();
        input.submit();
        input.buffer = "second".into();
        input.submit();
        input.buffer = "current".into();

        assert!(input.history_up()); // shows "second"
        assert_eq!(input.text(), "second");
        assert!(input.history_up()); // shows "first"
        assert_eq!(input.text(), "first");
        assert!(input.history_down()); // back to "second"
        assert_eq!(input.text(), "second");
        assert!(input.history_down()); // back to stashed "current"
        assert_eq!(input.text(), "current");
    }

    #[test]
    fn kill_and_yank() {
        let mut input = InputBox::new();
        input.buffer = "hello world".into();
        input.cursor = 5;
        input.kill_to_end();
        assert_eq!(input.text(), "hello");
        assert_eq!(input.yank_buffer, " world");
        input.yank();
        assert_eq!(input.text(), "hello world");
    }

    #[test]
    fn delete_word_backward() {
        let mut input = InputBox::new();
        input.buffer = "hello world foo".into();
        input.cursor = input.buffer.len();
        input.delete_word_backward();
        assert_eq!(input.text(), "hello world ");
    }

    #[test]
    fn word_movement() {
        let mut input = InputBox::new();
        input.buffer = "hello world foo".into();
        input.cursor = 0;
        input.move_word_forward();
        assert_eq!(input.cursor, 6); // after "hello "
        input.move_word_forward();
        assert_eq!(input.cursor, 12); // after "world "
        input.move_word_backward();
        assert_eq!(input.cursor, 6); // back to "world"
    }
}
