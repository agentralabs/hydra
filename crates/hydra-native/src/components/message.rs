//! Message rendering — basic markdown support.

/// Render segments for a message (for converting markdown to styled elements)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageSegment {
    Text(String),
    Code {
        language: Option<String>,
        content: String,
    },
    InlineCode(String),
    Bold(String),
}

/// Parse a message into renderable segments
pub fn parse_message(content: &str) -> Vec<MessageSegment> {
    let mut segments = Vec::new();
    let mut remaining = content;

    while !remaining.is_empty() {
        // Check for code block
        if remaining.starts_with("```") {
            let after_fence = &remaining[3..];
            if let Some(end_idx) = after_fence.find("```") {
                let block = &after_fence[..end_idx];
                let (language, code) = if let Some(newline) = block.find('\n') {
                    let lang = block[..newline].trim();
                    let lang = if lang.is_empty() {
                        None
                    } else {
                        Some(lang.to_string())
                    };
                    (lang, block[newline + 1..].to_string())
                } else {
                    (None, block.to_string())
                };
                segments.push(MessageSegment::Code {
                    language,
                    content: code,
                });
                remaining = &after_fence[end_idx + 3..];
                continue;
            }
        }

        // Check for inline code
        if remaining.starts_with('`') {
            let after_tick = &remaining[1..];
            if let Some(end_idx) = after_tick.find('`') {
                segments.push(MessageSegment::InlineCode(
                    after_tick[..end_idx].to_string(),
                ));
                remaining = &after_tick[end_idx + 1..];
                continue;
            }
        }

        // Check for bold
        if remaining.starts_with("**") {
            let after_stars = &remaining[2..];
            if let Some(end_idx) = after_stars.find("**") {
                segments.push(MessageSegment::Bold(after_stars[..end_idx].to_string()));
                remaining = &after_stars[end_idx + 2..];
                continue;
            }
        }

        // Find next special character
        let next_special = remaining
            .find(|c: char| c == '`' || c == '*')
            .unwrap_or(remaining.len());

        if next_special > 0 {
            segments.push(MessageSegment::Text(remaining[..next_special].to_string()));
            remaining = &remaining[next_special..];
        } else {
            // Single special char that didn't match a pattern
            segments.push(MessageSegment::Text(remaining[..1].to_string()));
            remaining = &remaining[1..];
        }
    }

    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_text() {
        let segments = parse_message("Hello world");
        assert_eq!(segments, vec![MessageSegment::Text("Hello world".into())]);
    }

    #[test]
    fn test_inline_code() {
        let segments = parse_message("Use `cargo test` here");
        assert_eq!(
            segments,
            vec![
                MessageSegment::Text("Use ".into()),
                MessageSegment::InlineCode("cargo test".into()),
                MessageSegment::Text(" here".into()),
            ]
        );
    }

    #[test]
    fn test_code_block() {
        let segments = parse_message("Before\n```rust\nfn main() {}\n```\nAfter");
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0], MessageSegment::Text("Before\n".into()));
        assert!(
            matches!(&segments[1], MessageSegment::Code { language: Some(lang), .. } if lang == "rust")
        );
        assert_eq!(segments[2], MessageSegment::Text("\nAfter".into()));
    }

    #[test]
    fn test_bold() {
        let segments = parse_message("This is **bold** text");
        assert_eq!(
            segments,
            vec![
                MessageSegment::Text("This is ".into()),
                MessageSegment::Bold("bold".into()),
                MessageSegment::Text(" text".into()),
            ]
        );
    }
}
