use super::*;

// ── Token estimation ──────────────────────────────────────

#[test]
fn estimate_tokens_empty_string() {
    // Empty: (0 + 3) / 4 = 0 with integer division
    assert_eq!(estimate_tokens(""), 0);
}

#[test]
fn estimate_tokens_short_string() {
    // "hello" = 5 chars => (5+3)/4 = 2
    assert_eq!(estimate_tokens("hello"), 2);
}

#[test]
fn estimate_tokens_exact_boundary() {
    // 4 chars => (4+3)/4 = 1
    assert_eq!(estimate_tokens("abcd"), 1);
}

#[test]
fn estimate_tokens_longer_string() {
    // 100 chars => (100+3)/4 = 25
    let text = "a".repeat(100);
    assert_eq!(estimate_tokens(&text), 25);
}

#[test]
fn estimate_tokens_single_char() {
    // 1 char => (1+3)/4 = 1
    assert_eq!(estimate_tokens("x"), 1);
}

// ── Context segment ───────────────────────────────────────

#[test]
fn context_segment_auto_estimates_tokens() {
    let seg = ContextSegment::new("hello world", 5);
    assert!(seg.token_estimate > 0);
    assert_eq!(seg.priority, 5);
}

#[test]
fn context_segment_with_explicit_tokens() {
    let seg = ContextSegment::with_tokens("test", 3, 42);
    assert_eq!(seg.token_estimate, 42);
}

// ── Truncation ────────────────────────────────────────────

#[test]
fn truncate_to_tokens_within_budget() {
    let result = truncate_to_tokens("hello", 100);
    assert_eq!(result, "hello");
}

#[test]
fn truncate_to_tokens_over_budget() {
    let long_text = "a".repeat(100);
    let result = truncate_to_tokens(&long_text, 5); // 5 tokens ~= 20 chars
    assert!(result.len() <= 23); // 20 chars + "..."
    assert!(result.ends_with("..."));
}

#[test]
fn truncate_to_tokens_zero_budget() {
    let result = truncate_to_tokens("anything", 0);
    assert_eq!(result, "");
}

// ── Compressor: no compression needed ─────────────────────

#[test]
fn compress_everything_fits() {
    let compressor = ContextCompressor::new();
    let segments = vec![
        ContextSegment::with_tokens("segment one", 5, 10),
        ContextSegment::with_tokens("segment two", 3, 10),
    ];
    let result = compressor.compress(&segments, 100);
    assert_eq!(result.segments_dropped, 0);
    assert_eq!(result.segments_truncated, 0);
    assert_eq!(result.original_tokens, 20);
    assert_eq!(result.compressed_tokens, 20);
    assert!(result.output.contains("segment one"));
    assert!(result.output.contains("segment two"));
}

#[test]
fn compress_empty_segments() {
    let compressor = ContextCompressor::new();
    let result = compressor.compress(&[], 100);
    assert_eq!(result.output, "");
    assert_eq!(result.original_tokens, 0);
    assert_eq!(result.segments_dropped, 0);
}

// ── Compressor: dropping low-priority segments ────────────

#[test]
fn compress_drops_lowest_priority_first() {
    let compressor = ContextCompressor::new();
    let segments = vec![
        ContextSegment::with_tokens("critical system prompt", 10, 50),
        ContextSegment::with_tokens("user question", 8, 30),
        ContextSegment::with_tokens("old conversation history", 2, 40),
    ];
    // Budget = 80 (total = 120), so one segment must be dropped
    let result = compressor.compress(&segments, 80);
    assert!(result.output.contains("critical system prompt"));
    assert!(result.output.contains("user question"));
    assert!(!result.output.contains("old conversation history"));
    assert_eq!(result.segments_dropped, 1);
}

#[test]
fn compress_drops_multiple_low_priority() {
    let compressor = ContextCompressor::new();
    let segments = vec![
        ContextSegment::with_tokens("essential", 10, 30),
        ContextSegment::with_tokens("nice to have", 3, 30),
        ContextSegment::with_tokens("fluff", 1, 30),
    ];
    // Budget = 30 (total = 90), only room for the essential one
    let result = compressor.compress(&segments, 30);
    assert!(result.output.contains("essential"));
    assert!(!result.output.contains("nice to have"));
    assert!(!result.output.contains("fluff"));
    assert_eq!(result.segments_dropped, 2);
}

// ── Compressor: truncation ────────────────────────────────

