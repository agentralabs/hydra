//! Input decomposition — chunks large inputs at natural boundaries.
//!
//! UCU Module #6 (Wave 4). Handles massive user inputs (logs, code dumps,
//! documents) by splitting at natural boundaries rather than truncating.
//! Why not a sister? Purely in-memory text analysis — no I/O, no LLM.

/// A chunk of decomposed input with context.
#[derive(Debug, Clone)]
pub struct Chunk {
    /// The actual content of this chunk.
    pub content: String,
    /// Brief summary of what came before this chunk.
    pub context_summary: String,
    /// Position in the sequence (0-indexed).
    pub index: usize,
    /// Total number of chunks.
    pub total: usize,
    /// Type of content detected in this chunk.
    pub content_type: ContentType,
}

/// Detected content type for smarter chunking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentType {
    /// Prose text (paragraphs, docs).
    Prose,
    /// Source code.
    Code,
    /// Log output (timestamped lines).
    LogOutput,
    /// Structured data (JSON, YAML, TOML).
    StructuredData,
    /// Mixed/unknown content.
    Mixed,
}

/// Check if input needs decomposition.
pub fn needs_decomposition(input: &str, max_chars: usize) -> bool {
    input.len() > max_chars
}

/// Decompose large input into manageable chunks.
pub fn decompose_input(input: &str, max_chunk_chars: usize) -> Vec<Chunk> {
    if input.len() <= max_chunk_chars {
        return vec![Chunk {
            content: input.to_string(),
            context_summary: String::new(),
            index: 0,
            total: 1,
            content_type: detect_content_type(input),
        }];
    }

    let content_type = detect_content_type(input);
    let boundaries = find_boundaries(input, content_type);
    let mut chunks = Vec::new();
    let mut start = 0;

    while start < input.len() {
        let end = find_chunk_end(input, start, max_chunk_chars, &boundaries);
        let content = &input[start..end];

        let context_summary = if chunks.is_empty() {
            String::new()
        } else {
            build_context_summary(&chunks)
        };

        chunks.push(Chunk {
            content: content.to_string(),
            context_summary,
            index: chunks.len(),
            total: 0, // Updated after loop
            content_type,
        });

        start = end;
        // Skip whitespace between chunks
        while start < input.len() && input.as_bytes()[start] == b'\n' {
            start += 1;
        }
    }

    let total = chunks.len();
    for chunk in &mut chunks {
        chunk.total = total;
    }
    chunks
}

/// Detect the content type of input text.
pub fn detect_content_type(input: &str) -> ContentType {
    let lines: Vec<&str> = input.lines().take(20).collect();
    if lines.is_empty() { return ContentType::Mixed; }

    // Log detection: timestamps at line starts
    let log_patterns = ["[20", "2024-", "2025-", "2026-", "INFO ", "WARN ", "ERROR ", "DEBUG "];
    let log_count = lines.iter().filter(|l| log_patterns.iter().any(|p| l.starts_with(p))).count();
    if log_count > lines.len() / 3 { return ContentType::LogOutput; }

    // Code detection: indentation patterns, keywords
    let code_indicators = ["fn ", "def ", "class ", "import ", "pub ", "struct ", "const ",
        "let ", "var ", "function ", "module ", "#include", "package "];
    let code_count = lines.iter().filter(|l| {
        let trimmed = l.trim();
        code_indicators.iter().any(|p| trimmed.starts_with(p)) || trimmed.ends_with('{') || trimmed.ends_with(';')
    }).count();
    if code_count > lines.len() / 3 { return ContentType::Code; }

    // Structured data detection
    let first = lines[0].trim();
    if first.starts_with('{') || first.starts_with('[') { return ContentType::StructuredData; }
    if first.starts_with("---") || first.contains(": ") && !first.contains(". ") { return ContentType::StructuredData; }

    // Default to prose
    ContentType::Prose
}

