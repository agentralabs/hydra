//! Search result parsers — HTML scraping for DuckDuckGo and result formatting.
//! No API keys needed. Extracts real search results from DDG HTML page.

use crate::v2::search_task::SearchResult;

/// Parse DuckDuckGo HTML results page to extract titles, URLs, and snippets.
pub fn parse_ddg_html(html: &str) -> Vec<SearchResult> {
    let mut results = Vec::new();
    for chunk in html.split("class=\"result__body") {
        if results.len() >= 5 { break; }
        let url = extract_between(chunk, "class=\"result__a\" href=\"", "\"")
            .map(|u| decode_ddg_url(u));
        let title = extract_between(chunk, "class=\"result__a\"", "</a>")
            .map(|t| t.split_once('>').map(|(_, rest)| rest).unwrap_or(t))
            .map(strip_html_tags);
        let snippet = extract_between(chunk, "class=\"result__snippet\"", "</a>")
            .or_else(|| extract_between(chunk, "class=\"result__snippet\"", "</td>"))
            .map(|s| s.split_once('>').map(|(_, rest)| rest).unwrap_or(s))
            .map(strip_html_tags);

        if let (Some(url), Some(title)) = (url, title) {
            if !url.is_empty() && !title.is_empty() && url.starts_with("http") {
                results.push(SearchResult {
                    title,
                    url,
                    snippet: snippet.unwrap_or_default(),
                });
            }
        }
    }
    results
}

/// Format search results for display.
pub fn format_results(results: &[SearchResult]) -> String {
    results
        .iter()
        .enumerate()
        .map(|(i, r)| format!("{}. **{}**\n   {}\n   {}", i + 1, r.title, r.snippet, r.url))
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn extract_between<'a>(text: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let s = text.find(start)? + start.len();
    let e = text[s..].find(end)? + s;
    Some(&text[s..e])
}

fn decode_ddg_url(url: &str) -> String {
    if let Some(uddg) = url.split("uddg=").nth(1) {
        let encoded = uddg.split('&').next().unwrap_or(uddg);
        url_decode(encoded)
    } else if url.starts_with("//") {
        format!("https:{url}")
    } else {
        url.to_string()
    }
}

fn url_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.bytes();
    while let Some(b) = chars.next() {
        if b == b'%' {
            let hi = chars.next().unwrap_or(b'0');
            let lo = chars.next().unwrap_or(b'0');
            let val = hex_val(hi) * 16 + hex_val(lo);
            result.push(val as char);
        } else if b == b'+' {
            result.push(' ');
        } else {
            result.push(b as char);
        }
    }
    result
}

fn hex_val(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => 0,
    }
}

fn strip_html_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        if c == '<' { in_tag = true; }
        else if c == '>' { in_tag = false; }
        else if !in_tag { out.push(c); }
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_tags() {
        assert_eq!(strip_html_tags("<b>hello</b> world"), "hello world");
        assert_eq!(strip_html_tags("no tags"), "no tags");
    }

    #[test]
    fn url_decode_works() {
        assert_eq!(url_decode("hello%20world"), "hello world");
        assert_eq!(url_decode("a%2Fb"), "a/b");
    }

    #[test]
    fn ddg_url_decode() {
        let url = "//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com&rut=abc";
        assert_eq!(decode_ddg_url(url), "https://example.com");
    }
}
