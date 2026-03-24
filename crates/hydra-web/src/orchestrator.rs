//! SearchOrchestrator — the master brain.
//! Pipeline: cache → knowledge → fan-out → deep-fetch → rank → synthesize → store → return.
//! Every search makes Hydra smarter. Repeat queries are instant.

use std::time::Instant;

use crate::cache::SearchCache;
use crate::constants::{DEEP_FETCH_TIMEOUT_SECS, MAX_DEEP_FETCH_PAGES};
use crate::engines;
use crate::errors::WebError;
use crate::extractor;
use crate::ranker;
use crate::synthesis;
use crate::types::*;

/// The web access engine.
pub struct SearchOrchestrator {
    cache: SearchCache,
}

impl SearchOrchestrator {
    pub fn new() -> Self {
        Self { cache: SearchCache::new() }
    }

    /// Full search pipeline.
    pub async fn search(&mut self, request: WebSearchRequest) -> Result<WebSearchResponse, WebError> {
        let start = Instant::now();

        // 1. CACHE CHECK — instant if we've seen this before (semantic matching)
        if request.cache_policy == CachePolicy::Allow {
            if let Some(cached) = self.cache.check(&request.query, request.content_focus) {
                eprintln!("hydra-web: cache hit for '{}'", request.query);
                return Ok(cached);
            }
        }

        // 2. MULTI-ENGINE FAN-OUT — DDG + Wikipedia + GitHub + StackExchange in parallel
        eprintln!("hydra-web: searching '{}' across all engines", request.query);
        let mut raw_hits = engines::fan_out(&request.query, request.content_focus).await;
        if raw_hits.is_empty() {
            return Err(WebError::AllEnginesFailed { query: request.query });
        }
        let engines_used: Vec<EngineLabel> = raw_hits
            .iter()
            .map(|h| h.source_engine)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        // 3. DEEP FETCH — get actual page content for top results
        if request.deep_fetch {
            deep_fetch_top(&mut raw_hits, MAX_DEEP_FETCH_PAGES).await;
        }

        // 4. RANK — multi-signal scoring
        let mut ranked = ranker::rank(raw_hits, &request.query);
        ranked.truncate(request.max_results);

        // 5. SYNTHESIZE — optional LLM pass
        let synthesis = if request.synthesize {
            synthesis::try_synthesize(&request.query, &ranked).await
        } else { None };

        let duration_ms = start.elapsed().as_millis() as u64;

        // 6. CACHE STORE — save for future instant recall
        self.cache.store(&request.query, &ranked, synthesis.as_deref(), request.content_focus);

        eprintln!(
            "hydra-web: '{}' → {} results from {:?} in {}ms",
            request.query, ranked.len(), engines_used, duration_ms,
        );

        Ok(WebSearchResponse {
            query: request.query,
            hits: ranked,
            synthesis,
            from_cache: false,
            engines_used,
            duration_ms,
        })
    }

    /// Quick search: 10 results, deep fetch on, no synthesis.
    pub async fn quick_search(&mut self, query: &str) -> Result<WebSearchResponse, WebError> {
        self.search(WebSearchRequest::quick(query)).await
    }

    /// Deep search: with LLM synthesis.
    pub async fn deep_search(&mut self, query: &str) -> Result<WebSearchResponse, WebError> {
        self.search(WebSearchRequest::deep(query)).await
    }

    /// Synchronous search for slash command handlers.
    pub fn search_blocking(&mut self, query: &str) -> Result<String, String> {
        // Check cache first (no async needed)
        if let Some(cached) = self.cache.check(query, ContentFocus::General) {
            return Ok(cached.format_display());
        }

        // Build a blocking client for fan-out
        let client = reqwest::blocking::Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
            .timeout(std::time::Duration::from_secs(10))
            .build().map_err(|e| format!("{e}"))?;

        let mut all_hits = Vec::new();

        // DDG HTML scrape (blocking)
        if let Ok(resp) = client.get("https://html.duckduckgo.com/html/").query(&[("q", query)]).send() {
            if let Ok(html) = resp.text() { all_hits.extend(parse_ddg_blocking(&html)); }
        }

        // Wikipedia API (blocking)
        if let Ok(resp) = client.get("https://en.wikipedia.org/w/api.php")
            .query(&[("action", "query"), ("list", "search"), ("srsearch", query),
                     ("format", "json"), ("srlimit", "5"), ("srprop", "snippet")])
            .send()
        {
            if let Ok(body) = resp.json::<serde_json::Value>() {
                if let Some(results) = body.get("query").and_then(|q| q.get("search")).and_then(|s| s.as_array()) {
                    for item in results {
                        if let (Some(title), Some(snippet)) = (
                            item.get("title").and_then(|t| t.as_str()),
                            item.get("snippet").and_then(|s| s.as_str()),
                        ) {
                            all_hits.push(RawSearchHit {
                                title: title.to_string(),
                                url: format!("https://en.wikipedia.org/wiki/{}", title.replace(' ', "_")),
                                snippet: strip_tags(snippet),
                                source_engine: EngineLabel::Wikipedia,
                                fetched_content: None,
                            });
                        }
                    }
                }
            }
        }

        // StackExchange (blocking)
        if let Ok(resp) = client.get("https://api.stackexchange.com/2.3/search/advanced")
            .query(&[("order", "desc"), ("sort", "relevance"), ("q", query),
                     ("site", "stackoverflow"), ("pagesize", "5")])
            .send()
        {
            if let Ok(body) = resp.json::<serde_json::Value>() {
                if let Some(items) = body.get("items").and_then(|i| i.as_array()) {
                    for item in items {
                        if let (Some(title), Some(url)) = (
                            item.get("title").and_then(|t| t.as_str()),
                            item.get("link").and_then(|l| l.as_str()),
                        ) {
                            let score = item.get("score").and_then(|s| s.as_i64()).unwrap_or(0);
                            all_hits.push(RawSearchHit {
                                title: strip_tags(title),
                                url: url.to_string(),
                                snippet: format!("[Score {score}]"),
                                source_engine: EngineLabel::StackExchange,
                                fetched_content: None,
                            });
                        }
                    }
                }
            }
        }

        if all_hits.is_empty() {
            return Err("No results from any engine".into());
        }

        let ranked = ranker::rank(all_hits, query);
        let top: Vec<SearchHit> = ranked.into_iter().take(10).collect();

        // Try LLM synthesis
        let synthesis = synthesis::try_synthesize_blocking(query, &top);

        // Store in cache
        self.cache.store(query, &top, synthesis.as_deref(), ContentFocus::General);

        let resp = WebSearchResponse {
            query: query.to_string(), hits: top, synthesis, from_cache: false,
            engines_used: vec![EngineLabel::DuckDuckGo, EngineLabel::Wikipedia, EngineLabel::StackExchange],
            duration_ms: 0,
        };
        Ok(resp.format_display())
    }

