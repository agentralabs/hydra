//! Shared types for the web engine.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Which search engine produced a result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EngineLabel {
    DuckDuckGo,
    Wikipedia,
    GitHub,
    StackExchange,
    KnowledgeIndex,
    GenomeCache,
}

impl std::fmt::Display for EngineLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuckDuckGo => write!(f, "DDG"),
            Self::Wikipedia => write!(f, "Wikipedia"),
            Self::GitHub => write!(f, "GitHub"),
            Self::StackExchange => write!(f, "StackOverflow"),
            Self::KnowledgeIndex => write!(f, "KnowledgeIndex"),
            Self::GenomeCache => write!(f, "Cache"),
        }
    }
}

/// What type of content the user is looking for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentFocus {
    General,
    News,
    Documentation,
    Code,
    Academic,
}

impl Default for ContentFocus {
    fn default() -> Self { Self::General }
}

/// Cache behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CachePolicy {
    /// Use cache if available (default).
    Allow,
    /// Force fresh search, skip cache.
    Skip,
}

impl Default for CachePolicy {
    fn default() -> Self { Self::Allow }
}

/// A web search request with all parameters.
#[derive(Debug, Clone)]
pub struct WebSearchRequest {
    pub query: String,
    pub max_results: usize,
    pub deep_fetch: bool,
    pub synthesize: bool,
    pub cache_policy: CachePolicy,
    pub content_focus: ContentFocus,
}

impl WebSearchRequest {
    pub fn quick(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            max_results: crate::constants::DEFAULT_MAX_RESULTS,
            deep_fetch: true,
            synthesize: false,
            cache_policy: CachePolicy::Allow,
            content_focus: ContentFocus::General,
        }
    }

    pub fn deep(query: impl Into<String>) -> Self {
        Self {
            synthesize: true,
            ..Self::quick(query)
        }
    }
}

/// A raw search hit before ranking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawSearchHit {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub source_engine: EngineLabel,
    #[serde(skip)]
    pub fetched_content: Option<ExtractedContent>,
}

/// Extracted main content from a fetched page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedContent {
    pub main_text: String,
    pub code_blocks: Vec<String>,
    pub tables: Vec<String>,
    pub word_count: usize,
    pub fetched_at: DateTime<Utc>,
}

/// A ranked, scored search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub content: Option<ExtractedContent>,
    pub score: f64,
    pub source: EngineLabel,
    pub confidence: f64,
}

/// The full response from the web engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchResponse {
    pub query: String,
    pub hits: Vec<SearchHit>,
    pub synthesis: Option<String>,
    pub from_cache: bool,
    pub engines_used: Vec<EngineLabel>,
    pub duration_ms: u64,
}

impl WebSearchResponse {
    /// Format results for display (markdown).
    pub fn format_display(&self) -> String {
        let mut out = String::new();
        if let Some(synthesis) = &self.synthesis {
            out.push_str(synthesis);
            out.push_str("\n\n---\n\n");
        }
        for (i, hit) in self.hits.iter().enumerate() {
            out.push_str(&format!(
                "{}. **{}** [{}] (score: {:.2})\n   {}\n   {}\n",
                i + 1, hit.title, hit.source, hit.score, hit.snippet, hit.url,
            ));
            if let Some(content) = &hit.content {
                if !content.code_blocks.is_empty() {
                    out.push_str(&format!("   ({} code blocks)\n", content.code_blocks.len()));
                }
            }
            out.push('\n');
        }
        if self.from_cache {
            out.push_str("(from cache)\n");
        } else {
            let engines: Vec<String> = self.engines_used.iter().map(|e| e.to_string()).collect();
            out.push_str(&format!("Sources: {} | {}ms\n", engines.join(", "), self.duration_ms));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quick_request_defaults() {
        let req = WebSearchRequest::quick("rust ownership");
        assert_eq!(req.max_results, 10);
        assert!(req.deep_fetch);
        assert!(!req.synthesize);
    }

    #[test]
    fn deep_request_enables_synthesis() {
        let req = WebSearchRequest::deep("rust ownership");
        assert!(req.synthesize);
    }

    #[test]
    fn engine_labels_display() {
        assert_eq!(EngineLabel::DuckDuckGo.to_string(), "DDG");
        assert_eq!(EngineLabel::StackExchange.to_string(), "StackOverflow");
    }

    #[test]
    fn response_format_display() {
        let resp = WebSearchResponse {
            query: "test".into(),
            hits: vec![SearchHit {
                title: "Test".into(), url: "https://example.com".into(),
                snippet: "A test".into(), content: None, score: 0.85,
                source: EngineLabel::DuckDuckGo, confidence: 0.65,
            }],
            synthesis: None, from_cache: false,
            engines_used: vec![EngineLabel::DuckDuckGo], duration_ms: 200,
        };
        let text = resp.format_display();
        assert!(text.contains("Test"));
        assert!(text.contains("DDG"));
    }
}
