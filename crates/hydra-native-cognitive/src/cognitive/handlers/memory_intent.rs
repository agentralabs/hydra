//! Smart memory intent classification and intent-aware recall.
//!
//! Instead of always calling memory_query with raw user text,
//! classify what the user is asking for and pick the right memory tool.

use crate::sisters::SistersHandle;
use crate::sisters::connection::extract_text;
use super::memory::extract_memory_facts;

/// What kind of memory query the user is making.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum MemoryIntent {
    /// "what do you know about me" / "remember anything about me"
    AboutMe,
    /// "what did we talk about" / "last time" / "yesterday"
    RecentConversation,
    /// "what did I say about databases" / "remember the auth bug"
    SpecificTopic(String),
    /// Default: top relevant memories for context
    General,
}

/// Classify user input into a memory intent — simple pattern matching, no LLM.
pub(crate) fn classify_memory_intent(input: &str) -> MemoryIntent {
    let lower = input.to_lowercase();

    // AboutMe patterns
    if lower.contains("about me")
        || lower.contains("know about me")
        || lower.contains("remember about me")
        || lower.contains("what do you know")
        || lower.contains("tell me about myself")
        || lower.contains("what have you learned")
    {
        return MemoryIntent::AboutMe;
    }

    // RecentConversation patterns
    if lower.contains("last time")
        || lower.contains("we talked")
        || lower.contains("we chatted")
        || lower.contains("we discussed")
        || lower.contains("yesterday")
        || lower.contains("earlier today")
        || lower.contains("last session")
        || lower.contains("previous conversation")
        || lower.contains("we were talking about")
        || lower.contains("last week")
        || lower.contains("recently")
    {
        return MemoryIntent::RecentConversation;
    }

    // SpecificTopic patterns — "about X", "regarding X", "the X bug"
    let topic_prefixes = [
        "what did i say about ",
        "what did i tell you about ",
        "do you remember ",
        "remind me about ",
        "what about ",
        "regarding ",
        "what do you know about ",
        "tell me about ",
    ];
    for prefix in &topic_prefixes {
        if let Some(rest) = lower.strip_prefix(prefix) {
            let topic = rest.trim_end_matches('?').trim().to_string();
            if !topic.is_empty() && topic != "me" {
                return MemoryIntent::SpecificTopic(topic);
            }
        }
    }

    // Also catch mid-sentence "about X" patterns
    if let Some(pos) = lower.find(" about ") {
        let after = &lower[pos + 7..];
        let topic = after.trim_end_matches('?').trim();
        if !topic.is_empty()
            && topic != "me"
            && topic != "myself"
            && topic.len() > 2
        {
            return MemoryIntent::SpecificTopic(topic.to_string());
        }
    }

    MemoryIntent::General
}