    /// Evict stale cache entries (call periodically).
    pub fn evict_stale(&mut self) { self.cache.evict_stale(); }
}

impl Default for SearchOrchestrator {
    fn default() -> Self { Self::new() }
}

/// Deep fetch top N pages in parallel, extract content.
async fn deep_fetch_top(hits: &mut Vec<RawSearchHit>, max: usize) {
    let timeout = std::time::Duration::from_secs(DEEP_FETCH_TIMEOUT_SECS);
    let urls: Vec<(usize, String)> = hits.iter().enumerate()
        .take(max)
        .map(|(i, h)| (i, h.url.clone()))
        .collect();

    let mut tasks = Vec::new();
    for (idx, url) in urls {
        tasks.push(tokio::spawn(async move {
            let result = tokio::time::timeout(timeout, fetch_and_extract(&url)).await;
            (idx, result.ok().flatten())
        }));
    }

    for task in tasks {
        if let Ok((idx, Some(content))) = task.await {
            if idx < hits.len() {
                hits[idx].fetched_content = Some(content);
            }
        }
    }
}

async fn fetch_and_extract(url: &str) -> Option<ExtractedContent> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
        .timeout(std::time::Duration::from_secs(DEEP_FETCH_TIMEOUT_SECS))
        .build().ok()?;
    let resp = client.get(url).send().await.ok()?;
    if !resp.status().is_success() { return None; }
    let html = resp.text().await.ok()?;
    let content = extractor::extract(&html);
    if content.word_count < 20 { return None; }
    Some(content)
}

fn parse_ddg_blocking(html: &str) -> Vec<RawSearchHit> {
    let mut results = Vec::new();
    for chunk in html.split("class=\"result__body") {
        if results.len() >= 10 { break; }
        let url = extract_between(chunk, "class=\"result__a\" href=\"", "\"").map(decode_ddg);
        let title = extract_between(chunk, "class=\"result__a\"", "</a>")
            .map(|t| t.split_once('>').map(|(_, r)| r).unwrap_or(t)).map(strip_tags);
        let snippet = extract_between(chunk, "class=\"result__snippet\"", "</a>")
            .or_else(|| extract_between(chunk, "class=\"result__snippet\"", "</td>"))
            .map(|s| s.split_once('>').map(|(_, r)| r).unwrap_or(s)).map(strip_tags);
        if let (Some(url), Some(title)) = (url, title) {
            if !url.is_empty() && url.starts_with("http") {
                results.push(RawSearchHit {
                    title, url, snippet: snippet.unwrap_or_default(),
                    source_engine: EngineLabel::DuckDuckGo, fetched_content: None,
                });
            }
        }
    }
    results
}

fn extract_between<'a>(t: &'a str, s: &str, e: &str) -> Option<&'a str> {
    let start = t.find(s)? + s.len();
    let end = t[start..].find(e)? + start;
    Some(&t[start..end])
}
fn decode_ddg(url: &str) -> String {
    if let Some(u) = url.split("uddg=").nth(1) {
        url_dec(u.split('&').next().unwrap_or(u))
    } else if url.starts_with("//") { format!("https:{url}") } else { url.to_string() }
}
fn url_dec(s: &str) -> String {
    let mut r = String::new();
    let mut b = s.bytes();
    while let Some(c) = b.next() {
        match c {
            b'%' => { let h = b.next().unwrap_or(b'0'); let l = b.next().unwrap_or(b'0');
                       r.push((hx(h)*16+hx(l)) as char); }
            b'+' => r.push(' '),
            _ => r.push(c as char),
        }
    }
    r
}
fn hx(b: u8) -> u8 { match b { b'0'..=b'9'=>b-b'0', b'a'..=b'f'=>b-b'a'+10, b'A'..=b'F'=>b-b'A'+10, _=>0 } }
fn strip_tags(s: &str) -> String {
    let mut o = String::new(); let mut t = false;
    for c in s.chars() { if c=='<'{t=true;} else if c=='>'{t=false;} else if !t{o.push(c);} }
    o.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orchestrator_creates() {
        let _orch = SearchOrchestrator::new();
    }

    #[test]
    fn cache_miss_on_fresh_orchestrator() {
        let orch = SearchOrchestrator::new();
        assert!(orch.cache.check("nonexistent query xyz", ContentFocus::General).is_none());
    }
}
