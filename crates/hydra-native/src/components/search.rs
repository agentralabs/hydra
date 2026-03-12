//! Search overlay — Cmd+F search within conversation messages.

use serde::{Deserialize, Serialize};

/// A search match within a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMatch {
    pub message_index: usize,
    pub start: usize,
    pub end: usize,
}

/// The search overlay state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOverlay {
    pub query: String,
    pub matches: Vec<SearchMatch>,
    pub current_match: usize,
    pub visible: bool,
}

impl SearchOverlay {
    /// Create a new hidden search overlay.
    pub fn new() -> Self {
        Self {
            query: String::new(),
            matches: Vec::new(),
            current_match: 0,
            visible: false,
        }
    }

    /// Show the search overlay.
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// Hide and reset.
    pub fn hide(&mut self) {
        self.visible = false;
        self.query.clear();
        self.matches.clear();
        self.current_match = 0;
    }

    /// Update search query and recompute matches against messages.
    pub fn search(&mut self, query: &str, messages: &[(String, String, String)]) {
        self.query = query.to_string();
        self.matches.clear();
        self.current_match = 0;

        if query.is_empty() {
            return;
        }

        let query_lower = query.to_lowercase();
        for (i, (_role, content, _css)) in messages.iter().enumerate() {
            let content_lower = content.to_lowercase();
            let mut start = 0;
            while let Some(pos) = content_lower[start..].find(&query_lower) {
                let abs_pos = start + pos;
                self.matches.push(SearchMatch {
                    message_index: i,
                    start: abs_pos,
                    end: abs_pos + query.len(),
                });
                start = abs_pos + 1;
            }
        }
    }

    /// Move to the next match.
    pub fn next_match(&mut self) {
        if !self.matches.is_empty() {
            self.current_match = (self.current_match + 1) % self.matches.len();
        }
    }

    /// Move to the previous match.
    pub fn prev_match(&mut self) {
        if !self.matches.is_empty() {
            if self.current_match == 0 {
                self.current_match = self.matches.len() - 1;
            } else {
                self.current_match -= 1;
            }
        }
    }

    /// Total match count.
    pub fn match_count(&self) -> usize {
        self.matches.len()
    }

    /// Display string for match count (e.g. "3/12").
    pub fn match_display(&self) -> String {
        if self.matches.is_empty() {
            if self.query.is_empty() { String::new() } else { "0 results".into() }
        } else {
            format!("{}/{}", self.current_match + 1, self.matches.len())
        }
    }

    /// Get the message index of the current match (for scrolling).
    pub fn current_message_index(&self) -> Option<usize> {
        self.matches.get(self.current_match).map(|m| m.message_index)
    }
}

impl Default for SearchOverlay {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_messages() -> Vec<(String, String, String)> {
        vec![
            ("user".into(), "Hello Hydra, how are you?".into(), "msg".into()),
            ("hydra".into(), "I'm doing well! How can I help?".into(), "msg".into()),
            ("user".into(), "Can you help me with Rust?".into(), "msg".into()),
        ]
    }

    #[test]
    fn test_search_overlay_creation() {
        let overlay = SearchOverlay::new();
        assert!(!overlay.visible);
        assert!(overlay.query.is_empty());
    }

    #[test]
    fn test_show_hide() {
        let mut overlay = SearchOverlay::new();
        overlay.show();
        assert!(overlay.visible);
        overlay.hide();
        assert!(!overlay.visible);
        assert!(overlay.query.is_empty());
    }

    #[test]
    fn test_search_finds_matches() {
        let mut overlay = SearchOverlay::new();
        let messages = sample_messages();
        overlay.search("help", &messages);
        assert_eq!(overlay.match_count(), 2); // "help" in messages 1 and 2
    }

    #[test]
    fn test_search_case_insensitive() {
        let mut overlay = SearchOverlay::new();
        let messages = sample_messages();
        overlay.search("HYDRA", &messages);
        assert_eq!(overlay.match_count(), 1);
    }

    #[test]
    fn test_search_empty_query() {
        let mut overlay = SearchOverlay::new();
        let messages = sample_messages();
        overlay.search("", &messages);
        assert_eq!(overlay.match_count(), 0);
    }

    #[test]
    fn test_navigation() {
        let mut overlay = SearchOverlay::new();
        let messages = sample_messages();
        overlay.search("help", &messages);
        assert_eq!(overlay.current_match, 0);
        overlay.next_match();
        assert_eq!(overlay.current_match, 1);
        overlay.next_match(); // Wraps around
        assert_eq!(overlay.current_match, 0);
        overlay.prev_match(); // Wraps to end
        assert_eq!(overlay.current_match, 1);
    }

    #[test]
    fn test_match_display() {
        let mut overlay = SearchOverlay::new();
        let messages = sample_messages();
        assert_eq!(overlay.match_display(), "");
        overlay.search("help", &messages);
        assert_eq!(overlay.match_display(), "1/2");
        overlay.next_match();
        assert_eq!(overlay.match_display(), "2/2");
    }

    #[test]
    fn test_no_matches() {
        let mut overlay = SearchOverlay::new();
        let messages = sample_messages();
        overlay.search("nonexistent", &messages);
        assert_eq!(overlay.match_count(), 0);
        assert_eq!(overlay.match_display(), "0 results");
    }

    #[test]
    fn test_current_message_index() {
        let mut overlay = SearchOverlay::new();
        let messages = sample_messages();
        overlay.search("help", &messages);
        assert!(overlay.current_message_index().is_some());
    }
}
