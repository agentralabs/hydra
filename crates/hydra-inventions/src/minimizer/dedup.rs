//! SemanticDedup — detect and remove semantically duplicate content.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Result of deduplication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DedupResult {
    pub original_tokens: usize,
    pub deduped_tokens: usize,
    pub duplicates_found: usize,
    pub content: String,
    pub removed: Vec<DuplicateEntry>,
}

impl DedupResult {
    pub fn compression_ratio(&self) -> f64 {
        if self.original_tokens == 0 {
            return 1.0;
        }
        1.0 - (self.deduped_tokens as f64 / self.original_tokens as f64)
    }
}

/// A detected duplicate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateEntry {
    pub original_position: usize,
    pub duplicate_position: usize,
    pub content: String,
    pub similarity: f64,
}

/// Semantic deduplication engine
pub struct SemanticDedup {
    /// Minimum similarity threshold for dedup (0.0 - 1.0)
    similarity_threshold: f64,
    /// Minimum chunk size in tokens to consider
    min_chunk_size: usize,
}

impl SemanticDedup {
    pub fn new(similarity_threshold: f64, min_chunk_size: usize) -> Self {
        Self {
            similarity_threshold: similarity_threshold.clamp(0.0, 1.0),
            min_chunk_size,
        }
    }

    /// Deduplicate content by finding and removing repeated segments
    pub fn deduplicate(&self, content: &str) -> DedupResult {
        let chunks: Vec<&str> = content.split('\n').collect();
        let original_tokens = estimate_tokens(content);

        // Find duplicate chunks using n-gram fingerprinting
        let mut seen: HashMap<String, usize> = HashMap::new();
        let mut kept_chunks: Vec<&str> = Vec::new();
        let mut removed: Vec<DuplicateEntry> = Vec::new();

        for (i, chunk) in chunks.iter().enumerate() {
            let normalized = normalize(chunk);
            if normalized.len() < self.min_chunk_size {
                kept_chunks.push(chunk);
                continue;
            }

            if let Some(&original_pos) = seen.get(&normalized) {
                let similarity = compute_similarity(chunk, chunks[original_pos]);
                if similarity >= self.similarity_threshold {
                    removed.push(DuplicateEntry {
                        original_position: original_pos,
                        duplicate_position: i,
                        content: chunk.to_string(),
                        similarity,
                    });
                    // Replace with reference marker
                    kept_chunks.push("[see above]");
                    continue;
                }
            }

            seen.insert(normalized, i);
            kept_chunks.push(chunk);
        }

        let deduped_content = kept_chunks.join("\n");
        let deduped_tokens = estimate_tokens(&deduped_content);

        DedupResult {
            original_tokens,
            deduped_tokens,
            duplicates_found: removed.len(),
            content: deduped_content,
            removed,
        }
    }

    /// Deduplicate across multiple documents
    pub fn deduplicate_multi(&self, documents: &[&str]) -> Vec<DedupResult> {
        documents.iter().map(|doc| self.deduplicate(doc)).collect()
    }
}

impl Default for SemanticDedup {
    fn default() -> Self {
        Self::new(0.9, 10)
    }
}

/// Normalize text for comparison (lowercase, trim, collapse whitespace)
fn normalize(text: &str) -> String {
    text.trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Estimate token count (~4 chars per token)
fn estimate_tokens(text: &str) -> usize {
    (text.len() + 3) / 4
}

/// Compute similarity between two strings using character overlap
fn compute_similarity(a: &str, b: &str) -> f64 {
    let a_norm = normalize(a);
    let b_norm = normalize(b);

    if a_norm == b_norm {
        return 1.0;
    }

    let a_chars: std::collections::HashSet<char> = a_norm.chars().collect();
    let b_chars: std::collections::HashSet<char> = b_norm.chars().collect();

    let intersection = a_chars.intersection(&b_chars).count();
    let union = a_chars.union(&b_chars).count();

    if union == 0 {
        return 0.0;
    }

    intersection as f64 / union as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_dedup() {
        let dedup = SemanticDedup::new(0.9, 5);
        let content = "This is a test line with enough content to match\nSome other content here\nThis is a test line with enough content to match";
        let result = dedup.deduplicate(content);
        assert_eq!(result.duplicates_found, 1);
        assert!(result.compression_ratio() > 0.0);
    }

    #[test]
    fn test_no_duplicates() {
        let dedup = SemanticDedup::new(0.9, 5);
        let content = "first unique line here\nsecond unique line here\nthird unique line here";
        let result = dedup.deduplicate(content);
        assert_eq!(result.duplicates_found, 0);
    }

    #[test]
    fn test_short_chunks_skipped() {
        let dedup = SemanticDedup::new(0.9, 100);
        let content = "short\nshort";
        let result = dedup.deduplicate(content);
        assert_eq!(result.duplicates_found, 0);
    }

    #[test]
    fn test_compression_ratio() {
        let result = DedupResult {
            original_tokens: 100,
            deduped_tokens: 50,
            duplicates_found: 2,
            content: String::new(),
            removed: vec![],
        };
        assert!((result.compression_ratio() - 0.5).abs() < f64::EPSILON);
    }
}
