//! Memory-related helper functions — extract, format, filter, normalize facts.

/// Extract cleaned facts from raw memory JSON.
///
/// Memory sister returns: `{"count": N, "nodes": [{"content": "...", "confidence": 0.95, ...}]}`
/// Returns a list of cleaned fact strings with common prefixes stripped.
pub(crate) fn extract_memory_facts(raw: &str) -> Vec<String> {
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(raw) {
        if let Some(nodes) = parsed.get("nodes").and_then(|n| n.as_array()) {
            return nodes.iter()
                .filter_map(|node| {
                    node.get("content").and_then(|c| c.as_str()).map(|s| {
                        // Strip common prefixes
                        let cleaned = s.strip_prefix("User preference: ")
                            .or_else(|| s.strip_prefix("User decision: "))
                            .or_else(|| s.strip_prefix("User stated: "))
                            .or_else(|| s.strip_prefix("User fact: "))
                            .or_else(|| s.strip_prefix("Fact: "))
                            .unwrap_or(s);
                        // Strip "User: X\nHydra: Y" transcript format — keep only the user part
                        if let Some(user_part) = cleaned.strip_prefix("User: ") {
                            user_part.split("\nHydra:").next().unwrap_or(user_part).trim().to_string()
                        } else {
                            cleaned.to_string()
                        }
                    })
                })
                .filter(|s| !s.is_empty() && !is_error_response(s))
                .collect();
        }
    }
    // Not JSON — return as single item, but filter out error messages and raw JSON
    if !raw.is_empty() && !is_error_response(raw) && !raw.starts_with('{') {
        vec![raw.to_string()]
    } else {
        vec![]
    }
}

/// Check if a raw response is an error message rather than actual memory content.
fn is_error_response(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("invalid params") || lower.contains("missing field")
        || lower.contains("error:") || lower.contains("not found")
        || lower.starts_with("error") || lower.contains("tool_not_found")
}

/// Format memory recall through a micro-LLM call for natural, conversational response.
///
/// Instead of parroting raw facts ("My favorite database is PostgreSQL"),
/// Hydra responds as someone who KNOWS the user ("PostgreSQL — solid choice
/// for what you're building.").
pub(crate) async fn format_memory_recall_naturally(
    query: &str,
    facts: &[String],
    user_name: &str,
    llm_config: &hydra_model::LlmConfig,
    model: &str,
) -> String {
    if facts.is_empty() {
        return "I don't have anything stored about that.".into();
    }

    let facts_text = facts.join("\n");

    // Build a tiny prompt (~100 output tokens) to format the recall naturally
    let system = format!(
        "You are recalling facts you know about the user{}. \
         Respond naturally as someone who KNOWS them — like a trusted partner, not a database. \
         Rules:\n\
         - NEVER parrot the raw fact back. Don't say \"Your favorite X is Y\" robotically.\n\
         - Show you REMEMBER — weave the fact into a warm, brief response.\n\
         - The facts belong to THE USER, not to you. Never say \"My favorite...\".\n\
         - Match their vibe: if they're technical, be technical. If casual, be casual.\n\
         - Keep it to 1-2 sentences. Be warm, direct, personal.\n\
         - If relevant, offer to help with something related.\n\
         - If multiple facts, naturally weave them together.\n\
         - If any fact says 'my X' or 'I am' or 'User\\'s X', rewrite as 'your X' or 'you are' in your response.\n\n\
         Examples of GOOD responses:\n\
         Query: \"what's my favorite database\" | Fact: \"PostgreSQL\"\n\
         → \"PostgreSQL — you've been solid on that. Want me to set up a new one?\"\n\n\
         Query: \"what languages do I know\" | Fact: \"Rust, Python, TypeScript\"\n\
         → \"Rust is your main thing, plus Python and TypeScript. Need help with any of them?\"\n\n\
         Query: \"what am I working on\" | Fact: \"Building Hydra AI orchestrator\"\n\
         → \"Hydra — the AI orchestrator. What's the next piece you want to tackle?\"",
        if user_name.is_empty() { String::new() } else { format!(" ({})", user_name) }
    );

    let user_message = format!("User asked: \"{}\"\nFacts I know: {}", query, facts_text);

    let request = hydra_model::CompletionRequest {
        model: model.to_string(),
        messages: vec![hydra_model::providers::Message {
            role: "user".into(),
            content: user_message,
        }],
        max_tokens: 150,
        temperature: Some(0.7),
        system: Some(system),
    };

    // Use the cheapest available model for this tiny formatting call
    let result = if llm_config.anthropic_api_key.is_some() {
        match hydra_model::providers::anthropic::AnthropicClient::new(llm_config) {
            Ok(client) => match client.complete(request).await {
                Ok(r) => Some(r),
                Err(e) => { eprintln!("[hydra:recall:format] Anthropic LLM failed: {}", e); None }
            },
            Err(e) => { eprintln!("[hydra:recall:format] Anthropic client init failed: {}", e); None }
        }
    } else if llm_config.openai_api_key.is_some() {
        match hydra_model::providers::openai::OpenAiClient::new(llm_config) {
            Ok(client) => match client.complete(request).await {
                Ok(r) => Some(r),
                Err(e) => { eprintln!("[hydra:recall:format] OpenAI LLM failed: {}", e); None }
            },
            Err(e) => { eprintln!("[hydra:recall:format] OpenAI client init failed: {}", e); None }
        }
    } else {
        eprintln!("[hydra:recall:format] No API key available for formatting call");
        None
    };

    if let Some(resp) = result {
        if !resp.content.trim().is_empty() {
            return resp.content.trim().to_string();
        }
    }

    // Fallback: if LLM call fails, format locally (better than raw dump)
    format_memory_fallback(facts)
}

