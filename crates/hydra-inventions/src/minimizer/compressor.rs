//! ContextCompressor — compress context to minimize token usage.

use serde::{Deserialize, Serialize};

/// Compression level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionLevel {
    /// Light: remove whitespace, comments
    Light,
    /// Medium: also summarize verbose sections
    Medium,
    /// Aggressive: maximum compression, may lose nuance
    Aggressive,
}

/// Result of compression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionResult {
    pub original_tokens: usize,
    pub compressed_tokens: usize,
    pub level: CompressionLevel,
    pub content: String,
    pub techniques_applied: Vec<String>,
}

impl CompressionResult {
    pub fn compression_ratio(&self) -> f64 {
        if self.original_tokens == 0 {
            return 1.0;
        }
        1.0 - (self.compressed_tokens as f64 / self.original_tokens as f64)
    }
}

/// Context compressor that reduces token usage
pub struct ContextCompressor {
    level: CompressionLevel,
    preserve_code: bool,
}

impl ContextCompressor {
    pub fn new(level: CompressionLevel) -> Self {
        Self {
            level,
            preserve_code: true,
        }
    }

    pub fn with_code_preservation(mut self, preserve: bool) -> Self {
        self.preserve_code = preserve;
        self
    }

    /// Compress content according to the configured level
    pub fn compress(&self, content: &str) -> CompressionResult {
        let original_tokens = estimate_tokens(content);
        let mut compressed = content.to_string();
        let mut techniques = Vec::new();

        // Light: whitespace normalization
        compressed = normalize_whitespace(&compressed);
        techniques.push("whitespace_normalization".into());

        // Light: remove empty lines
        compressed = remove_excess_blank_lines(&compressed);
        techniques.push("blank_line_removal".into());

        if matches!(
            self.level,
            CompressionLevel::Medium | CompressionLevel::Aggressive
        ) {
            // Medium: remove comments (unless preserving code)
            if !self.preserve_code {
                compressed = remove_comments(&compressed);
                techniques.push("comment_removal".into());
            }

            // Medium: abbreviate common patterns
            compressed = abbreviate_patterns(&compressed);
            techniques.push("pattern_abbreviation".into());
        }

        if self.level == CompressionLevel::Aggressive {
            // Aggressive: truncate long lines
            compressed = truncate_long_lines(&compressed, 200);
            techniques.push("line_truncation".into());

            // Aggressive: remove redundant prefixes
            compressed = remove_redundant_prefixes(&compressed);
            techniques.push("prefix_removal".into());
        }

        let compressed_tokens = estimate_tokens(&compressed);

        CompressionResult {
            original_tokens,
            compressed_tokens,
            level: self.level,
            content: compressed,
            techniques_applied: techniques,
        }
    }
}

impl Default for ContextCompressor {
    fn default() -> Self {
        Self::new(CompressionLevel::Medium)
    }
}

fn estimate_tokens(text: &str) -> usize {
    (text.len() + 3) / 4
}

fn normalize_whitespace(text: &str) -> String {
    text.lines()
        .map(|line| {
            // Preserve leading indent, collapse internal whitespace
            let trimmed = line.trim_end();
            let leading = line.len() - line.trim_start().len();
            let indent: String = line.chars().take(leading).collect();
            let rest = trimmed.trim_start();
            if rest.is_empty() {
                String::new()
            } else {
                format!(
                    "{}{}",
                    indent,
                    rest.split_whitespace().collect::<Vec<_>>().join(" ")
                )
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn remove_excess_blank_lines(text: &str) -> String {
    let mut result = Vec::new();
    let mut prev_blank = false;

    for line in text.lines() {
        let is_blank = line.trim().is_empty();
        if is_blank && prev_blank {
            continue;
        }
        result.push(line);
        prev_blank = is_blank;
    }

    result.join("\n")
}

fn remove_comments(text: &str) -> String {
    text.lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.starts_with("//") && !trimmed.starts_with('#')
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn abbreviate_patterns(text: &str) -> String {
    text.replace("function ", "fn ")
        .replace("return ", "ret ")
        .replace("const ", "c ")
}

fn truncate_long_lines(text: &str, max_len: usize) -> String {
    text.lines()
        .map(|line| {
            if line.len() > max_len {
                format!("{}...", &line[..max_len])
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn remove_redundant_prefixes(text: &str) -> String {
    text.replace("Error: error:", "Error:")
        .replace("Warning: warning:", "Warning:")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_light_compression() {
        let compressor = ContextCompressor::new(CompressionLevel::Light);
        let content = "hello   world\n\n\n\nfoo   bar\n\n\nbaz";
        let result = compressor.compress(content);
        assert!(result.compressed_tokens <= result.original_tokens);
        assert!(result
            .techniques_applied
            .contains(&"whitespace_normalization".to_string()));
    }

    #[test]
    fn test_medium_compression() {
        let compressor = ContextCompressor::new(CompressionLevel::Medium);
        let content = "function hello() {\n  return true;\n}\nconst x = 1;";
        let result = compressor.compress(content);
        assert!(result.content.contains("fn hello()"));
        assert!(result
            .techniques_applied
            .contains(&"pattern_abbreviation".to_string()));
    }

    #[test]
    fn test_aggressive_compression() {
        let compressor = ContextCompressor::new(CompressionLevel::Aggressive);
        let long_line = "x".repeat(300);
        let content = format!("short\n{}\nshort", long_line);
        let result = compressor.compress(&content);
        assert!(result.content.lines().all(|l| l.len() <= 203)); // 200 + "..."
    }

    #[test]
    fn test_compression_ratio() {
        let compressor = ContextCompressor::new(CompressionLevel::Aggressive);
        let content = "hello   world   foo   bar\n\n\n\nbaz   qux\n\n\n\n";
        let result = compressor.compress(content);
        assert!(result.compression_ratio() > 0.0);
    }
}
