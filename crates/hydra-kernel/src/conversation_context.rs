//! Conversation context tracker — sliding window of messages with topic extraction.

pub struct ConversationContext {
    messages: Vec<(String, String)>,
    topics: Vec<String>,
    window_size: usize,
}

impl ConversationContext {
    pub fn new(window_size: usize) -> Self {
        Self {
            messages: Vec::new(),
            topics: Vec::new(),
            window_size,
        }
    }

    /// Add a message and extract key topics from it.
    pub fn add_message(&mut self, role: &str, content: &str) {
        self.messages.push((role.to_string(), content.to_string()));
        if self.messages.len() > self.window_size {
            self.messages.remove(0);
        }
        // Extract significant words as topics
        let stop = ["the","a","an","is","are","was","i","you","we","to","of","in",
            "for","on","with","it","that","this","can","do","what","how","my"];
        for word in content.split_whitespace() {
            let clean = word.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
            if clean.len() >= 4 && !stop.contains(&clean.as_str()) {
                if !self.topics.contains(&clean) {
                    self.topics.push(clean);
                }
                if self.topics.len() > 20 { self.topics.remove(0); }
            }
        }
    }

    /// Most recent/frequent topic.
    pub fn current_topic(&self) -> Option<String> {
        self.topics.last().cloned()
    }

    /// 2-3 sentence summary for LLM prompt injection.
    pub fn context_summary(&self) -> String {
        let recent: Vec<String> = self.messages.iter()
            .rev().take(3).rev()
            .map(|(role, content)| {
                let truncated = if content.len() > 80 { &content[..80] } else { content };
                format!("{}: {}", role, truncated)
            })
            .collect();
        if recent.is_empty() {
            "No prior context.".to_string()
        } else {
            format!("Recent conversation: {}", recent.join(" | "))
        }
    }

    /// Detect if text references previous messages ("that", "it", "this", "those").
    pub fn references_previous(&self, text: &str) -> bool {
        let lower = text.to_lowercase();
        let referential = ["that", "those", "this", "these", "the same",
            "implement it", "do it", "fix it", "what you said",
            "you mentioned", "earlier", "above"];
        self.messages.len() > 1 && referential.iter().any(|r| lower.contains(r))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_topic() {
        let mut ctx = ConversationContext::new(5);
        ctx.add_message("user", "implement the uptime tracker");
        assert!(ctx.current_topic().is_some());
    }

    #[test]
    fn test_window_limit() {
        let mut ctx = ConversationContext::new(3);
        ctx.add_message("user", "first");
        ctx.add_message("hydra", "second");
        ctx.add_message("user", "third");
        ctx.add_message("hydra", "fourth");
        assert_eq!(ctx.messages.len(), 3);
    }

    #[test]
    fn test_context_summary() {
        let mut ctx = ConversationContext::new(10);
        ctx.add_message("user", "hello");
        ctx.add_message("hydra", "hi there");
        let summary = ctx.context_summary();
        assert!(summary.contains("user: hello"));
    }

    #[test]
    fn test_references_previous() {
        let mut ctx = ConversationContext::new(10);
        ctx.add_message("user", "improve error handling");
        ctx.add_message("hydra", "I can do that");
        assert!(ctx.references_previous("can you implement that?"));
        assert!(!ctx.references_previous("build a new feature"));
    }

    #[test]
    fn test_no_reference_with_empty_history() {
        let ctx = ConversationContext::new(10);
        assert!(!ctx.references_previous("do that thing"));
    }
}
