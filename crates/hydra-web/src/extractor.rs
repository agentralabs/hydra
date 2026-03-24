//! Content extraction — readability-style main body extraction from HTML.
//! Goes beyond simple tag stripping: finds the actual content area,
//! preserves code blocks and tables, extracts structured data.

use crate::constants::{MAX_CONTENT_LENGTH, MIN_CONTENT_LENGTH};
use crate::types::ExtractedContent;

/// Extract structured content from raw HTML.
pub fn extract(html: &str) -> ExtractedContent {
    let code_blocks = extract_code_blocks(html);
    let tables = extract_tables(html);
    let json_ld = extract_json_ld(html);
    let main_text = extract_main_body(html);

    let mut full_text = main_text;
    // Append structured data if found and useful
    if let Some(ld) = json_ld {
        if let Some(desc) = ld.get("description").and_then(|d| d.as_str()) {
            if !full_text.contains(desc) && desc.len() > 50 {
                full_text = format!("{full_text}\n\n{desc}");
            }
        }
    }
    let word_count = full_text.split_whitespace().count();

    ExtractedContent {
        main_text: smart_truncate(&full_text, MAX_CONTENT_LENGTH),
        code_blocks,
        tables,
        word_count,
        fetched_at: chrono::Utc::now(),
    }
}

/// Readability-style main body extraction.
/// Heuristic: strip noise elements, split by block tags, score each block
/// by text_density * length, return the highest-scoring block(s).
fn extract_main_body(html: &str) -> String {
    // Step 1: Remove noise elements (nav, footer, header, script, style, aside)
    let cleaned = remove_noise(html);

    // Step 2: Split by block-level elements and score
    let blocks = split_blocks(&cleaned);
    if blocks.is_empty() {
        // Fallback: strip all tags
        return strip_all_tags(&cleaned);
    }

    // Step 3: Score blocks by text density * absolute length
    let mut scored: Vec<(f64, &str)> = blocks
        .iter()
        .map(|block| {
            let text = strip_all_tags(block);
            let text_len = text.len() as f64;
            let total_len = block.len().max(1) as f64;
            let density = text_len / total_len;
            let score = density * text_len.sqrt(); // reward dense + long blocks
            (score, block.as_str())
        })
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    // Step 4: Take top blocks until we have enough content
    let mut result = String::new();
    for (_, block) in scored.iter().take(crate::constants::MAX_DEEP_FETCH_PAGES) {
        let text = strip_all_tags(block);
        if text.len() < 30 { continue; } // skip tiny fragments
        if !result.is_empty() { result.push_str("\n\n"); }
        result.push_str(&text);
        if result.len() >= MIN_CONTENT_LENGTH { break; }
    }

    if result.len() < MIN_CONTENT_LENGTH {
        // Fallback: just strip everything
        strip_all_tags(&cleaned)
    } else {
        result
    }
}

/// Remove nav, footer, header, script, style, aside elements.
fn remove_noise(html: &str) -> String {
    let mut result = html.to_string();
    for tag in &["script", "style", "nav", "footer", "header", "aside", "noscript", "iframe"] {
        loop {
            let open = format!("<{}", tag);
            let close = format!("</{}>", tag);
            if let Some(start) = result.to_lowercase().find(&open) {
                if let Some(end_offset) = result[start..].to_lowercase().find(&close) {
                    let end = start + end_offset + close.len();
                    result.replace_range(start..end, " ");
                    continue;
                }
            }
            break;
        }
    }
    result
}

/// Split HTML into block-level chunks.
fn split_blocks(html: &str) -> Vec<String> {
    let block_tags = ["<div", "<section", "<article", "<main", "<p>", "<p ", "<li", "<td", "<blockquote"];
    let mut blocks = Vec::new();
    let lower = html.to_lowercase();
    let mut last = 0;
    for tag in &block_tags {
        for (i, _) in lower.match_indices(tag) {
            if i > last {
                let chunk = &html[last..i];
                if chunk.trim().len() > 20 { blocks.push(chunk.to_string()); }
            }
            last = i;
        }
    }
    if last < html.len() {
        let chunk = &html[last..];
        if chunk.trim().len() > 20 { blocks.push(chunk.to_string()); }
    }
    blocks
}

