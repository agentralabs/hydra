use crate::colors;

pub fn print_header(text: &str) {
    println!("{} {}", colors::blue("\u{25c9}"), colors::bold(text));
}

pub fn print_success(text: &str) {
    println!("  {} {}", colors::green("\u{2713}"), text);
}

pub fn print_error(text: &str) {
    println!("  {} {}", colors::red("\u{2717}"), text);
}

pub fn print_warning(text: &str) {
    println!("  {} {}", colors::yellow("\u{26a0}"), text);
}

pub fn print_info(text: &str) {
    println!("  {} {}", colors::blue("\u{2139}"), text);
}

pub fn print_dimmed(text: &str) {
    println!("  {}", colors::dim(text));
}

pub fn print_kv(key: &str, value: &str) {
    println!("  {} {}", colors::dim(&format!("{}:", key)), value);
}

pub fn print_table(headers: &[&str], rows: &[Vec<String>]) {
    if headers.is_empty() {
        return;
    }

    // Calculate column widths
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < widths.len() {
                // Strip ANSI codes for width calculation
                let visible_len = strip_ansi(cell).len();
                if visible_len > widths[i] {
                    widths[i] = visible_len;
                }
            }
        }
    }

    // Print header
    let header_line: Vec<String> = headers
        .iter()
        .zip(&widths)
        .map(|(h, w)| format!("{:<width$}", h, width = w))
        .collect();
    println!("  {}", colors::bold(&header_line.join("  ")));

    // Print separator
    let sep: Vec<String> = widths.iter().map(|w| "\u{2500}".repeat(*w)).collect();
    println!("  {}", colors::dim(&sep.join("  ")));

    // Print rows
    for row in rows {
        let cells: Vec<String> = row
            .iter()
            .zip(&widths)
            .map(|(c, w)| {
                let visible_len = strip_ansi(c).len();
                let padding = if visible_len < *w { *w - visible_len } else { 0 };
                format!("{}{}", c, " ".repeat(padding))
            })
            .collect();
        println!("  {}", cells.join("  "));
    }
}

pub fn print_box(lines: &[&str]) {
    if lines.is_empty() {
        return;
    }

    let max_width = lines
        .iter()
        .map(|l| strip_ansi(l).len())
        .max()
        .unwrap_or(0);
    let width = max_width + 2; // 1 space padding each side

    println!("  \u{250c}{}\u{2510}", "\u{2500}".repeat(width));
    for line in lines {
        let visible_len = strip_ansi(line).len();
        let padding = if visible_len < max_width {
            max_width - visible_len
        } else {
            0
        };
        println!("  \u{2502} {}{} \u{2502}", line, " ".repeat(padding));
    }
    println!("  \u{2514}{}\u{2518}", "\u{2500}".repeat(width));
}

pub fn print_progress_story(steps: &[(&str, bool)]) {
    for (i, (label, done)) in steps.iter().enumerate() {
        let is_last = i == steps.len() - 1;
        let icon = if *done {
            colors::green("\u{2713}")
        } else if !is_last && !done {
            colors::yellow("\u{25c9}")
        } else {
            colors::dim("\u{25cb}")
        };
        println!("  {} {}", icon, label);
    }
}

pub fn format_tokens(tokens: u64) -> String {
    if tokens == 0 {
        return "0".to_string();
    }

    let s = tokens.to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    for (i, ch) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(*ch);
    }
    result
}

pub fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else if max <= 3 {
        s[..max].to_string()
    } else {
        format!("{}...", &s[..max - 3])
    }
}

fn strip_ansi(s: &str) -> String {
    let mut result = String::new();
    let mut in_escape = false;
    for ch in s.chars() {
        if ch == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if ch == 'm' {
                in_escape = false;
            }
        } else {
            result.push(ch);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_table_handles_empty_headers() {
        print_table(&[], &[]);
    }

    #[test]
    fn print_table_handles_data() {
        let headers = &["Name", "Status"];
        let rows = vec![
            vec!["hydra-core".to_string(), "ok".to_string()],
            vec!["hydra-cli".to_string(), "ok".to_string()],
        ];
        print_table(headers, &rows);
    }

    #[test]
    fn format_tokens_basic() {
        assert_eq!(format_tokens(0), "0");
        assert_eq!(format_tokens(42), "42");
        assert_eq!(format_tokens(999), "999");
        assert_eq!(format_tokens(1000), "1,000");
        assert_eq!(format_tokens(1234), "1,234");
        assert_eq!(format_tokens(1_000_000), "1,000,000");
    }

    #[test]
    fn truncate_short_string() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_exact_length() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn truncate_with_ellipsis() {
        assert_eq!(truncate("hello world", 8), "hello...");
    }

    #[test]
    fn strip_ansi_removes_codes() {
        let colored = "\x1b[32mhello\x1b[0m";
        assert_eq!(strip_ansi(colored), "hello");
    }

    #[test]
    fn print_box_does_not_panic() {
        print_box(&["line one", "line two is longer"]);
    }

    #[test]
    fn print_box_empty() {
        print_box(&[]);
    }
}
