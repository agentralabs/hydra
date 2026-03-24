//! Markdown rendering for assistant text — uses t.assistant_text for readable color.
//!
//! Handles: code blocks, **bold**, `inline code`, tables, blockquotes,
//! numbered/bullet lists, horizontal rules, OSC 8 hyperlinks.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::constants;
use crate::theme::Theme;

/// Render assistant text with markdown support.
pub fn render_assistant_text(text: &str, t: &Theme) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut in_code_block = false;
    let mut code_lang = String::new();
    let (gr, gg, gb) = constants::HYDRA_GREEN;
    let green = Color::Rgb(gr, gg, gb);

    for raw_line in text.lines() {
        let trimmed = raw_line.trim_start();

        // Code block fences
        if trimmed.starts_with("```") {
            if in_code_block {
                lines.push(Line::from(Span::styled("  └───", Style::default().fg(t.fg_muted))));
                in_code_block = false;
                code_lang.clear();
            } else {
                code_lang = trimmed.trim_start_matches('`').to_string();
                let label = if code_lang.is_empty() { "code".to_string() } else { code_lang.clone() };
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
                Style::default().fg(green).bg(t.bg_secondary),
            )));
            continue;
        }

        // Horizontal rule: --- or *** or ___
        if trimmed.len() >= 3 && (trimmed.chars().all(|c| c == '-') || trimmed.chars().all(|c| c == '*') || trimmed.chars().all(|c| c == '_')) {
            let rule_width = 40.min(raw_line.len().max(20));
            lines.push(Line::from(Span::styled(
                format!("  {}", "─".repeat(rule_width)),
                Style::default().fg(t.dim),
            )));
            continue;
        }

        // Blockquote: > text → │ text
        if trimmed.starts_with("> ") {
            let quote_text = trimmed.strip_prefix("> ").unwrap_or(trimmed);
            lines.push(Line::from(vec![
                Span::styled("  │ ", Style::default().fg(t.dim)),
                Span::styled(quote_text.to_string(), Style::default().fg(t.assistant_text)),
            ]));
            continue;
        }

        // Table rows: lines with |
        if trimmed.starts_with('|') && trimmed.ends_with('|') {
            // Separator row (|---|---|)
            if trimmed.contains("---") {
                let cols = trimmed.split('|').filter(|s| !s.is_empty()).count();
                let sep = format!("  ├{}┤", vec!["───────"; cols].join("┼"));
                lines.push(Line::from(Span::styled(sep, Style::default().fg(t.dim))));
            } else {
                let cells: Vec<&str> = trimmed.split('|').filter(|s| !s.is_empty()).collect();
                let mut spans = vec![Span::styled("  │", Style::default().fg(t.dim))];
                for cell in cells {
                    spans.push(Span::styled(format!(" {} ", cell.trim()), Style::default().fg(t.assistant_text)));
                    spans.push(Span::styled("│", Style::default().fg(t.dim)));
                }
                lines.push(Line::from(spans));
            }
            continue;
        }

        // Numbered lists: 1. text
        if let Some(rest) = try_numbered_list(trimmed) {
            let (num, content) = rest;
            let (br, bg, bb) = constants::HYDRA_BLUE;
            lines.push(Line::from(vec![
                Span::styled(format!("  {num}. "), Style::default().fg(Color::Rgb(br, bg, bb))),
                Span::styled(content.to_string(), Style::default().fg(t.assistant_text)),
            ]));
            continue;
        }

        // Bullet lists: - text or * text
        if (trimmed.starts_with("- ") || trimmed.starts_with("* ")) && trimmed.len() > 2 {
            let content = &trimmed[2..];
            let (br, bg, bb) = constants::HYDRA_BLUE;
            lines.push(Line::from(vec![
                Span::styled("  • ", Style::default().fg(Color::Rgb(br, bg, bb))),
                Span::styled(content.to_string(), Style::default().fg(t.assistant_text)),
            ]));
            continue;
        }

        // Normal text with inline markdown
        let line = render_inline_markdown(raw_line, t);
        lines.push(line);
    }

    if in_code_block {
        lines.push(Line::from(Span::styled("  └───", Style::default().fg(t.fg_muted))));
    }

    if lines.is_empty() {
        lines.push(Line::from(""));
    }
    lines
}

/// Try to parse a numbered list item like "1. text". Returns (number, rest).
fn try_numbered_list(line: &str) -> Option<(&str, &str)> {
    let dot_pos = line.find(". ")?;
    let num = &line[..dot_pos];
    if num.len() > 3 || !num.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some((num, &line[dot_pos + 2..]))
}

/// Render inline markdown: **bold**, `code`.
fn render_inline_markdown(line: &str, t: &Theme) -> Line<'static> {
    let mut spans = Vec::new();
    let mut chars = line.chars().peekable();
    let mut current = String::new();
    let text_style = Style::default().fg(t.assistant_text);
    let (gr, gg, gb) = constants::HYDRA_GREEN;

    while let Some(ch) = chars.next() {
        match ch {
            '`' => {
                if !current.is_empty() {
                    spans.push(Span::styled(current.clone(), text_style));
                    current.clear();
                }
                let mut code = String::new();
                for c in chars.by_ref() {
                    if c == '`' { break; }
                    code.push(c);
                }
                spans.push(Span::styled(
                    format!("`{code}`"),
                    Style::default().fg(Color::Rgb(gr, gg, gb)).bg(t.bg_secondary),
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
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        let linked = linkify(&current);
        spans.push(Span::styled(linked, text_style));
    }

    if spans.is_empty() { Line::from("") } else { Line::from(spans) }
}

/// Detect URLs in text and wrap with OSC 8 hyperlink sequences.
pub fn linkify(text: &str) -> String {
    let mut result = String::new();
    let mut remaining = text;

    while let Some(start) = remaining.find("https://").or_else(|| remaining.find("http://")) {
        result.push_str(&remaining[..start]);
        let url_text = &remaining[start..];
        let end = url_text
            .find(|c: char| c.is_whitespace() || c == ')' || c == ']' || c == '>' || c == '"')
            .unwrap_or(url_text.len());
        let url = &url_text[..end];
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
    }

    #[test]
    fn inline_code_renders() {
        let t = crate::theme::Theme::dark();
        let lines = render_assistant_text("Use `cargo build` to compile", &t);
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn bullet_list_renders() {
        let t = crate::theme::Theme::dark();
        let lines = render_assistant_text("- item one\n- item two", &t);
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn numbered_list_renders() {
        let t = crate::theme::Theme::dark();
        let lines = render_assistant_text("1. first\n2. second", &t);
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn horizontal_rule_renders() {
        let t = crate::theme::Theme::dark();
        let lines = render_assistant_text("above\n---\nbelow", &t);
        assert_eq!(lines.len(), 3);
    }
}
