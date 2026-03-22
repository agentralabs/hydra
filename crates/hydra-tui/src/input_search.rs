//! Input search — reverse history search (Ctrl+R).
//!
//! Separated from input.rs to keep files under 400 lines.

/// Search state for the input box.
#[derive(Debug, Clone, Default)]
pub struct SearchState {
    /// Whether in reverse search mode.
    pub active: bool,
    /// Current search query.
    pub query: String,
    /// Index of current match in history.
    pub match_index: Option<usize>,
}

impl SearchState {
    /// Start a new search.
    pub fn start(&mut self) {
        self.active = true;
        self.query.clear();
        self.match_index = None;
    }

    /// Stop searching.
    pub fn stop(&mut self) {
        self.active = false;
        self.query.clear();
        self.match_index = None;
    }

    /// Add a character to the search query.
    pub fn insert(&mut self, ch: char) {
        self.query.push(ch);
    }

    /// Remove last character from search query.
    pub fn backspace(&mut self) {
        self.query.pop();
    }

    /// Get formatted search prompt for display.
    pub fn prompt(&self, matched_text: &str) -> String {
        format!("(reverse-i-search)`{}': {}", self.query, matched_text)
    }

    /// Find a matching history entry searching backward.
    pub fn find_match(&self, history: &[String], from: Option<usize>) -> Option<usize> {
        if history.is_empty() || self.query.is_empty() {
            return None;
        }
        let query_lower = self.query.to_lowercase();
        let len = history.len();
        let start = from.unwrap_or(len.wrapping_sub(1));

        for offset in 0..len {
            let idx = start.wrapping_sub(offset) % len;
            if idx >= len {
                continue;
            }
            if history[idx].to_lowercase().contains(&query_lower) {
                return Some(idx);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_finds_match() {
        let mut state = SearchState::default();
        state.start();
        state.insert('c');
        state.insert('i');
        state.insert('r');

        let history = vec![
            "explain circuit breakers".into(),
            "hello world".into(),
            "what is rust".into(),
        ];
        let m = state.find_match(&history, None);
        assert_eq!(m, Some(0)); // "circuit" matches
    }

    #[test]
    fn search_no_match() {
        let mut state = SearchState::default();
        state.start();
        state.insert('z');
        state.insert('z');
        state.insert('z');

        let history = vec!["hello".into(), "world".into()];
        assert!(state.find_match(&history, None).is_none());
    }
}
