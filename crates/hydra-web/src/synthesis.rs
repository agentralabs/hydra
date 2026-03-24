//! LLM synthesis — optional Perplexity-style coherent answer with citations.
//! Degrades gracefully: returns None when no API key is set.

use crate::constants::SYNTHESIS_MAX_INPUT_CHARS;
use crate::types::SearchHit;

/// Attempt to synthesize a coherent answer from search results.
/// Returns None if no API key is available (graceful degradation).
pub async fn try_synthesize(query: &str, hits: &[SearchHit]) -> Option<String> {
    let api_key = std::env::var("ANTHROPIC_API_KEY").ok()?;
    if hits.is_empty() { return None; }

    let prompt = build_prompt(query, hits);
    call_llm(&api_key, &prompt).await
}

/// Synchronous version for command handlers.
pub fn try_synthesize_blocking(query: &str, hits: &[SearchHit]) -> Option<String> {
    let api_key = std::env::var("ANTHROPIC_API_KEY").ok()?;
    if hits.is_empty() { return None; }
    let prompt = build_prompt(query, hits);
    call_llm_blocking(&api_key, &prompt)
}

fn build_prompt(query: &str, hits: &[SearchHit]) -> String {
    let mut sources = String::new();
    let mut char_budget = SYNTHESIS_MAX_INPUT_CHARS;

    for (i, hit) in hits.iter().enumerate() {
        if char_budget == 0 { break; }
        let content_text = hit.content.as_ref()
            .map(|c| c.main_text.as_str())
            .unwrap_or(&hit.snippet);

        let entry = format!(
            "[{}] {} ({})\n{}\n\n",
            i + 1, hit.title, hit.url,
            &content_text[..content_text.len().min(char_budget)]
        );
        char_budget = char_budget.saturating_sub(entry.len());
        sources.push_str(&entry);
    }

    format!(
        "You are a research assistant. The user asked: \"{query}\"\n\n\
         Below are search results. Synthesize a coherent, well-sourced answer.\n\
         Use [N] citation markers to reference sources. Be concise but thorough.\n\
         If results are insufficient, say so.\n\n\
         SOURCES:\n{sources}"
    )
}

async fn call_llm(api_key: &str, prompt: &str) -> Option<String> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "model": "claude-haiku-4-5-20251001",
        "max_tokens": 1024,
        "messages": [{"role": "user", "content": prompt}]
    });

    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send().await.ok()?;

    if !resp.status().is_success() {
        eprintln!("hydra-web: synthesis API error: {}", resp.status());
        return None;
    }

    let parsed: serde_json::Value = resp.json().await.ok()?;
    parsed.get("content")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|block| block.get("text"))
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
}

fn call_llm_blocking(api_key: &str, prompt: &str) -> Option<String> {
    let client = reqwest::blocking::Client::new();
    let body = serde_json::json!({
        "model": "claude-haiku-4-5-20251001",
        "max_tokens": 1024,
        "messages": [{"role": "user", "content": prompt}]
    });

    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send().ok()?;

    if !resp.status().is_success() { return None; }
    let parsed: serde_json::Value = resp.json().ok()?;
    parsed.get("content")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|block| block.get("text"))
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::EngineLabel;

    #[test]
    fn build_prompt_includes_sources() {
        let hits = vec![SearchHit {
            title: "Rust Book".into(), url: "https://doc.rust-lang.org".into(),
            snippet: "Learn Rust".into(), content: None, score: 0.9,
            source: EngineLabel::DuckDuckGo, confidence: 0.65,
        }];
        let prompt = build_prompt("rust ownership", &hits);
        assert!(prompt.contains("[1] Rust Book"));
        assert!(prompt.contains("rust ownership"));
    }

    #[test]
    fn prompt_respects_char_budget() {
        let hits: Vec<SearchHit> = (0..20).map(|i| SearchHit {
            title: format!("Result {i}"), url: format!("https://example.com/{i}"),
            snippet: "x".repeat(2000), content: None, score: 0.5,
            source: EngineLabel::DuckDuckGo, confidence: 0.5,
        }).collect();
        let prompt = build_prompt("test", &hits);
        assert!(prompt.len() < SYNTHESIS_MAX_INPUT_CHARS + 500); // some overhead for frame
    }
}
