//! Query interface for the code semantic index.
//!
//! Provides keyword-based symbol lookup for the cognitive loop's PERCEIVE
//! phase, formatting matched symbols as LLM-readable context strings.

use std::sync::Arc;

use hydra_db::HydraDb;

/// Minimum word length to use as a search keyword.
const MIN_KEYWORD_LEN: usize = 3;

/// Maximum symbols to return per keyword query.
const MAX_RESULTS_PER_KEYWORD: usize = 8;

/// Maximum total symbols to include in context.
const MAX_TOTAL_SYMBOLS: usize = 20;

/// Query the code index for symbols relevant to user's question.
pub(crate) fn query_relevant_symbols(
    text: &str,
    db: &Arc<HydraDb>,
) -> Option<String> {
    let keywords = extract_keywords(text);
    if keywords.is_empty() {
        return None;
    }

    let mut seen = std::collections::HashSet::new();
    let mut entries = Vec::new();

    for keyword in &keywords {
        let pattern = format!("%{}%", keyword);
        if let Ok(rows) = db.query_symbols(&pattern, MAX_RESULTS_PER_KEYWORD) {
            for row in rows {
                let key = format!("{}:{}", row.file_path, row.symbol_name);
                if seen.insert(key) {
                    entries.push(format_symbol_entry(&row));
                }
                if entries.len() >= MAX_TOTAL_SYMBOLS {
                    break;
                }
            }
        }
        if entries.len() >= MAX_TOTAL_SYMBOLS {
            break;
        }
    }

    if entries.is_empty() {
        return None;
    }

    let header = format!(
        "### Codebase Symbols ({} matches)\n",
        entries.len()
    );
    Some(format!("{}{}", header, entries.join("\n")))
}

/// Get a summary of the indexed codebase.
pub(crate) fn index_summary(db: &Arc<HydraDb>) -> Option<String> {
    let count = db.symbol_count().unwrap_or(0);
    if count == 0 {
        return None;
    }
    Some(format!("Code index: {} symbols indexed", count))
}

// ---------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------

/// Extract keywords from user text for symbol search.
fn extract_keywords(text: &str) -> Vec<String> {
    // Common stop words to filter out
    let stop_words: &[&str] = &[
        "the", "and", "for", "are", "but", "not", "you", "all",
        "can", "had", "her", "was", "one", "our", "out", "has",
        "what", "how", "why", "when", "where", "which", "this",
        "that", "with", "from", "have", "will", "does", "into",
        "about", "could", "would", "should", "there", "their",
        "been", "make", "like", "just", "also", "than",
    ];

    text.split(|c: char| !c.is_alphanumeric() && c != '_')
        .map(|w| w.to_lowercase())
        .filter(|w| w.len() >= MIN_KEYWORD_LEN)
        .filter(|w| !stop_words.contains(&w.as_str()))
        .collect::<Vec<_>>()
}

/// Format a single symbol row as a concise context entry.
fn format_symbol_entry(row: &hydra_db::CodeSymbolRow) -> String {
    let sig = row
        .signature
        .as_deref()
        .unwrap_or(&row.symbol_name);

    // Shorten file path for readability
    let short_path = shorten_path(&row.file_path);

    format!(
        "- [{}] `{}` ({}:{})",
        row.symbol_type, sig, short_path, row.line_number
    )
}

/// Shorten a file path by keeping only the last 3 components.
fn shorten_path(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() <= 3 {
        return path.to_string();
    }
    format!(".../{}", parts[parts.len() - 3..].join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_keywords() {
        let kw = extract_keywords("how does the CodeIndexer work?");
        assert!(kw.contains(&"codeindexer".to_string()));
        assert!(kw.contains(&"work".to_string()));
        assert!(!kw.contains(&"how".to_string()));
        assert!(!kw.contains(&"the".to_string()));
    }

    #[test]
    fn test_extract_keywords_filters_short() {
        let kw = extract_keywords("I am ok");
        assert!(kw.is_empty());
    }

    #[test]
    fn test_shorten_path_short() {
        assert_eq!(shorten_path("src/lib.rs"), "src/lib.rs");
    }

    #[test]
    fn test_shorten_path_long() {
        let short = shorten_path("/home/user/project/crates/hydra-kernel/src/lib.rs");
        assert_eq!(short, ".../hydra-kernel/src/lib.rs");
    }
}
