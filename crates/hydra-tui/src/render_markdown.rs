//! Markdown rendering for assistant text.
//!
//! Handles: code blocks (``` with language label), **bold**, `inline code`.
//! Code blocks render with a bordered box and different background.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::theme::Theme;

/// Render assistant text with basic markdown support.
pub fn render_assistant_text(text: &str, t: &Theme) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut in_code_block = false;
    let mut code_lang = String::new();

    for raw_line in text.lines() {
        if raw_line.trim_start().starts_with("```") {
            if in_code_block {
                lines.push(Line::from(Span::styled(
                    "  └───",
                    Style::default().fg(t.fg_muted),
                )));
                in_code_block = false;
                code_lang.clear();
            } else {
                code_lang = raw_line.trim_start().trim_start_matches('`').to_string();
                let label = if code_lang.is_empty() {
                    "code".to_string()
                } else {
                    code_lang.clone()
                };
                lines.push(Line::from(Span::styled(
                    format!("  ┌─ {label} ─"),
                    Style::default().fg(t.fg_muted),
                )));
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            lines.push(Line::from(Span::styled(
                format!("  │ {raw_line}"),
                Style::default().fg(Color::Rgb(180, 190, 210)).bg(t.bg_secondary),
            )));
        } else {
            let line = render_inline_markdown(raw_line, t);
            lines.push(line);
        }
    }

    if in_code_block {
        lines.push(Line::from(Span::styled(
            "  └───",
            Style::default().fg(t.fg_muted),
        )));
    }

    if lines.is_empty() {
        lines.push(Line::from(""));
    }
    lines
}

/// Render inline markdown: **bold**, `code`.
fn render_inline_markdown(line: &str, t: &Theme) -> Line<'static> {
    let mut spans = Vec::new();
    let mut chars = line.chars().peekable();
    let mut current = String::new();
    let text_style = Style::default().fg(t.assistant_text);

    while let Some(ch) = chars.next() {
        match ch {
            '`' => {
                if !current.is_empty() {
                    spans.push(Span::styled(current.clone(), text_style));
                    current.clear();
                }
                let mut code = String::new();
                for c in chars.by_ref() {
                    if c == '`' {
                        break;
                    }
                    code.push(c);
                }
                spans.push(Span::styled(
                    format!("`{code}`"),
                    Style::default().fg(Color::Rgb(180, 190, 210)).bg(t.bg_secondary),
                ));
            }
            '*' if chars.peek() == Some(&'*') => {
                chars.next();
                if !current.is_empty() {
                    spans.push(Span::styled(current.clone(), text_style));
                    current.clear();
                }
                let mut bold = String::new();
                while let Some(c) = chars.next() {
                    if c == '*' && chars.peek() == Some(&'*') {
                        chars.next();
                        break;
                    }
                    bold.push(c);
                }
                spans.push(Span::styled(
                    bold,
                    Style::default().fg(t.assistant_text).add_modifier(Modifier::BOLD),
                ));
            }
            _ => {
                current.push(ch);
            }
        }
    }

    if !current.is_empty() {
        // Apply OSC 8 hyperlinks to plain text segments
        let linked = linkify(&current);
        spans.push(Span::styled(linked, text_style));
    }

    if spans.is_empty() {
        Line::from("")
    } else {
        Line::from(spans)
    }
}

/// Detect URLs in text and wrap with OSC 8 hyperlink sequences.
/// Makes URLs Cmd+clickable in supporting terminals (iTerm2, WezTerm, Ghostty, Kitty).
pub fn linkify(text: &str) -> String {
    let mut result = String::new();
    let mut remaining = text;

    while let Some(start) = remaining.find("https://").or_else(|| remaining.find("http://")) {
        // Add text before the URL
        result.push_str(&remaining[..start]);

        let url_text = &remaining[start..];
        // Find end of URL (space, newline, or end of string)
        let end = url_text
            .find(|c: char| c.is_whitespace() || c == ')' || c == ']' || c == '>' || c == '"')
            .unwrap_or(url_text.len());
        let url = &url_text[..end];

        // OSC 8 hyperlink: \x1b]8;;URL\x07text\x1b]8;;\x07
        result.push_str(&format!("\x1b]8;;{url}\x07{url}\x1b]8;;\x07"));

        remaining = &url_text[end..];
    }
    result.push_str(remaining);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_block_renders_with_border() {
        let t = crate::theme::Theme::dark();
        let text = "Hello\n```rust\nfn main() {}\n```\nGoodbye";
        let lines = render_assistant_text(text, &t);
        // Should have: Hello, ┌─ rust ─, │ fn main..., └───, Goodbye
        assert!(lines.len() >= 5);
    }

    #[test]
    fn bold_renders() {
        let t = crate::theme::Theme::dark();
        let lines = render_assistant_text("This is **bold** text", &t);
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn linkify_wraps_urls() {
        let text = "Visit https://agentralabs.com for more info.";
        let linked = super::linkify(text);
        assert!(linked.contains("\x1b]8;;https://agentralabs.com"));
        assert!(linked.contains("\x1b]8;;\x07"));
    }

    #[test]
    fn inline_code_renders() {
        let t = crate::theme::Theme::dark();
        let lines = render_assistant_text("Use `cargo build` to compile", &t);
        assert_eq!(lines.len(), 1);
    }
}
