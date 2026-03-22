//! Input box — text entry with cursor and editing.

/// A text input box with cursor support.
#[derive(Debug, Clone)]
pub struct InputBox {
    /// The current text content.
    buffer: String,
    /// Cursor position (byte offset in the buffer).
    cursor: usize,
}

impl InputBox {
    /// Create a new empty input box.
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            cursor: 0,
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
            // Find the previous character boundary.
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

    /// Move cursor to the beginning.
    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    /// Move cursor to the end.
    pub fn move_end(&mut self) {
        self.cursor = self.buffer.len();
    }

    /// Submit the current text and clear the buffer. Returns the submitted text.
    pub fn submit(&mut self) -> String {
        let text = std::mem::take(&mut self.buffer);
        self.cursor = 0;
        text
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
