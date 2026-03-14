//! Generic memory recall — no intent classification, no event_type filtering.
//!
//! RULE: Query broadly (semantic + recent), return plenty (max_results=20),
//! let the LLM decide what's relevant. No hardcoded classifiers.

use crate::sisters::SistersHandle;
use crate::sisters::connection::extract_text;
use super::memory::extract_memory_facts;

/// Generic memory recall — two queries, merged, no classification.
///
/// Query 1: Semantic search for what the user is asking about (max 20)
/// Query 2: Recent episodes for session context (max 10)
/// Merge, dedup, return all. Let the LLM pick relevance.
pub(crate) async fn smart_memory_recall(
    text: &str,
    sisters_handle: &SistersHandle,
    _is_simple: bool,
) -> Option<String> {
    let mem = sisters_handle.memory.as_ref()?;
    eprintln!("[hydra:memory] Generic recall for '{}'", &text[..text.len().min(80)]);

    // Query 1: Semantic search — no event_type filter, max 20
    let semantic_fut = async {
        match mem.call_tool("memory_query", serde_json::json!({
            "query": text, "max_results": 20
        })).await {
            Ok(v) => {
                let raw = extract_text(&v);
                eprintln!("[hydra:memory] semantic query OK: {} chars", raw.len());
                if raw.is_empty() || raw.contains("No memories found") { vec![] }
                else { extract_memory_facts(&raw) }
            }
            Err(e) => { eprintln!("[hydra:memory] semantic query FAILED: {}", e); vec![] }
        }
    };

    // Query 2: Recent episodes (temporal, last entries)
    let recent_fut = async {
        match mem.call_tool("memory_query", serde_json::json!({
            "query": text, "max_results": 10, "sort_by": "most_recent"
        })).await {
            Ok(v) => {
                let raw = extract_text(&v);
                eprintln!("[hydra:memory] recent query OK: {} chars", raw.len());
                if raw.is_empty() || raw.contains("No memories found") { vec![] }
                else { extract_memory_facts(&raw) }
            }
            Err(e) => { eprintln!("[hydra:memory] recent query FAILED: {}", e); vec![] }
        }
    };

    let (semantic, recent) = tokio::join!(semantic_fut, recent_fut);

    // Merge and dedup
    let mut merged = semantic;
    for fact in recent {
        if !merged.iter().any(|existing| existing == &fact) {
            merged.push(fact);
        }
    }

    if merged.is_empty() {
        eprintln!("[hydra:memory] No memories found from either query");
        // Last resort: try memory_similar
        if let Ok(v) = mem.call_tool("memory_similar", serde_json::json!({
            "content": text, "limit": 10
        })).await {
            let raw = extract_text(&v);
            if !raw.is_empty() && !raw.contains("No memories") {
                let facts = extract_memory_facts(&raw);
                if !facts.is_empty() {
                    eprintln!("[hydra:memory] similarity fallback: {} facts", facts.len());
                    return Some(facts.join("\n"));
                }
            }
        }
        return None;
    }

    eprintln!("[hydra:memory] Returning {} merged facts", merged.len());
    Some(merged.join("\n"))
}

/// PRE-LLM OPTIMIZATION: Classifies whether input is a question before LLM.
/// Known violation of "No Hardcoded Intelligence" rule — documented as tech debt.
/// When modifying: prefer removing patterns and letting LLM handle classification.
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

/// PRE-LLM OPTIMIZATION: Classifies whether input is a greeting before LLM.
/// Known violation of "No Hardcoded Intelligence" rule — documented as tech debt.
/// When modifying: prefer removing patterns and letting LLM handle classification.
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
    fn test_hash_dedup() {
        let h1 = hash_memory_response("fact1\nfact2");
        let h2 = hash_memory_response("fact1\nfact2");
        let h3 = hash_memory_response("different facts");
        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
    }
}
