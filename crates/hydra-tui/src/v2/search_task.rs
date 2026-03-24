//! Web search — live information retrieval. No API keys required.
//! Priority: Brave (if key) → DuckDuckGo HTML scrape → DDG instant answers.

use tokio::sync::mpsc;

use crate::stream::ConversationStream;
use crate::stream_types::StreamItem;
use crate::v2::search_parse;

/// A single search result.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

/// Updates from a web search.
#[derive(Debug, Clone)]
pub enum SearchUpdate {
    Results(Vec<SearchResult>),
    Error(String),
}

/// Spawn a web search. Returns a receiver for results.
pub fn spawn(rt: &tokio::runtime::Runtime, query: String) -> mpsc::Receiver<SearchUpdate> {
    let (tx, rx) = mpsc::channel(8);
    rt.spawn(async move { run(query, tx).await });
    rx
}

async fn run(query: String, tx: mpsc::Sender<SearchUpdate>) {
    // Try Brave Search API first (if key available)
    if let Ok(key) = std::env::var("BRAVE_API_KEY") {
        if let Ok(results) = brave_search(&query, &key).await {
            if !results.is_empty() {
                let _ = tx.send(SearchUpdate::Results(results)).await;
                return;
            }
        }
    }
    // Primary free: DuckDuckGo HTML scrape (real results, no API key)
    if let Ok(results) = ddg_html_search(&query).await {
        if !results.is_empty() {
            let _ = tx.send(SearchUpdate::Results(results)).await;
            return;
        }
    }
    // Last resort: DDG instant answers
    match ddg_api_search(&query).await {
        Ok(results) => { let _ = tx.send(SearchUpdate::Results(results)).await; }
        Err(e) => { let _ = tx.send(SearchUpdate::Error(format!("Search failed: {e}"))).await; }
    }
}

async fn brave_search(query: &str, api_key: &str) -> Result<Vec<SearchResult>, String> {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://api.search.brave.com/res/v1/web/search")
        .header("X-Subscription-Token", api_key)
        .header("Accept", "application/json")
        .query(&[("q", query), ("count", "5")])
        .send().await.map_err(|e| format!("{e}"))?;
    if !resp.status().is_success() { return Err(format!("API {}", resp.status())); }
    let body: serde_json::Value = resp.json().await.map_err(|e| format!("{e}"))?;
    Ok(parse_brave_json(&body))
}

async fn ddg_html_search(query: &str) -> Result<Vec<SearchResult>, String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
        .build().map_err(|e| format!("{e}"))?;
    let resp = client.get("https://html.duckduckgo.com/html/")
        .query(&[("q", query)]).send().await.map_err(|e| format!("{e}"))?;
    let html = resp.text().await.map_err(|e| format!("{e}"))?;
    Ok(search_parse::parse_ddg_html(&html))
}

async fn ddg_api_search(query: &str) -> Result<Vec<SearchResult>, String> {
    let client = reqwest::Client::new();
    let resp = client.get("https://api.duckduckgo.com/")
        .query(&[("q", query), ("format", "json"), ("no_redirect", "1")])
        .send().await.map_err(|e| format!("{e}"))?;
    let body: serde_json::Value = resp.json().await.map_err(|e| format!("{e}"))?;
    let mut results = Vec::new();
    if let Some(text) = body.get("AbstractText").and_then(|t| t.as_str()) {
        if !text.is_empty() {
            results.push(SearchResult {
                title: body.get("Heading").and_then(|h| h.as_str()).unwrap_or("Result").into(),
                url: body.get("AbstractURL").and_then(|u| u.as_str()).unwrap_or("").into(),
                snippet: text.into(),
            });
        }
    }
    if let Some(topics) = body.get("RelatedTopics").and_then(|t| t.as_array()) {
        for topic in topics.iter().take(4) {
            if let (Some(text), Some(url)) = (
                topic.get("Text").and_then(|t| t.as_str()),
                topic.get("FirstURL").and_then(|u| u.as_str()),
            ) {
                results.push(SearchResult {
                    title: text.chars().take(80).collect(), url: url.into(), snippet: text.into(),
                });
            }
        }
    }
    if results.is_empty() { Err("No results".into()) } else { Ok(results) }
}

fn parse_brave_json(body: &serde_json::Value) -> Vec<SearchResult> {
    body.get("web").and_then(|w| w.get("results")).and_then(|r| r.as_array())
        .map(|arr| arr.iter().take(5).filter_map(|item| {
            Some(SearchResult {
                title: item.get("title")?.as_str()?.into(),
                url: item.get("url")?.as_str()?.into(),
                snippet: item.get("description").and_then(|d| d.as_str()).unwrap_or("").into(),
            })
        }).collect()).unwrap_or_default()
}

/// Drain search updates into the conversation stream. Returns true when done.
pub fn drain_search(rx: &mut mpsc::Receiver<SearchUpdate>, stream: &mut ConversationStream) -> bool {
    while let Ok(update) = rx.try_recv() {
        match update {
            SearchUpdate::Results(results) => {
                if results.is_empty() {
                    stream.push(StreamItem::SystemNotification {
                        id: uuid::Uuid::new_v4(), content: "No search results found".into(),
                        timestamp: chrono::Utc::now(),
                    });
                } else {
                    stream.push(StreamItem::AssistantText {
                        id: uuid::Uuid::new_v4(),
                        text: search_parse::format_results(&results),
                        timestamp: chrono::Utc::now(),
                    });
                }
                stream.scroll_to_bottom();
                return true;
            }
            SearchUpdate::Error(e) => {
                stream.push(StreamItem::SystemNotification {
                    id: uuid::Uuid::new_v4(), content: format!("Search: {e}"),
                    timestamp: chrono::Utc::now(),
                });
                stream.scroll_to_bottom();
                return true;
            }
        }
    }
    false
}

/// Synchronous web search for slash command handlers. No API keys required.
pub fn search_blocking(query: &str) -> Result<String, String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
        .build().map_err(|e| format!("{e}"))?;
    // Try Brave if key available
    if let Ok(api_key) = std::env::var("BRAVE_API_KEY") {
        if let Ok(resp) = client
            .get("https://api.search.brave.com/res/v1/web/search")
            .header("X-Subscription-Token", &api_key)
            .header("Accept", "application/json")
            .query(&[("q", query), ("count", "5")]).send()
        {
            if resp.status().is_success() {
                if let Ok(body) = resp.json::<serde_json::Value>() {
                    let r = parse_brave_json(&body);
                    if !r.is_empty() { return Ok(search_parse::format_results(&r)); }
                }
            }
        }
    }
    // Primary free: DDG HTML scrape
    if let Ok(resp) = client.get("https://html.duckduckgo.com/html/").query(&[("q", query)]).send() {
        if let Ok(html) = resp.text() {
            let r = search_parse::parse_ddg_html(&html);
            if !r.is_empty() { return Ok(search_parse::format_results(&r)); }
        }
    }
    Err("No results found".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drain_results_formats_correctly() {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let (tx, mut rx) = mpsc::channel(8);
        rt.block_on(async {
            tx.send(SearchUpdate::Results(vec![SearchResult {
                title: "Test".into(), url: "https://example.com".into(), snippet: "A test".into(),
            }])).await.unwrap();
        });
        let mut stream = crate::stream::ConversationStream::new();
        assert!(drain_search(&mut rx, &mut stream));
    }
}
