use serde::{Deserialize, Serialize};

/// A segment of context with a priority for compression decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSegment {
    pub content: String,
    /// Higher priority segments are kept when budget is tight (0 = lowest)
    pub priority: u8,
    /// Estimated token count for this segment
    pub token_estimate: u64,
}

impl ContextSegment {
    pub fn new(content: impl Into<String>, priority: u8) -> Self {
        let content = content.into();
        let token_estimate = estimate_tokens(&content);
        Self {
            content,
            priority,
            token_estimate,
        }
    }

    /// Create a segment with an explicit token estimate
    pub fn with_tokens(content: impl Into<String>, priority: u8, tokens: u64) -> Self {
        Self {
            content: content.into(),
            priority,
            token_estimate: tokens,
        }
    }
}

/// Result of a compression operation
#[derive(Debug, Clone)]
pub struct CompressionResult {
    /// The compressed output text
    pub output: String,
    /// Total tokens in the original input
    pub original_tokens: u64,
    /// Total tokens after compression
    pub compressed_tokens: u64,
    /// Number of segments that were dropped
    pub segments_dropped: usize,
    /// Number of segments that were truncated
    pub segments_truncated: usize,
}

impl CompressionResult {
    /// Compression ratio (0.0 = nothing kept, 1.0 = no compression)
    pub fn ratio(&self) -> f64 {
        if self.original_tokens == 0 {
            1.0
        } else {
            self.compressed_tokens as f64 / self.original_tokens as f64
        }
    }

    /// Tokens saved by compression
    pub fn tokens_saved(&self) -> u64 {
        self.original_tokens.saturating_sub(self.compressed_tokens)
    }
}

/// Context compressor — reduces context to fit within a token budget
///
/// Strategies applied in order:
/// 1. Drop lowest-priority segments first
/// 2. Truncate remaining segments if still over budget
pub struct ContextCompressor {
    /// Separator used between segments in the output
    separator: String,
}

impl ContextCompressor {
    pub fn new() -> Self {
        Self {
            separator: "\n---\n".to_string(),
        }
    }

    pub fn with_separator(separator: impl Into<String>) -> Self {
        Self {
            separator: separator.into(),
        }
    }

    /// Compress context segments to fit within a token budget
    pub fn compress(&self, segments: &[ContextSegment], budget: u64) -> CompressionResult {
        let original_tokens: u64 = segments.iter().map(|s| s.token_estimate).sum();

        if original_tokens <= budget {
            // Everything fits, no compression needed
            let output = segments
                .iter()
                .map(|s| s.content.as_str())
                .collect::<Vec<_>>()
                .join(&self.separator);
            return CompressionResult {
                output,
                original_tokens,
                compressed_tokens: original_tokens,
                segments_dropped: 0,
                segments_truncated: 0,
            };
        }

        // Sort by priority descending (highest priority first)
        let mut sorted: Vec<(usize, &ContextSegment)> =
            segments.iter().enumerate().collect();
        sorted.sort_by(|a, b| b.1.priority.cmp(&a.1.priority));

        let mut kept: Vec<(usize, String, u64)> = Vec::new();
        let mut used_tokens: u64 = 0;
        let mut segments_dropped = 0;
        let mut segments_truncated = 0;

        for (idx, segment) in &sorted {
            if used_tokens >= budget {
                segments_dropped += 1;
                continue;
            }

            let remaining = budget.saturating_sub(used_tokens);
            if segment.token_estimate <= remaining {
                // Entire segment fits
                kept.push((*idx, segment.content.clone(), segment.token_estimate));
                used_tokens += segment.token_estimate;
            } else if remaining > 0 {
                // Truncate segment to fit remaining budget
                let truncated = truncate_to_tokens(&segment.content, remaining);
                let truncated_tokens = estimate_tokens(&truncated);
                kept.push((*idx, truncated, truncated_tokens));
                used_tokens += truncated_tokens;
                segments_truncated += 1;
            } else {
                segments_dropped += 1;
            }
        }

        // Restore original order for output
        kept.sort_by_key(|(idx, _, _)| *idx);

        let output = kept
            .iter()
            .map(|(_, content, _)| content.as_str())
            .collect::<Vec<_>>()
            .join(&self.separator);
        let compressed_tokens: u64 = kept.iter().map(|(_, _, t)| t).sum();

        CompressionResult {
            output,
            original_tokens,
            compressed_tokens,
            segments_dropped,
            segments_truncated,
        }
    }

    /// Compress a single string to fit within a token budget
    pub fn truncate(&self, text: &str, budget: u64) -> String {
        let current = estimate_tokens(text);
        if current <= budget {
            text.to_string()
        } else {
            truncate_to_tokens(text, budget)
        }
    }
}

impl Default for ContextCompressor {
    fn default() -> Self {
        Self::new()
    }
}

/// Estimate token count from text (approximation: ~4 chars per token for English)
pub fn estimate_tokens(text: &str) -> u64 {
    let chars = text.len() as u64;
    // Ceiling division to avoid underestimating
    (chars + 3) / 4
}

/// Truncate text to approximately fit within a token budget
fn truncate_to_tokens(text: &str, budget: u64) -> String {
    if budget == 0 {
        return String::new();
    }
    let max_chars = (budget * 4) as usize;
    if text.len() <= max_chars {
        return text.to_string();
    }
    // Find a char boundary near max_chars
    let mut end = max_chars;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    let mut result = text[..end].to_string();
    result.push_str("...");
    result
}

#[cfg(test)]
#[path = "compression_tests.rs"]
mod tests;