/// Find natural boundary positions in the text.
fn find_boundaries(input: &str, content_type: ContentType) -> Vec<usize> {
    let mut boundaries = Vec::new();

    match content_type {
        ContentType::Prose => {
            // Paragraphs (double newlines) and section headers
            for (i, _) in input.match_indices("\n\n") {
                boundaries.push(i + 2);
            }
            for (i, _) in input.match_indices("\n# ") {
                boundaries.push(i + 1);
            }
            for (i, _) in input.match_indices("\n## ") {
                boundaries.push(i + 1);
            }
        }
        ContentType::Code => {
            // Function/class boundaries
            for (i, _) in input.match_indices("\nfn ") { boundaries.push(i + 1); }
            for (i, _) in input.match_indices("\npub ") { boundaries.push(i + 1); }
            for (i, _) in input.match_indices("\ndef ") { boundaries.push(i + 1); }
            for (i, _) in input.match_indices("\nclass ") { boundaries.push(i + 1); }
            for (i, _) in input.match_indices("\n\n") { boundaries.push(i + 2); }
        }
        ContentType::LogOutput => {
            // Every N lines is a boundary for logs
            let mut line_start = 0;
            let mut line_count = 0;
            for (i, c) in input.char_indices() {
                if c == '\n' {
                    line_count += 1;
                    if line_count % 50 == 0 {
                        boundaries.push(i + 1);
                    }
                    line_start = i + 1;
                }
            }
            let _ = line_start; // suppress warning
        }
        ContentType::StructuredData => {
            // Top-level object boundaries
            for (i, _) in input.match_indices("\n}\n") { boundaries.push(i + 3); }
            for (i, _) in input.match_indices("\n---\n") { boundaries.push(i + 5); }
        }
        ContentType::Mixed => {
            for (i, _) in input.match_indices("\n\n") { boundaries.push(i + 2); }
        }
    }

    boundaries.sort();
    boundaries.dedup();
    boundaries
}

/// Find the best chunk end position within max_chars.
fn find_chunk_end(input: &str, start: usize, max_chars: usize, boundaries: &[usize]) -> usize {
    let ideal_end = (start + max_chars).min(input.len());

    // If we're already at the end, return it
    if ideal_end >= input.len() { return input.len(); }

    // Find the nearest boundary before ideal_end
    let best_boundary = boundaries.iter().rev()
        .find(|&&b| b > start && b <= ideal_end)
        .copied();

    if let Some(b) = best_boundary {
        return b;
    }

    // No boundary found — fall back to nearest newline
    if let Some(nl) = input[start..ideal_end].rfind('\n') {
        return start + nl + 1;
    }

    ideal_end
}

/// Build a brief context summary from previous chunks.
fn build_context_summary(chunks: &[Chunk]) -> String {
    let last = chunks.last().unwrap();
    let first_line = last.content.lines().next().unwrap_or("").trim();
    format!("[Chunk {}/{} preceded this. Last section started with: {}]",
        last.index + 1, last.total.max(last.index + 1),
        &first_line[..first_line.len().min(60)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_input_no_decompose() {
        let chunks = decompose_input("hello world", 1000);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, "hello world");
    }

    #[test]
    fn test_large_input_splits() {
        let input = "a\n\n".repeat(100); // ~300 chars with natural boundaries
        let chunks = decompose_input(&input, 50);
        assert!(chunks.len() > 1);
        assert_eq!(chunks[0].index, 0);
        assert_eq!(chunks.last().unwrap().index, chunks.len() - 1);
    }

    #[test]
    fn test_detect_code() {
        let code = "fn main() {\n    println!(\"hello\");\n}\n\npub struct Foo {\n    x: i32,\n}\n";
        assert_eq!(detect_content_type(code), ContentType::Code);
    }

    #[test]
    fn test_detect_log() {
        let log = "[2026-03-13 10:00] INFO Starting\n[2026-03-13 10:01] ERROR Failed\n[2026-03-13 10:02] WARN Retry\n";
        assert_eq!(detect_content_type(log), ContentType::LogOutput);
    }

    #[test]
    fn test_detect_json() {
        assert_eq!(detect_content_type("{\"key\": \"value\"}"), ContentType::StructuredData);
    }

    #[test]
    fn test_needs_decomposition() {
        assert!(!needs_decomposition("short", 1000));
        assert!(needs_decomposition(&"x".repeat(2000), 1000));
    }

    #[test]
    fn test_chunk_context_summary() {
        let input = format!("{}\n\n{}", "a".repeat(100), "b".repeat(100));
        let chunks = decompose_input(&input, 120);
        if chunks.len() > 1 {
            assert!(!chunks[1].context_summary.is_empty());
        }
    }
}
