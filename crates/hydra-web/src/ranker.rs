//! Result ranking — multi-signal scoring that exploits Hydra's intelligence stack.
//! Signals: source reliability, BM25 relevance, content depth, recency, domain authority, user history.

use crate::constants::*;
use crate::types::{EngineLabel, RawSearchHit, SearchHit};

/// Rank raw hits into scored SearchHits.
pub fn rank(hits: Vec<RawSearchHit>, query: &str) -> Vec<SearchHit> {
    let query_terms = tokenize(query);
    let mut scored: Vec<SearchHit> = hits
        .into_iter()
        .map(|hit| {
            let reliability = source_reliability(hit.source_engine);
            let relevance = bm25_score(&query_terms, &hit.title, &hit.snippet);
            let depth = content_depth_score(&hit);
            let recency = 0.5; // neutral — no date extraction yet
            let authority = domain_authority(&hit.url);
            let history = 0.5; // neutral — no WebsiteMemory integration yet

            let score = 0.30 * relevance
                + 0.20 * reliability
                + 0.15 * depth
                + 0.15 * authority
                + 0.10 * recency
                + 0.10 * history;

            SearchHit {
                title: hit.title,
                url: hit.url,
                snippet: hit.snippet,
                content: hit.fetched_content,
                score,
                source: hit.source_engine,
                confidence: reliability,
            }
        })
        .collect();

    scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    scored
}

/// Base source reliability from constants.
fn source_reliability(engine: EngineLabel) -> f64 {
    match engine {
        EngineLabel::GenomeCache => RELIABILITY_GENOME_CACHE,
        EngineLabel::KnowledgeIndex => RELIABILITY_KNOWLEDGE_INDEX,
        EngineLabel::Wikipedia => RELIABILITY_WIKIPEDIA,
        EngineLabel::StackExchange => RELIABILITY_STACKEXCHANGE,
        EngineLabel::GitHub => RELIABILITY_GITHUB,
        EngineLabel::DuckDuckGo => RELIABILITY_DDG,
    }
}

/// BM25-lite: IDF-weighted term overlap between query and document.
fn bm25_score(query_terms: &[String], title: &str, snippet: &str) -> f64 {
    if query_terms.is_empty() { return 0.0; }
    let doc_terms = tokenize(&format!("{title} {snippet}"));
    if doc_terms.is_empty() { return 0.0; }

    let mut matches = 0.0;
    let mut total_idf = 0.0;
    for qt in query_terms {
        // Simple IDF approximation: rarer terms in the query get more weight
        let idf = 1.0 / (query_terms.iter().filter(|t| *t == qt).count() as f64 + 0.5);
        total_idf += idf;
        if doc_terms.iter().any(|dt| dt == qt || dt.contains(qt.as_str())) {
            matches += idf;
        }
    }
    if total_idf > 0.0 { (matches / total_idf).min(1.0) } else { 0.0 }
}

/// Score content depth — richer pages rank higher.
fn content_depth_score(hit: &RawSearchHit) -> f64 {
    match &hit.fetched_content {
        Some(content) => {
            let word_score = (content.word_count as f64 / 500.0).min(1.0);
            let code_bonus = if content.code_blocks.is_empty() { 0.0 } else { 0.2 };
            let table_bonus = if content.tables.is_empty() { 0.0 } else { 0.1 };
            (word_score + code_bonus + table_bonus).min(1.0)
        }
        None => {
            // Score based on snippet length
            let len = hit.snippet.len() as f64;
            (len / 200.0).min(0.5) // max 0.5 without deep fetch
        }
    }
}

/// Domain authority heuristic — .edu, .gov, docs, wiki get boosts.
fn domain_authority(url: &str) -> f64 {
    let lower = url.to_lowercase();
    // High authority
    if lower.contains(".edu") || lower.contains(".gov") { return 0.90; }
    if lower.contains("wikipedia.org") { return 0.85; }
    if lower.contains("docs.") || lower.contains("/docs/") || lower.contains("/documentation/") { return 0.85; }
    // Medium authority
    if lower.contains("stackoverflow.com") || lower.contains("stackexchange.com") { return 0.78; }
    if lower.contains("github.com") { return 0.75; }
    if lower.contains("developer.") || lower.contains("devdocs.") { return 0.80; }
    if lower.contains("mozilla.org") || lower.contains("w3.org") { return 0.82; }
    // Standard
    if lower.contains(".org") { return 0.65; }
    0.50 // default
}

/// Tokenize text into lowercase terms.
fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
        .filter(|w| w.len() >= 2)
        .map(|w| stem(w))
        .collect()
}

/// Minimal stemmer — strip common suffixes for better matching.
fn stem(word: &str) -> String {
    let w = word.to_lowercase();
    for suffix in &["ship", "ing", "tion", "sion", "ment", "ness", "able", "ible", "ful", "less", "ous", "ive", "ly", "er", "ed", "es", "s"] {
        if w.len() > suffix.len() + 3 {
            if let Some(stripped) = w.strip_suffix(suffix) {
                return stripped.to_string();
            }
        }
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bm25_exact_match_scores_high() {
        let terms = tokenize("rust ownership");
        let score = bm25_score(&terms, "Rust Ownership Model", "Understanding ownership in Rust");
        assert!(score > 0.5, "Expected high score, got {score}");
    }

    #[test]
    fn bm25_no_match_scores_low() {
        let terms = tokenize("rust ownership");
        let score = bm25_score(&terms, "Python Tutorial", "Learn Python programming");
        assert!(score < 0.1, "Expected low score, got {score}");
    }

    #[test]
    fn domain_authority_edu_high() {
        assert!(domain_authority("https://cs.stanford.edu/paper") > 0.8);
    }

    #[test]
    fn domain_authority_random_low() {
        assert!(domain_authority("https://random-blog.com/post") < 0.6);
    }

    #[test]
    fn stem_removes_suffixes() {
        assert_eq!(stem("ownership"), "owner");
        assert_eq!(stem("programming"), "programm");
        assert_eq!(stem("rust"), "rust"); // too short to stem
    }

    #[test]
    fn rank_sorts_by_score() {
        let hits = vec![
            RawSearchHit { title: "Low".into(), url: "https://x.com".into(), snippet: "unrelated".into(), source_engine: EngineLabel::DuckDuckGo, fetched_content: None },
            RawSearchHit { title: "Rust Ownership".into(), url: "https://docs.rust-lang.org".into(), snippet: "ownership model".into(), source_engine: EngineLabel::Wikipedia, fetched_content: None },
        ];
        let ranked = rank(hits, "rust ownership");
        assert_eq!(ranked[0].title, "Rust Ownership");
        assert!(ranked[0].score > ranked[1].score);
    }
}
