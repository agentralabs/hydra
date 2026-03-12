#[cfg(test)]
mod tests {
    use crate::utils::markdown::*;

    #[test]
    fn test_escape_html() {
        assert_eq!(escape_html("<b>\"hi\" & bye</b>"), "&lt;b&gt;&quot;hi&quot; &amp; bye&lt;/b&gt;");
    }

    #[test]
    fn test_heading_h1() {
        assert_eq!(markdown_to_html("# Hello"), "<h1>Hello</h1>\n");
    }

    #[test]
    fn test_heading_h3() {
        assert_eq!(markdown_to_html("### Third"), "<h3>Third</h3>\n");
    }

    #[test]
    fn test_heading_h6() {
        assert_eq!(markdown_to_html("###### Deepest"), "<h6>Deepest</h6>\n");
    }

    #[test]
    fn test_bold() {
        assert_eq!(
            markdown_to_html("This is **bold** text"),
            "<p>This is <strong>bold</strong> text</p>\n"
        );
    }

    #[test]
    fn test_italic() {
        assert_eq!(
            markdown_to_html("This is *italic* text"),
            "<p>This is <em>italic</em> text</p>\n"
        );
    }

    #[test]
    fn test_inline_code() {
        assert_eq!(
            markdown_to_html("Use `cargo test` here"),
            "<p>Use <code>cargo test</code> here</p>\n"
        );
    }

    #[test]
    fn test_code_block_with_language() {
        let input = "```rust\nfn main() {}\n```";
        let expected = "<pre><code class=\"language-rust\">fn main() {}</code></pre>\n";
        assert_eq!(markdown_to_html(input), expected);
    }

    #[test]
    fn test_code_block_html_escaping() {
        let input = "```html\n<div class=\"a\">&</div>\n```";
        let expected =
            "<pre><code class=\"language-html\">&lt;div class=&quot;a&quot;&gt;&amp;&lt;/div&gt;</code></pre>\n";
        assert_eq!(markdown_to_html(input), expected);
    }

    #[test]
    fn test_code_block_no_language() {
        let input = "```\nplain code\n```";
        let expected = "<pre><code>plain code</code></pre>\n";
        assert_eq!(markdown_to_html(input), expected);
    }

    #[test]
    fn test_link() {
        assert_eq!(
            markdown_to_html("Visit [Rust](https://rust-lang.org)"),
            "<p>Visit <a href=\"https://rust-lang.org\" target=\"_blank\" rel=\"noopener\">Rust</a></p>\n"
        );
    }

    #[test]
    fn test_unordered_list() {
        let input = "- Alpha\n- Beta\n- Gamma";
        let expected = "<ul>\n<li>Alpha</li>\n<li>Beta</li>\n<li>Gamma</li>\n</ul>\n";
        assert_eq!(markdown_to_html(input), expected);
    }

    #[test]
    fn test_ordered_list() {
        let input = "1. First\n2. Second\n3. Third";
        let expected = "<ol>\n<li>First</li>\n<li>Second</li>\n<li>Third</li>\n</ol>\n";
        assert_eq!(markdown_to_html(input), expected);
    }

    #[test]
    fn test_blockquote() {
        let input = "> This is a quote";
        let expected = "<blockquote>This is a quote</blockquote>\n";
        assert_eq!(markdown_to_html(input), expected);
    }

    #[test]
    fn test_horizontal_rule() {
        assert_eq!(markdown_to_html("---"), "<hr>\n");
        assert_eq!(markdown_to_html("***"), "<hr>\n");
        assert_eq!(markdown_to_html("___"), "<hr>\n");
    }

    #[test]
    fn test_table() {
        let input = "| Name | Age |\n| --- | --- |\n| Alice | 30 |\n| Bob | 25 |";
        let html = markdown_to_html(input);
        assert!(html.contains("<table>"));
        assert!(html.contains("<th>Name</th>"));
        assert!(html.contains("<th>Age</th>"));
        assert!(html.contains("<td>Alice</td>"));
        assert!(html.contains("<td>30</td>"));
        assert!(html.contains("<td>Bob</td>"));
        assert!(html.contains("</table>"));
    }

    #[test]
    fn test_paragraph() {
        assert_eq!(markdown_to_html("Just text"), "<p>Just text</p>\n");
    }

    #[test]
    fn test_blank_lines_ignored() {
        let input = "# Title\n\nBody text";
        let expected = "<h1>Title</h1>\n<p>Body text</p>\n";
        assert_eq!(markdown_to_html(input), expected);
    }

    #[test]
    fn test_mixed_inline() {
        let input = "Use **bold** and *italic* and `code`";
        let html = markdown_to_html(input);
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<em>italic</em>"));
        assert!(html.contains("<code>code</code>"));
    }

    #[test]
    fn test_link_with_special_chars() {
        let input = "[a&b](https://example.com?a=1&b=2)";
        let html = markdown_to_html(input);
        assert!(html.contains("href=\"https://example.com?a=1&amp;b=2\""));
        assert!(html.contains("a&amp;b</a>"));
    }

    #[test]
    fn test_heading_without_space_not_parsed() {
        // "#word" without space after # should not be a heading
        let html = markdown_to_html("#nospace");
        assert_eq!(html, "<p>#nospace</p>\n");
    }

    #[test]
    fn test_multiline_blockquote() {
        let input = "> Line one\n> Line two";
        let html = markdown_to_html(input);
        assert!(html.contains("<blockquote>"));
        assert!(html.contains("Line one"));
        assert!(html.contains("Line two"));
    }

    #[test]
    fn test_empty_input() {
        assert_eq!(markdown_to_html(""), "");
    }

    #[test]
    fn test_inline_code_escapes_html() {
        let html = markdown_to_html("Use `<div>`");
        assert!(html.contains("<code>&lt;div&gt;</code>"));
    }
}