/// Smart memory recall — picks the right memory tool based on intent.
///
/// Returns (facts_string, tool_used) for logging.
pub(crate) async fn smart_memory_recall(
    text: &str,
    sisters_handle: &SistersHandle,
    is_simple: bool,
) -> Option<String> {
    let mem = sisters_handle.memory.as_ref()?;
    let intent = classify_memory_intent(text);
    let mem_limit = if is_simple { 5 } else { 10 };

    eprintln!("[hydra:memory] Intent: {:?} for '{}'",
        intent, &text[..text.len().min(80)]);

    match intent {
        MemoryIntent::AboutMe => {
            // Broad search for user facts, preferences, decisions
            let result = mem.call_tool("memory_query", serde_json::json!({
                "query": "user preferences facts decisions identity",
                "event_types": ["fact", "correction", "decision"],
                "max_results": mem_limit,
                "sort_by": "highest_confidence"
            })).await.ok()?;
            let raw = extract_text(&result);
            if raw.is_empty() || raw.contains("No memories found") {
                return None;
            }
            let facts = extract_memory_facts(&raw);
            if facts.is_empty() { return None; }
            eprintln!("[hydra:memory] AboutMe: {} facts", facts.len());
            Some(facts.join("\n"))
        }

        MemoryIntent::RecentConversation => {
            // Try temporal recall first, then fall back to recent episodes
            let temporal = mem.call_tool("memory_temporal_recall", serde_json::json!({
                "query": text,
                "limit": mem_limit
            })).await.ok();
            let temporal_text = temporal.as_ref()
                .map(|v| extract_text(v))
                .filter(|t| !t.is_empty() && !t.contains("No memories found"));

            if let Some(t) = temporal_text {
                let facts = extract_memory_facts(&t);
                if !facts.is_empty() {
                    eprintln!("[hydra:memory] RecentConversation (temporal): {} facts", facts.len());
                    return Some(facts.join("\n"));
                }
            }

            // Fallback: query episodes
            let result = mem.call_tool("memory_query", serde_json::json!({
                "query": text,
                "event_types": ["episode"],
                "max_results": mem_limit,
                "sort_by": "most_recent"
            })).await.ok()?;
            let raw = extract_text(&result);
            if raw.is_empty() || raw.contains("No memories found") {
                return None;
            }
            let facts = extract_memory_facts(&raw);
            if facts.is_empty() { return None; }
            eprintln!("[hydra:memory] RecentConversation (episodes): {} facts", facts.len());
            Some(facts.join("\n"))
        }

        MemoryIntent::SpecificTopic(ref topic) => {
            // Use similarity search for the specific topic
            let similar = mem.call_tool("memory_similar", serde_json::json!({
                "content": topic,
                "limit": mem_limit
            })).await.ok();
            let similar_text = similar.as_ref()
                .map(|v| extract_text(v))
                .filter(|t| !t.is_empty() && !t.contains("No memories found"));

            if let Some(s) = similar_text {
                let facts = extract_memory_facts(&s);
                if !facts.is_empty() {
                    eprintln!("[hydra:memory] SpecificTopic '{}' (similar): {} facts", topic, facts.len());
                    return Some(facts.join("\n"));
                }
            }

            // Fallback: regular query with the topic
            let result = mem.call_tool("memory_query", serde_json::json!({
                "query": topic,
                "max_results": mem_limit,
                "sort_by": "highest_confidence"
            })).await.ok()?;
            let raw = extract_text(&result);
            if raw.is_empty() || raw.contains("No memories found") {
                return None;
            }
            let facts = extract_memory_facts(&raw);
            if facts.is_empty() { return None; }
            eprintln!("[hydra:memory] SpecificTopic '{}' (query): {} facts", topic, facts.len());
            Some(facts.join("\n"))
        }

        MemoryIntent::General => {
            // Default: standard memory_query with relevance sorting
            let result = mem.call_tool("memory_query", serde_json::json!({
                "query": text,
                "max_results": if is_simple { 3 } else { 8 },
                "sort_by": "highest_confidence"
            })).await.ok()?;
            let raw = extract_text(&result);
            if raw.is_empty() || raw.contains("No memories found") {
                return None;
            }
            let facts = extract_memory_facts(&raw);
            if facts.is_empty() { return None; }

            // Sort by relevance to input
            let input_lower = text.to_lowercase();
            let input_words: Vec<&str> = input_lower.split_whitespace()
                .filter(|w| w.len() >= 3).collect();
            let mut scored: Vec<(usize, &String)> = facts.iter()
                .map(|f| {
                    let fl = f.to_lowercase();
                    let score = input_words.iter()
                        .filter(|w| fl.contains(*w)).count();
                    (score, f)
                }).collect();
            scored.sort_by(|a, b| b.0.cmp(&a.0));
            let sorted: Vec<String> = scored.iter()
                .map(|(_, f)| (*f).clone()).collect();
            eprintln!("[hydra:memory] General: {} facts (sorted)", sorted.len());
            Some(sorted.join("\n"))
        }
    }
}

/// Check if a user message is a question (should NOT be stored as a memory).
pub(crate) fn is_question(text: &str) -> bool {
    let trimmed = text.trim();
    let lower = trimmed.to_lowercase();

    trimmed.ends_with('?')
        || lower.starts_with("do you")
        || lower.starts_with("can you")
        || lower.starts_with("what ")
        || lower.starts_with("how ")
        || lower.starts_with("why ")
        || lower.starts_with("when ")
        || lower.starts_with("where ")
        || lower.starts_with("who ")
        || lower.starts_with("which ")
        || lower.starts_with("is ")
        || lower.starts_with("are ")
        || lower.starts_with("does ")
        || lower.starts_with("did ")
        || lower.starts_with("will ")
        || lower.starts_with("would ")
        || lower.starts_with("could ")
        || lower.starts_with("should ")
        || lower.starts_with("have you")
}

