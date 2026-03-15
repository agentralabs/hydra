//! Line-by-line diff engine using simple LCS algorithm.
//!
//! Computes diffs between old and new file contents,
//! formats them for display, and provides summary statistics.

/// Type of change for a diff line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeType {
    Added,
    Removed,
    Context,
}

/// A single line in a diff output.
#[derive(Debug, Clone)]
pub struct DiffLine {
    pub line_num_old: Option<usize>,
    pub line_num_new: Option<usize>,
    pub content: String,
    pub change_type: ChangeType,
}

/// Compute line-by-line diff using a simple LCS (Longest Common Subsequence) approach.
///
/// Returns a list of `DiffLine` entries representing additions, removals, and context.
pub fn compute_diff(old: &str, new: &str) -> Vec<DiffLine> {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    let m = old_lines.len();
    let n = new_lines.len();

    // Build LCS table
    let mut dp = vec![vec![0u32; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            if old_lines[i - 1] == new_lines[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    // Backtrack to produce diff
    let mut result = Vec::new();
    let mut i = m;
    let mut j = n;
    let mut stack = Vec::new();

    while i > 0 || j > 0 {
        if i > 0 && j > 0 && old_lines[i - 1] == new_lines[j - 1] {
            stack.push(DiffLine {
                line_num_old: Some(i),
                line_num_new: Some(j),
                content: old_lines[i - 1].to_string(),
                change_type: ChangeType::Context,
            });
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
            stack.push(DiffLine {
                line_num_old: None,
                line_num_new: Some(j),
                content: new_lines[j - 1].to_string(),
                change_type: ChangeType::Added,
            });
            j -= 1;
        } else if i > 0 {
            stack.push(DiffLine {
                line_num_old: Some(i),
                line_num_new: None,
                content: old_lines[i - 1].to_string(),
                change_type: ChangeType::Removed,
            });
            i -= 1;
        }
    }

    // Reverse since we built it backwards
    stack.reverse();
    result.extend(stack);
    result
}

/// Format diff lines for human-readable display with +/- prefixes and context.
///
/// Context lines are prefixed with a space, additions with `+`, removals with `-`.
/// Only shows context lines near changes (3 lines of context by default).
pub fn format_diff_display(lines: &[DiffLine]) -> String {
    if lines.is_empty() {
        return String::from("(no changes)");
    }

    let context_radius = 3;
    let mut output = String::new();

    // Mark which lines are near a change
    let change_indices: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.change_type != ChangeType::Context)
        .map(|(i, _)| i)
        .collect();

    if change_indices.is_empty() {
        return String::from("(no changes)");
    }

    let mut visible = vec![false; lines.len()];
    for &ci in &change_indices {
        let start = ci.saturating_sub(context_radius);
        let end = (ci + context_radius + 1).min(lines.len());
        for idx in start..end {
            visible[idx] = true;
        }
    }

    let mut last_shown = None;
    for (i, line) in lines.iter().enumerate() {
        if !visible[i] {
            continue;
        }
        // Insert separator if there's a gap
        if let Some(last) = last_shown {
            if i > last + 1 {
                output.push_str("  ...\n");
            }
        }
        last_shown = Some(i);

        let prefix = match line.change_type {
            ChangeType::Added => "+",
            ChangeType::Removed => "-",
            ChangeType::Context => " ",
        };

        let line_info = match (line.line_num_old, line.line_num_new) {
            (Some(o), Some(n)) => format!("{:>4}:{:<4}", o, n),
            (Some(o), None) => format!("{:>4}:    ", o),
            (None, Some(n)) => format!("    :{:<4}", n),
            (None, None) => "    :    ".to_string(),
        };

        output.push_str(&format!("{} {} {}\n", prefix, line_info, line.content));
    }

    output
}

/// Count (added, removed) lines in a diff.
pub fn diff_summary(lines: &[DiffLine]) -> (usize, usize) {
    let added = lines.iter().filter(|l| l.change_type == ChangeType::Added).count();
    let removed = lines.iter().filter(|l| l.change_type == ChangeType::Removed).count();
    (added, removed)
}

/// Produce a one-line summary string for a diff.
pub fn diff_summary_string(old: &str, new: &str) -> String {
    let lines = compute_diff(old, new);
    let (added, removed) = diff_summary(&lines);
    format!("+{} -{}", added, removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_content() {
        let text = "line1\nline2\nline3";
        let diff = compute_diff(text, text);
        assert!(diff.iter().all(|l| l.change_type == ChangeType::Context));
        let (added, removed) = diff_summary(&diff);
        assert_eq!(added, 0);
        assert_eq!(removed, 0);
    }

    #[test]
    fn test_simple_addition() {
        let old = "line1\nline3";
        let new = "line1\nline2\nline3";
        let diff = compute_diff(old, new);
        let (added, removed) = diff_summary(&diff);
        assert_eq!(added, 1);
        assert_eq!(removed, 0);
        let added_line = diff.iter().find(|l| l.change_type == ChangeType::Added).unwrap();
        assert_eq!(added_line.content, "line2");
    }

    #[test]
    fn test_simple_removal() {
        let old = "line1\nline2\nline3";
        let new = "line1\nline3";
        let diff = compute_diff(old, new);
        let (added, removed) = diff_summary(&diff);
        assert_eq!(added, 0);
        assert_eq!(removed, 1);
    }

    #[test]
    fn test_replacement() {
        let old = "hello\nworld";
        let new = "hello\nearth";
        let diff = compute_diff(old, new);
        let (added, removed) = diff_summary(&diff);
        assert_eq!(added, 1);
        assert_eq!(removed, 1);
    }

    #[test]
    fn test_empty_to_content() {
        let diff = compute_diff("", "line1\nline2");
        let (added, removed) = diff_summary(&diff);
        assert_eq!(added, 2);
        assert_eq!(removed, 0);
    }

    #[test]
    fn test_content_to_empty() {
        let diff = compute_diff("line1\nline2", "");
        let (added, removed) = diff_summary(&diff);
        assert_eq!(added, 0);
        assert_eq!(removed, 2);
    }

    #[test]
    fn test_format_diff_display_no_changes() {
        let text = "line1\nline2";
        let diff = compute_diff(text, text);
        let formatted = format_diff_display(&diff);
        assert_eq!(formatted, "(no changes)");
    }

    #[test]
    fn test_format_diff_display_with_changes() {
        let old = "line1\nold_line\nline3";
        let new = "line1\nnew_line\nline3";
        let diff = compute_diff(old, new);
        let formatted = format_diff_display(&diff);
        assert!(formatted.contains("-"));
        assert!(formatted.contains("+"));
        assert!(formatted.contains("old_line"));
        assert!(formatted.contains("new_line"));
    }

    #[test]
    fn test_diff_summary_string() {
        let old = "a\nb\nc";
        let new = "a\nx\ny\nc";
        let summary = diff_summary_string(old, new);
        assert!(summary.starts_with('+'));
        assert!(summary.contains('-'));
    }

    #[test]
    fn test_both_empty() {
        let diff = compute_diff("", "");
        assert!(diff.is_empty());
        let (added, removed) = diff_summary(&diff);
        assert_eq!(added, 0);
        assert_eq!(removed, 0);
    }
}
