//! Hand-rolled Markdown-to-HTML converter.
//!
//! Supports a practical subset of Markdown without external dependencies:
//! headers, bold, italic, code blocks, inline code, links, unordered and
//! ordered lists, blockquotes, horizontal rules, and tables.

/// Escape special HTML characters.
pub fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Convert a Markdown string to HTML.
pub fn markdown_to_html(input: &str) -> String {
    let lines: Vec<&str> = input.lines().collect();
    let mut html = String::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // Fenced code block
        if line.trim_start().starts_with("```") {
            let indent = line.len() - line.trim_start().len();
            let after_ticks = line.trim_start().trim_start_matches('`');
            let lang = after_ticks.trim();
            let class_attr = if lang.is_empty() {
                String::new()
            } else {
                format!(" class=\"language-{}\"", escape_html(lang))
            };
            i += 1;
            let mut code = String::new();
            while i < lines.len() {
                let cl = lines[i];
                if cl.trim_start().starts_with("```")
                    && cl.len() - cl.trim_start().len() <= indent + 4
                {
                    break;
                }
                if !code.is_empty() {
                    code.push('\n');
                }
                code.push_str(cl);
                i += 1;
            }
            html.push_str(&format!(
                "<pre><code{}>{}</code></pre>\n",
                class_attr,
                escape_html(&code)
            ));
            i += 1; // skip closing ```
            continue;
        }

        // Horizontal rule
        if is_horizontal_rule(line) {
            html.push_str("<hr>\n");
            i += 1;
            continue;
        }

        // Table
        if is_table_row(line) && i + 1 < lines.len() && is_table_separator(lines[i + 1]) {
            html.push_str("<table>\n<thead>\n<tr>");
            for cell in parse_table_cells(line) {
                html.push_str(&format!("<th>{}</th>", inline_markup(cell.trim())));
            }
            html.push_str("</tr>\n</thead>\n<tbody>\n");
            i += 2; // skip header + separator
            while i < lines.len() && is_table_row(lines[i]) {
                html.push_str("<tr>");
                for cell in parse_table_cells(lines[i]) {
                    html.push_str(&format!("<td>{}</td>", inline_markup(cell.trim())));
                }
                html.push_str("</tr>\n");
                i += 1;
            }
            html.push_str("</tbody>\n</table>\n");
            continue;
        }

        // Blockquote
        if line.starts_with("> ") || line == ">" {
            let mut bq_lines = Vec::new();
            while i < lines.len()
                && (lines[i].starts_with("> ") || lines[i] == ">")
            {
                let content = if lines[i] == ">" {
                    ""
                } else {
                    &lines[i][2..]
                };
                bq_lines.push(content);
                i += 1;
            }
            let inner = bq_lines.join("\n");
            html.push_str(&format!(
                "<blockquote>{}</blockquote>\n",
                inline_markup(&inner)
            ));
            continue;
        }

        // Heading
        if let Some(heading) = parse_heading(line) {
            html.push_str(&heading);
            html.push('\n');
            i += 1;
            continue;
        }

        // Unordered list
        if line.starts_with("- ") || line.starts_with("* ") {
            html.push_str("<ul>\n");
            while i < lines.len()
                && (lines[i].starts_with("- ") || lines[i].starts_with("* "))
            {
                let content = &lines[i][2..];
                html.push_str(&format!("<li>{}</li>\n", inline_markup(content)));
                i += 1;
            }
            html.push_str("</ul>\n");
            continue;
        }

        // Ordered list
        if let Some(_) = parse_ordered_list_item(line) {
            html.push_str("<ol>\n");
            while i < lines.len() {
                if let Some(content) = parse_ordered_list_item(lines[i]) {
                    html.push_str(&format!("<li>{}</li>\n", inline_markup(content)));
                    i += 1;
                } else {
                    break;
                }
            }
            html.push_str("</ol>\n");
            continue;
        }

        // Blank line
        if line.trim().is_empty() {
            i += 1;
            continue;
        }

        // Paragraph (default)
        html.push_str(&format!("<p>{}</p>\n", inline_markup(line)));
        i += 1;
    }

    html
}

/// Parse a heading line (# through ######).
fn parse_heading(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('#') {
        return None;
    }
    let level = trimmed.chars().take_while(|&c| c == '#').count();
    if level == 0 || level > 6 {
        return None;
    }
    let rest = &trimmed[level..];
    if !rest.is_empty() && !rest.starts_with(' ') {
        return None;
    }
    let content = rest.trim();
    Some(format!("<h{level}>{}</h{level}>", inline_markup(content)))
}

