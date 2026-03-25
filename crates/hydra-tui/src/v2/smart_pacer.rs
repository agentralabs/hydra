//! Smart Pacer — content-aware streaming speed (GAP 3).
//! Slows down at sentence/paragraph/code boundaries. Errors pace slower.
//! Makes streaming feel intelligent, not mechanical.

/// Determine how many chars to advance and whether to pause.
/// Returns (chars_to_advance, pause_ms_after).
pub fn pace(text: &str, cursor: usize, base_chars: usize) -> (usize, u64) {
    if cursor >= text.len() { return (0, 0); }
    let remaining = &text[cursor..];
    let advance = base_chars.min(remaining.len());
    if advance == 0 { return (0, 0); }

    // Look at the window we just revealed
    let end = cursor + advance;
    let window = &text[cursor..end];

    // Code block boundary: pause before ``` (user needs to prepare)
    if window.contains("```") { return (advance, 200); }
    // Paragraph boundary: double newline
    if window.contains("\n\n") { return (advance, 120); }
    // Sentence boundary: period/question/exclamation followed by space or newline
    if window.ends_with(". ") || window.ends_with(".\n")
        || window.ends_with("? ") || window.ends_with("!\n")
        || window.ends_with("! ") || window.ends_with("?\n")
    { return (advance, 80); }
    // List item: newline followed by "- " or "* " or number
    if window.contains("\n- ") || window.contains("\n* ") || window.contains("\n1.") {
        return (advance, 60);
    }

    (advance, 0) // Normal speed, no pause
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_text_no_pause() {
        let (adv, pause) = pace("hello world foo bar", 0, 10);
        assert_eq!(adv, 10);
        assert_eq!(pause, 0);
    }

    #[test]
    fn sentence_boundary_pauses() {
        let text = "First sentence. Second sentence.";
        let (adv, pause) = pace(text, 0, 16); // covers "First sentence. "
        assert_eq!(pause, 80);
    }

    #[test]
    fn code_block_pauses_longer() {
        let text = "Here is code:\n```rust\nfn main() {}\n```";
        let (adv, pause) = pace(text, 10, 10); // covers "e:\n```rust"
        assert_eq!(pause, 200);
    }

    #[test]
    fn paragraph_boundary_pauses() {
        let text = "First paragraph.\n\nSecond paragraph.";
        let (adv, pause) = pace(text, 10, 10); // covers "raph.\n\nSec"
        assert_eq!(pause, 120);
    }

    #[test]
    fn past_end_returns_zero() {
        let (adv, pause) = pace("hi", 5, 10);
        assert_eq!(adv, 0);
    }
}