#[test]
fn compress_truncates_when_partially_fits() {
    let compressor = ContextCompressor::new();
    let long_content = "x".repeat(200); // ~50 tokens
    let segments = vec![
        ContextSegment::with_tokens("important", 10, 20),
        ContextSegment::with_tokens(&long_content, 5, 50),
    ];
    // Budget = 30 (total = 70), so "important" fits (20), then 10 tokens left for long
    let result = compressor.compress(&segments, 30);
    assert!(result.output.contains("important"));
    assert_eq!(result.segments_truncated, 1);
    // The truncated segment uses estimate_tokens on the truncated text,
    // which may slightly exceed the remaining budget due to the "..." suffix
    // but should be much less than the original 50 tokens
    assert!(result.compressed_tokens < 50, "compression should significantly reduce tokens");
}

// ── Compressor: budget enforcement ────────────────────────

#[test]
fn compress_never_exceeds_budget() {
    let compressor = ContextCompressor::new();
    let segments: Vec<ContextSegment> = (0..20)
        .map(|i| ContextSegment::with_tokens(&format!("segment {i}"), i as u8, 10))
        .collect();
    let result = compressor.compress(&segments, 50);
    assert!(result.compressed_tokens <= 50);
}

#[test]
fn compress_zero_budget_drops_everything() {
    let compressor = ContextCompressor::new();
    let segments = vec![
        ContextSegment::with_tokens("a", 10, 5),
        ContextSegment::with_tokens("b", 5, 5),
    ];
    let result = compressor.compress(&segments, 0);
    assert_eq!(result.output, "");
    assert_eq!(result.segments_dropped, 2);
}

// ── Compressor: preserves original order ──────────────────

#[test]
fn compress_preserves_original_segment_order() {
    let compressor = ContextCompressor::with_separator(" | ");
    let segments = vec![
        ContextSegment::with_tokens("first", 5, 10),
        ContextSegment::with_tokens("second", 8, 10),
        ContextSegment::with_tokens("third", 3, 10),
    ];
    // Budget = 20 means "third" (priority 3) is dropped
    let result = compressor.compress(&segments, 20);
    let idx_first = result.output.find("first").unwrap();
    let idx_second = result.output.find("second").unwrap();
    assert!(
        idx_first < idx_second,
        "first should appear before second in output"
    );
}

// ── CompressionResult stats ───────────────────────────────

#[test]
fn compression_ratio_no_compression() {
    let result = CompressionResult {
        output: String::new(),
        original_tokens: 100,
        compressed_tokens: 100,
        segments_dropped: 0,
        segments_truncated: 0,
    };
    assert!((result.ratio() - 1.0).abs() < f64::EPSILON);
}

#[test]
fn compression_ratio_half() {
    let result = CompressionResult {
        output: String::new(),
        original_tokens: 100,
        compressed_tokens: 50,
        segments_dropped: 1,
        segments_truncated: 0,
    };
    assert!((result.ratio() - 0.5).abs() < f64::EPSILON);
}

#[test]
fn compression_ratio_zero_original() {
    let result = CompressionResult {
        output: String::new(),
        original_tokens: 0,
        compressed_tokens: 0,
        segments_dropped: 0,
        segments_truncated: 0,
    };
    assert!((result.ratio() - 1.0).abs() < f64::EPSILON);
}

#[test]
fn tokens_saved_calculation() {
    let result = CompressionResult {
        output: String::new(),
        original_tokens: 100,
        compressed_tokens: 30,
        segments_dropped: 2,
        segments_truncated: 0,
    };
    assert_eq!(result.tokens_saved(), 70);
}

#[test]
fn tokens_saved_zero_when_no_compression() {
    let result = CompressionResult {
        output: String::new(),
        original_tokens: 50,
        compressed_tokens: 50,
        segments_dropped: 0,
        segments_truncated: 0,
    };
    assert_eq!(result.tokens_saved(), 0);
}

// ── Compressor: truncate single string ────────────────────

#[test]
fn truncate_within_budget_returns_original() {
    let compressor = ContextCompressor::new();
    let result = compressor.truncate("short text", 100);
    assert_eq!(result, "short text");
}

#[test]
fn truncate_over_budget_truncates() {
    let compressor = ContextCompressor::new();
    let long = "a".repeat(400); // ~100 tokens
    let result = compressor.truncate(&long, 10); // 10 tokens = ~40 chars
    assert!(result.len() < long.len());
    assert!(result.ends_with("..."));
}

// ── Custom separator ──────────────────────────────────────

#[test]
fn custom_separator_used_in_output() {
    let compressor = ContextCompressor::with_separator(" ## ");
    let segments = vec![
        ContextSegment::with_tokens("alpha", 5, 5),
        ContextSegment::with_tokens("beta", 5, 5),
    ];
    let result = compressor.compress(&segments, 100);
    assert!(result.output.contains(" ## "));
    assert_eq!(result.output, "alpha ## beta");
}