/// Check if a user message is a greeting (should NOT be stored as a memory).
pub(crate) fn is_greeting(text: &str) -> bool {
    let lower = text.trim().to_lowercase();
    let greetings = [
        "hi", "hello", "hey", "yo", "sup", "howdy", "hola",
        "good morning", "good afternoon", "good evening",
        "what's up", "whats up", "wassup",
        "hi there", "hello there", "hey there",
        "thanks", "thank you", "thx", "ty",
        "ok", "okay", "k", "sure", "yes", "no", "yep", "nope",
        "bye", "goodbye", "see ya", "later", "gotta go",
    ];
    greetings.iter().any(|g| lower == *g || lower.starts_with(&format!("{} ", g)))
}

/// Classify the event type for memory storage based on content.
pub(crate) fn classify_event_type(text: &str) -> &'static str {
    let lower = text.to_lowercase();
    if lower.contains("i prefer") || lower.contains("i like")
        || lower.contains("my favorite") || lower.contains("i always use")
        || lower.contains("i never use")
    {
        "fact"
    } else if lower.starts_with("no,") || lower.starts_with("actually,")
        || lower.contains("that's wrong") || lower.contains("i meant")
    {
        "correction"
    } else if lower.contains("let's ") || lower.contains("i decided")
        || lower.contains("we should") || lower.contains("i want to")
    {
        "decision"
    } else {
        "episode"
    }
}

/// Compute a simple hash of memory recall results for dedup detection.
pub(crate) fn hash_memory_response(facts: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    facts.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_about_me() {
        assert_eq!(classify_memory_intent("what do you know about me?"), MemoryIntent::AboutMe);
        assert_eq!(classify_memory_intent("tell me about myself"), MemoryIntent::AboutMe);
    }

    #[test]
    fn test_classify_recent() {
        assert_eq!(classify_memory_intent("what did we talk about last time?"), MemoryIntent::RecentConversation);
        assert_eq!(classify_memory_intent("what were we discussing yesterday"), MemoryIntent::RecentConversation);
    }

    #[test]
    fn test_classify_specific_topic() {
        assert_eq!(classify_memory_intent("do you remember the auth bug?"),
            MemoryIntent::SpecificTopic("the auth bug".to_string()));
        assert_eq!(classify_memory_intent("what did I say about databases?"),
            MemoryIntent::SpecificTopic("databases".to_string()));
    }

    #[test]
    fn test_classify_general() {
        assert_eq!(classify_memory_intent("help me with this code"), MemoryIntent::General);
        assert_eq!(classify_memory_intent("write a function"), MemoryIntent::General);
    }

    #[test]
    fn test_is_question() {
        assert!(is_question("what is my favorite color?"));
        assert!(is_question("how do I run tests?"));
        assert!(is_question("can you help me?"));
        assert!(is_question("do you remember?"));
        assert!(!is_question("my favorite color is blue"));
        assert!(!is_question("I prefer PostgreSQL"));
    }

    #[test]
    fn test_is_greeting() {
        assert!(is_greeting("hi"));
        assert!(is_greeting("hello"));
        assert!(is_greeting("hey there"));
        assert!(is_greeting("thanks"));
        assert!(is_greeting("ok"));
        assert!(!is_greeting("help me with code"));
        assert!(!is_greeting("my database is postgres"));
    }

    #[test]
    fn test_classify_event_type() {
        assert_eq!(classify_event_type("I prefer PostgreSQL"), "fact");
        assert_eq!(classify_event_type("no, that's wrong"), "correction");
        assert_eq!(classify_event_type("let's use Rust for this"), "decision");
        assert_eq!(classify_event_type("run the tests"), "episode");
    }

    #[test]
    fn test_hash_dedup() {
        let h1 = hash_memory_response("fact1\nfact2");
        let h2 = hash_memory_response("fact1\nfact2");
        let h3 = hash_memory_response("different facts");
        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
    }

    #[test]
    fn test_about_me_not_specific_topic() {
        // "about me" should be AboutMe, not SpecificTopic
        assert_eq!(classify_memory_intent("what do you know about me"), MemoryIntent::AboutMe);
        assert_ne!(classify_memory_intent("what do you know about me"),
            MemoryIntent::SpecificTopic("me".to_string()));
    }
}
