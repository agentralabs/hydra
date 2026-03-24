//! Multi-engine search fan-out — DDG + Wikipedia + GitHub + StackExchange.
//! All free, zero API keys. Engines run concurrently with per-engine timeout.

use crate::constants::ENGINE_TIMEOUT_SECS;
use crate::types::{ContentFocus, EngineLabel, RawSearchHit};

/// Fan out to all relevant engines concurrently, deduplicate results.
pub async fn fan_out(query: &str, focus: ContentFocus) -> Vec<RawSearchHit> {
    let q = query.to_string();
    let q2 = q.clone();
    let q3 = q.clone();
    let q4 = q.clone();
    let timeout = std::time::Duration::from_secs(ENGINE_TIMEOUT_SECS);

    // Spawn all engines concurrently
    let ddg = tokio::spawn(async move {
        tokio::time::timeout(timeout, search_ddg(&q)).await.ok().flatten().unwrap_or_default()
    });
    let wiki = tokio::spawn(async move {
        tokio::time::timeout(timeout, search_wikipedia(&q2)).await.ok().flatten().unwrap_or_default()
    });
    // Prioritize engines by content focus
    let gh = if matches!(focus, ContentFocus::Code | ContentFocus::General) {
        Some(tokio::spawn(async move {
            tokio::time::timeout(timeout, search_github(&q3)).await.ok().flatten().unwrap_or_default()
        }))
    } else { None };
    let se = if matches!(focus, ContentFocus::Code | ContentFocus::General | ContentFocus::Documentation) {
        Some(tokio::spawn(async move {
            tokio::time::timeout(timeout, search_stackexchange(&q4)).await.ok().flatten().unwrap_or_default()
        }))
    } else { None };

    let mut all = Vec::new();
    if let Ok(r) = ddg.await { all.extend(r); }
    if let Ok(r) = wiki.await { all.extend(r); }
    if let Some(h) = gh { if let Ok(r) = h.await { all.extend(r); } }
    if let Some(h) = se { if let Ok(r) = h.await { all.extend(r); } }

    dedup_by_url(all)
}

fn dedup_by_url(mut hits: Vec<RawSearchHit>) -> Vec<RawSearchHit> {
    let mut seen = std::collections::HashSet::new();
    hits.retain(|h| seen.insert(h.url.clone()));
    hits
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
        .timeout(std::time::Duration::from_secs(ENGINE_TIMEOUT_SECS))
        .build()
        .unwrap_or_default()
}

// ── DuckDuckGo HTML Scrape ──

async fn search_ddg(query: &str) -> Option<Vec<RawSearchHit>> {
    let html = client()
        .get("https://html.duckduckgo.com/html/")
        .query(&[("q", query)])
        .send().await.ok()?
        .text().await.ok()?;
    Some(parse_ddg_html(&html))
}

fn parse_ddg_html(html: &str) -> Vec<RawSearchHit> {
    let mut results = Vec::new();
    for chunk in html.split("class=\"result__body") {
        if results.len() >= 10 { break; }
        let url = extract_between(chunk, "class=\"result__a\" href=\"", "\"").map(decode_ddg_url);
        let title = extract_between(chunk, "class=\"result__a\"", "</a>")
            .map(|t| t.split_once('>').map(|(_, r)| r).unwrap_or(t))
            .map(strip_tags);
        let snippet = extract_between(chunk, "class=\"result__snippet\"", "</a>")
            .or_else(|| extract_between(chunk, "class=\"result__snippet\"", "</td>"))
            .map(|s| s.split_once('>').map(|(_, r)| r).unwrap_or(s))
            .map(strip_tags);
        if let (Some(url), Some(title)) = (url, title) {
            if !url.is_empty() && !title.is_empty() && url.starts_with("http") {
                results.push(RawSearchHit {
                    title, url, snippet: snippet.unwrap_or_default(),
                    source_engine: EngineLabel::DuckDuckGo, fetched_content: None,
                });
            }
        }
    }
    results
}

// ── Wikipedia API ──

async fn search_wikipedia(query: &str) -> Option<Vec<RawSearchHit>> {
    let resp: serde_json::Value = client()
        .get("https://en.wikipedia.org/w/api.php")
        .query(&[("action", "query"), ("list", "search"), ("srsearch", query),
                 ("format", "json"), ("srlimit", "5"), ("srprop", "snippet")])
        .send().await.ok()?
        .json().await.ok()?;

    let results = resp.get("query")?.get("search")?.as_array()?;
    Some(results.iter().filter_map(|item| {
        let title = item.get("title")?.as_str()?.to_string();
        let snippet = strip_tags(item.get("snippet")?.as_str()?);
        let url = format!("https://en.wikipedia.org/wiki/{}", title.replace(' ', "_"));
        Some(RawSearchHit {
            title, url, snippet, source_engine: EngineLabel::Wikipedia, fetched_content: None,
        })
    }).collect())
}