/// Extract all code blocks (<pre> and <code> elements).
fn extract_code_blocks(html: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    for tag in &["pre", "code"] {
        let open = format!("<{}", tag);
        let close = format!("</{}>", tag);
        let lower = html.to_lowercase();
        let mut pos = 0;
        while let Some(start) = lower[pos..].find(&open) {
            let abs_start = pos + start;
            // Find the end of the opening tag
            if let Some(gt) = html[abs_start..].find('>') {
                let content_start = abs_start + gt + 1;
                if let Some(end) = lower[content_start..].find(&close) {
                    let content = strip_all_tags(&html[content_start..content_start + end]);
                    if content.trim().len() > 10 && *tag == "pre" {
                        blocks.push(content.trim().to_string());
                    }
                    pos = content_start + end + close.len();
                    continue;
                }
            }
            pos = abs_start + 1;
        }
    }
    blocks
}

/// Extract tables as pipe-delimited text.
fn extract_tables(html: &str) -> Vec<String> {
    let mut tables = Vec::new();
    let lower = html.to_lowercase();
    let mut pos = 0;
    while let Some(start) = lower[pos..].find("<table") {
        let abs_start = pos + start;
        if let Some(end) = lower[abs_start..].find("</table>") {
            let table_html = &html[abs_start..abs_start + end + 8];
            let text = table_to_text(table_html);
            if text.trim().len() > 10 { tables.push(text); }
            pos = abs_start + end + 8;
        } else { break; }
    }
    tables
}

fn table_to_text(html: &str) -> String {
    let mut rows: Vec<String> = Vec::new();
    let lower = html.to_lowercase();
    for row_chunk in lower.split("<tr") {
        let mut cells: Vec<String> = Vec::new();
        for cell_chunk in row_chunk.split("<td").chain(row_chunk.split("<th")) {
            if let Some(gt) = cell_chunk.find('>') {
                let after = &cell_chunk[gt + 1..];
                if let Some(end) = after.find('<') {
                    let text = after[..end].trim().to_string();
                    if !text.is_empty() { cells.push(text); }
                }
            }
        }
        if !cells.is_empty() { rows.push(cells.join(" | ")); }
    }
    rows.join("\n")
}

/// Extract JSON-LD structured data.
fn extract_json_ld(html: &str) -> Option<serde_json::Value> {
    let marker = "application/ld+json";
    let pos = html.to_lowercase().find(marker)?;
    let after = &html[pos + marker.len()..];
    let start = after.find('>')?;
    let content = &after[start + 1..];
    let end = content.find("</script>")?;
    let json_str = content[..end].trim();
    serde_json::from_str(json_str).ok()
}

/// Truncate at sentence boundary.
pub fn smart_truncate(text: &str, max: usize) -> String {
    if text.len() <= max { return text.to_string(); }
    // Find the last sentence ending before max
    let slice = &text[..max];
    if let Some(pos) = slice.rfind(|c: char| ".!?".contains(c)) {
        text[..=pos].to_string()
    } else {
        format!("{}...", &text[..max])
    }
}

fn strip_all_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    let mut last_was_space = false;
    for c in s.chars() {
        if c == '<' { in_tag = true; if !last_was_space { out.push(' '); last_was_space = true; } }
        else if c == '>' { in_tag = false; }
        else if !in_tag {
            if c.is_whitespace() {
                if !last_was_space { out.push(' '); last_was_space = true; }
            } else { out.push(c); last_was_space = false; }
        }
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_code_blocks_from_html() {
        let html = r#"<p>Text</p><pre class="code">fn main() { println!("hello"); }</pre>"#;
        let blocks = extract_code_blocks(html);
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].contains("fn main()"));
    }

    #[test]
    fn smart_truncate_at_sentence() {
        let text = "First sentence. Second sentence. Third sentence is long.";
        let t = smart_truncate(text, 35);
        assert!(t.ends_with('.'));
        assert!(t.len() <= 36);
    }

    #[test]
    fn strip_tags_collapses_whitespace() {
        let html = "<div>  hello   <span>world</span>  </div>";
        let text = strip_all_tags(html);
        assert_eq!(text, "hello world");
    }

    #[test]
    fn remove_noise_strips_scripts() {
        let html = "<p>content</p><script>evil()</script><p>more</p>";
        let cleaned = remove_noise(html);
        assert!(!cleaned.contains("evil"));
        assert!(cleaned.contains("content"));
    }

    #[test]
    fn extract_json_ld_works() {
        let html = r#"<script type="application/ld+json">{"description":"Test page"}</script>"#;
        let ld = extract_json_ld(html);
        assert!(ld.is_some());
        assert_eq!(ld.unwrap()["description"], "Test page");
    }
}