/// Local fallback formatting when LLM is unavailable — still conversational, not robotic.
pub(crate) fn format_memory_fallback(facts: &[String]) -> String {
    // Filter out noise: JSON, garbage dates, questions, very long entries
    let clean: Vec<&str> = facts.iter()
        .map(|f| f.as_str())
        .filter(|f| !f.starts_with('{') && !f.contains("\"nodes\"") && f.len() < 200)
        .filter(|f| !f.ends_with('?'))  // Don't recall questions as facts
        .filter(|f| f.len() >= 2)  // Skip empty/tiny noise
        .collect();
    if clean.is_empty() {
        return "I don't have anything stored about that yet.".into();
    }
    if clean.len() == 1 {
        format!("{} — that's what I remember.", clean[0])
    } else {
        let items: Vec<String> = clean.iter().take(5).map(|f| format!("**{}**", f)).collect();
        format!("Here's what I remember: {}. Want to dig into any of these?", items.join(", "))
    }
}

/// Simple hash for receipt chain (non-cryptographic, for audit trail integrity)
pub(crate) fn md5_simple(input: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}

/// Phase 2, Bug Fix 0B: Extract the topic from a memory recall question.
/// "what is my favorite color?" → "favorite color"
/// "do you remember my database preference?" → "database preference"
pub(crate) fn extract_memory_topic(input: &str) -> String {
    let lower = input.to_lowercase();
    let prefixes = [
        "what is my ", "what's my ", "whats my ", "what are my ",
        "do you remember my ", "do you remember ", "remind me about my ",
        "remind me about ", "remind me of my ", "remind me of ",
        "what did i say about ", "what did i tell you about ",
        "tell me about my ", "what about my ",
    ];
    for prefix in &prefixes {
        if let Some(rest) = lower.strip_prefix(prefix) {
            return rest.trim_end_matches('?').trim_end_matches('!').trim().to_string();
        }
    }
    // Fallback: remove common question words and return remainder
    lower.replace("what", "").replace("is", "").replace("my", "")
        .replace("the", "").replace("?", "").replace("!", "")
        .split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Phase 2, Bug Fix 0B: Filter memory facts to only those relevant to the topic.
pub(crate) fn filter_facts_by_relevance(facts: &[String], topic: &str) -> Vec<String> {
    let topic_words: Vec<&str> = topic.split_whitespace()
        .filter(|w| w.len() >= 3)
        .collect();
    if topic_words.is_empty() {
        return facts.to_vec();
    }
    let filtered: Vec<String> = facts.iter()
        .filter(|f| {
            let lower = f.to_lowercase();
            topic_words.iter().any(|w| lower.contains(w))
        })
        .cloned()
        .collect();
    // If filtering removed everything, return all facts (better than nothing)
    if filtered.is_empty() { facts.to_vec() } else { filtered }
}

/// Phase 2, Bug Fix 0A: Normalize pronouns for memory storage.
/// "my favorite color is blue" → "User's favorite color is blue"
/// "I prefer PostgreSQL" → "User prefers PostgreSQL"
pub(crate) fn normalize_memory_for_storage(input: &str) -> String {
    let trimmed = input.trim();

    // Common patterns with "my X is Y"
    if let Some(rest) = trimmed.strip_prefix("my ").or_else(|| trimmed.strip_prefix("My ")) {
        return format!("User's {}", rest);
    }

    // "I am X" → "User is X"
    if let Some(rest) = trimmed.strip_prefix("I am ").or_else(|| trimmed.strip_prefix("i am ")) {
        return format!("User is {}", rest);
    }

    // "I'm X" → "User is X"
    if let Some(rest) = trimmed.strip_prefix("I'm ").or_else(|| trimmed.strip_prefix("i'm ")) {
        return format!("User is {}", rest);
    }

    // "I like X" → "User likes X"
    if let Some(rest) = trimmed.strip_prefix("I like ").or_else(|| trimmed.strip_prefix("i like ")) {
        return format!("User likes {}", rest);
    }

    // "I prefer X" → "User prefers X"
    if let Some(rest) = trimmed.strip_prefix("I prefer ").or_else(|| trimmed.strip_prefix("i prefer ")) {
        return format!("User prefers {}", rest);
    }

    // "I use X" → "User uses X"
    if let Some(rest) = trimmed.strip_prefix("I use ").or_else(|| trimmed.strip_prefix("i use ")) {
        return format!("User uses {}", rest);
    }

    // "I work on X" → "User works on X"
    if let Some(rest) = trimmed.strip_prefix("I work on ").or_else(|| trimmed.strip_prefix("i work on ")) {
        return format!("User works on {}", rest);
    }

    // "I work at X" → "User works at X"
    if let Some(rest) = trimmed.strip_prefix("I work at ").or_else(|| trimmed.strip_prefix("i work at ")) {
        return format!("User works at {}", rest);
    }

    // If no pattern matches, prefix with "User stated: "
    // unless it already starts with a third-person reference
    if trimmed.starts_with("User") || trimmed.starts_with("user") {
        trimmed.to_string()
    } else {
        trimmed.to_string()
    }
}

/// Extract the subject of a belief statement from user text.
pub(crate) fn extract_belief_subject(text: &str, trigger: &str) -> String {
    let lower = text.to_lowercase();
    if let Some(pos) = lower.find(trigger) {
        let after = text[pos + trigger.len()..].trim();
        let subject = after.split_whitespace().take(5).collect::<Vec<_>>().join("_");
        subject.to_lowercase()
    } else {
        "general".to_string()
    }
}