// ── GitHub Search API ──

async fn search_github(query: &str) -> Option<Vec<RawSearchHit>> {
    let resp: serde_json::Value = client()
        .get("https://api.github.com/search/repositories")
        .header("Accept", "application/vnd.github+json")
        .query(&[("q", query), ("per_page", "5"), ("sort", "stars")])
        .send().await.ok()?
        .json().await.ok()?;

    let items = resp.get("items")?.as_array()?;
    Some(items.iter().filter_map(|item| {
        let name = item.get("full_name")?.as_str()?.to_string();
        let desc = item.get("description").and_then(|d| d.as_str()).unwrap_or("").to_string();
        let url = item.get("html_url")?.as_str()?.to_string();
        let stars = item.get("stargazers_count").and_then(|s| s.as_u64()).unwrap_or(0);
        Some(RawSearchHit {
            title: format!("{name} ({stars} stars)"),
            url, snippet: desc,
            source_engine: EngineLabel::GitHub, fetched_content: None,
        })
    }).collect())
}

// ── StackExchange API ──

async fn search_stackexchange(query: &str) -> Option<Vec<RawSearchHit>> {
    let resp: serde_json::Value = client()
        .get("https://api.stackexchange.com/2.3/search/advanced")
        .query(&[("order", "desc"), ("sort", "relevance"), ("q", query),
                 ("site", "stackoverflow"), ("pagesize", "5"), ("filter", "withbody")])
        .send().await.ok()?
        .json().await.ok()?;

    let items = resp.get("items")?.as_array()?;
    Some(items.iter().filter_map(|item| {
        let title = item.get("title")?.as_str()?.to_string();
        let title = strip_tags(&title);
        let url = item.get("link")?.as_str()?.to_string();
        let score = item.get("score").and_then(|s| s.as_i64()).unwrap_or(0);
        let answered = item.get("is_answered").and_then(|a| a.as_bool()).unwrap_or(false);
        let snippet = if answered { format!("[Answered, score {score}] {title}") }
            else { format!("[Score {score}] {title}") };
        Some(RawSearchHit {
            title, url, snippet,
            source_engine: EngineLabel::StackExchange, fetched_content: None,
        })
    }).collect())
}

// ── HTML Utilities ──

fn extract_between<'a>(text: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let s = text.find(start)? + start.len();
    let e = text[s..].find(end)? + s;
    Some(&text[s..e])
}

fn decode_ddg_url(url: &str) -> String {
    if let Some(uddg) = url.split("uddg=").nth(1) {
        url_decode(uddg.split('&').next().unwrap_or(uddg))
    } else if url.starts_with("//") {
        format!("https:{url}")
    } else { url.to_string() }
}

fn url_decode(s: &str) -> String {
    let mut r = String::with_capacity(s.len());
    let mut bytes = s.bytes();
    while let Some(b) = bytes.next() {
        match b {
            b'%' => {
                let hi = bytes.next().unwrap_or(b'0');
                let lo = bytes.next().unwrap_or(b'0');
                r.push((hex(hi) * 16 + hex(lo)) as char);
            }
            b'+' => r.push(' '),
            _ => r.push(b as char),
        }
    }
    r
}

fn hex(b: u8) -> u8 {
    match b { b'0'..=b'9' => b - b'0', b'a'..=b'f' => b - b'a' + 10, b'A'..=b'F' => b - b'A' + 10, _ => 0 }
}

fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        if c == '<' { in_tag = true; } else if c == '>' { in_tag = false; }
        else if !in_tag { out.push(c); }
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedup_removes_duplicate_urls() {
        let hits = vec![
            RawSearchHit { title: "A".into(), url: "https://a.com".into(), snippet: "".into(), source_engine: EngineLabel::DuckDuckGo, fetched_content: None },
            RawSearchHit { title: "A2".into(), url: "https://a.com".into(), snippet: "better".into(), source_engine: EngineLabel::Wikipedia, fetched_content: None },
            RawSearchHit { title: "B".into(), url: "https://b.com".into(), snippet: "".into(), source_engine: EngineLabel::GitHub, fetched_content: None },
        ];
        let deduped = dedup_by_url(hits);
        assert_eq!(deduped.len(), 2);
    }

    #[test]
    fn ddg_url_decode() {
        let url = "//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com&rut=abc";
        assert_eq!(decode_ddg_url(url), "https://example.com");
    }

    #[test]
    fn strip_tags_works() {
        assert_eq!(strip_tags("<b>hello</b> <i>world</i>"), "hello world");
    }
}