/// Check whether a line is a horizontal rule (---, ***, ___).
fn is_horizontal_rule(line: &str) -> bool {
    let t = line.trim();
    if t.len() < 3 {
        return false;
    }
    let chars: Vec<char> = t.chars().collect();
    let first = chars[0];
    (first == '-' || first == '*' || first == '_')
        && chars.iter().all(|&c| c == first || c == ' ')
        && chars.iter().filter(|&&c| c == first).count() >= 3
}

/// Check whether a line looks like a table row (starts and ends with |).
fn is_table_row(line: &str) -> bool {
    let t = line.trim();
    t.starts_with('|') && t.ends_with('|') && t.len() > 2
}

/// Check whether a line is a table separator (| --- | --- |).
fn is_table_separator(line: &str) -> bool {
    let t = line.trim();
    if !t.starts_with('|') || !t.ends_with('|') {
        return false;
    }
    t[1..t.len() - 1]
        .split('|')
        .all(|seg| {
            let s = seg.trim();
            !s.is_empty() && s.chars().all(|c| c == '-' || c == ':' || c == ' ')
        })
}

/// Split a table row into cells (strips outer pipes).
fn parse_table_cells(line: &str) -> Vec<&str> {
    let t = line.trim();
    let inner = &t[1..t.len() - 1]; // strip outer |
    inner.split('|').collect()
}

/// Parse an ordered list item, returning the content after `N. `.
fn parse_ordered_list_item(line: &str) -> Option<&str> {
    let t = line.trim_start();
    let num_end = t.find(". ")?;
    let prefix = &t[..num_end];
    if prefix.is_empty() || !prefix.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(&t[num_end + 2..])
}

/// Apply inline markup: bold, italic, inline code, links.
fn inline_markup(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Inline code
        if chars[i] == '`' {
            if let Some(end) = find_char(&chars, '`', i + 1) {
                let code_text: String = chars[i + 1..end].iter().collect();
                result.push_str(&format!("<code>{}</code>", escape_html(&code_text)));
                i = end + 1;
                continue;
            }
        }

        // Link [text](url)
        if chars[i] == '[' {
            if let Some((text, url, end)) = parse_link(&chars, i) {
                result.push_str(&format!(
                    "<a href=\"{}\" target=\"_blank\" rel=\"noopener\">{}</a>",
                    escape_html(&url),
                    escape_html(&text)
                ));
                i = end;
                continue;
            }
        }

        // Bold **text**
        if i + 1 < len && chars[i] == '*' && chars[i + 1] == '*' {
            if let Some(end) = find_double_char(&chars, '*', i + 2) {
                let inner: String = chars[i + 2..end].iter().collect();
                result.push_str(&format!("<strong>{}</strong>", escape_html(&inner)));
                i = end + 2;
                continue;
            }
        }

        // Italic *text*
        if chars[i] == '*' {
            if let Some(end) = find_single_not_double(&chars, '*', i + 1) {
                let inner: String = chars[i + 1..end].iter().collect();
                result.push_str(&format!("<em>{}</em>", escape_html(&inner)));
                i = end + 1;
                continue;
            }
        }

        // Plain character — escape HTML
        match chars[i] {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            c => result.push(c),
        }
        i += 1;
    }

    result
}

/// Find the next occurrence of `ch` starting at `from`.
fn find_char(chars: &[char], ch: char, from: usize) -> Option<usize> {
    (from..chars.len()).find(|&j| chars[j] == ch)
}

/// Find closing `**` starting at `from`.
fn find_double_char(chars: &[char], ch: char, from: usize) -> Option<usize> {
    let mut j = from;
    while j + 1 < chars.len() {
        if chars[j] == ch && chars[j + 1] == ch {
            return Some(j);
        }
        j += 1;
    }
    None
}

/// Find a single `*` that is not part of `**`.
fn find_single_not_double(chars: &[char], ch: char, from: usize) -> Option<usize> {
    let mut j = from;
    while j < chars.len() {
        if chars[j] == ch {
            if j + 1 < chars.len() && chars[j + 1] == ch {
                j += 2; // skip **
                continue;
            }
            return Some(j);
        }
        j += 1;
    }
    None
}

/// Parse `[text](url)` starting at position `start` (which points to `[`).
fn parse_link(chars: &[char], start: usize) -> Option<(String, String, usize)> {
    let close_bracket = find_char(chars, ']', start + 1)?;
    if close_bracket + 1 >= chars.len() || chars[close_bracket + 1] != '(' {
        return None;
    }
    let close_paren = find_char(chars, ')', close_bracket + 2)?;
    let text: String = chars[start + 1..close_bracket].iter().collect();
    let url: String = chars[close_bracket + 2..close_paren].iter().collect();
    Some((text, url, close_paren + 1))
}

#[cfg(test)]
#[path = "markdown_tests.rs"]
mod tests;
