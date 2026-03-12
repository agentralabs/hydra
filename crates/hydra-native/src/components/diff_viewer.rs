//! Diff viewer component — side-by-side diff rendering.

use serde::{Deserialize, Serialize};

/// Type of a diff line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffLineKind {
    Unchanged,
    Added,
    Removed,
    Header,
}

/// A single line in the diff view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub old_number: Option<usize>,
    pub new_number: Option<usize>,
    pub content: String,
}

/// A diff hunk (section with changes).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
    pub collapsed: bool,
}

/// A file diff view model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub file_path: String,
    pub language: Option<String>,
    pub hunks: Vec<DiffHunk>,
    pub additions: usize,
    pub deletions: usize,
}

impl FileDiff {
    /// Parse a unified diff string into a FileDiff.
    pub fn from_unified_diff(path: &str, diff_text: &str, language: Option<&str>) -> Self {
        let mut hunks = Vec::new();
        let mut current_hunk: Option<DiffHunk> = None;
        let mut additions = 0;
        let mut deletions = 0;
        let mut old_line = 0usize;
        let mut new_line = 0usize;

        for line in diff_text.lines() {
            if line.starts_with("@@") {
                if let Some(hunk) = current_hunk.take() {
                    hunks.push(hunk);
                }
                // Parse hunk header: @@ -old_start,old_count +new_start,new_count @@
                let (o, n) = parse_hunk_header(line);
                old_line = o;
                new_line = n;
                current_hunk = Some(DiffHunk {
                    header: line.to_string(),
                    lines: Vec::new(),
                    collapsed: false,
                });
            } else if let Some(ref mut hunk) = current_hunk {
                if let Some(content) = line.strip_prefix('+') {
                    hunk.lines.push(DiffLine {
                        kind: DiffLineKind::Added,
                        old_number: None,
                        new_number: Some(new_line),
                        content: content.to_string(),
                    });
                    new_line += 1;
                    additions += 1;
                } else if let Some(content) = line.strip_prefix('-') {
                    hunk.lines.push(DiffLine {
                        kind: DiffLineKind::Removed,
                        old_number: Some(old_line),
                        new_number: None,
                        content: content.to_string(),
                    });
                    old_line += 1;
                    deletions += 1;
                } else {
                    let content = line.strip_prefix(' ').unwrap_or(line);
                    hunk.lines.push(DiffLine {
                        kind: DiffLineKind::Unchanged,
                        old_number: Some(old_line),
                        new_number: Some(new_line),
                        content: content.to_string(),
                    });
                    old_line += 1;
                    new_line += 1;
                }
            }
        }

        if let Some(hunk) = current_hunk {
            hunks.push(hunk);
        }

        Self {
            file_path: path.to_string(),
            language: language.map(|s| s.to_string()),
            hunks,
            additions,
            deletions,
        }
    }

    /// Total changed lines.
    pub fn total_changes(&self) -> usize {
        self.additions + self.deletions
    }

    /// Summary string (e.g. "+12 -3").
    pub fn summary(&self) -> String {
        format!("+{} -{}", self.additions, self.deletions)
    }

    /// CSS class for the addition/deletion indicator.
    pub fn line_css_class(kind: DiffLineKind) -> &'static str {
        match kind {
            DiffLineKind::Added => "diff-added",
            DiffLineKind::Removed => "diff-removed",
            DiffLineKind::Unchanged => "diff-unchanged",
            DiffLineKind::Header => "diff-header",
        }
    }

    /// Toggle collapse on a hunk.
    pub fn toggle_hunk(&mut self, index: usize) {
        if let Some(hunk) = self.hunks.get_mut(index) {
            hunk.collapsed = !hunk.collapsed;
        }
    }
}

/// Parse a hunk header like "@@ -1,5 +1,7 @@" into (old_start, new_start).
fn parse_hunk_header(line: &str) -> (usize, usize) {
    let mut old_start = 1;
    let mut new_start = 1;

    if let Some(rest) = line.strip_prefix("@@ -") {
        let parts: Vec<&str> = rest.splitn(2, " +").collect();
        if let Some(old_part) = parts.first() {
            if let Some(start) = old_part.split(',').next() {
                old_start = start.parse().unwrap_or(1);
            }
        }
        if let Some(new_part) = parts.get(1) {
            if let Some(start) = new_part.split(',').next() {
                new_start = start.parse().unwrap_or(1);
            }
        }
    }

    (old_start, new_start)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_DIFF: &str = "\
@@ -1,5 +1,7 @@
 fn main() {
-    println!(\"hello\");
+    println!(\"hello world\");
+    println!(\"goodbye\");
 }
";

    #[test]
    fn test_parse_diff() {
        let diff = FileDiff::from_unified_diff("src/main.rs", SAMPLE_DIFF, Some("rust"));
        assert_eq!(diff.file_path, "src/main.rs");
        assert_eq!(diff.language, Some("rust".into()));
        assert_eq!(diff.additions, 2);
        assert_eq!(diff.deletions, 1);
        assert_eq!(diff.hunks.len(), 1);
    }

    #[test]
    fn test_diff_summary() {
        let diff = FileDiff::from_unified_diff("test.rs", SAMPLE_DIFF, None);
        assert_eq!(diff.summary(), "+2 -1");
        assert_eq!(diff.total_changes(), 3);
    }

    #[test]
    fn test_diff_line_kinds() {
        let diff = FileDiff::from_unified_diff("test.rs", SAMPLE_DIFF, None);
        let lines = &diff.hunks[0].lines;
        assert_eq!(lines[0].kind, DiffLineKind::Unchanged);
        assert_eq!(lines[1].kind, DiffLineKind::Removed);
        assert_eq!(lines[2].kind, DiffLineKind::Added);
        assert_eq!(lines[3].kind, DiffLineKind::Added);
        assert_eq!(lines[4].kind, DiffLineKind::Unchanged);
    }

    #[test]
    fn test_diff_line_numbers() {
        let diff = FileDiff::from_unified_diff("test.rs", SAMPLE_DIFF, None);
        let lines = &diff.hunks[0].lines;
        // First unchanged line
        assert_eq!(lines[0].old_number, Some(1));
        assert_eq!(lines[0].new_number, Some(1));
        // Removed line
        assert_eq!(lines[1].old_number, Some(2));
        assert_eq!(lines[1].new_number, None);
        // Added lines
        assert_eq!(lines[2].old_number, None);
        assert_eq!(lines[2].new_number, Some(2));
    }

    #[test]
    fn test_toggle_hunk() {
        let mut diff = FileDiff::from_unified_diff("test.rs", SAMPLE_DIFF, None);
        assert!(!diff.hunks[0].collapsed);
        diff.toggle_hunk(0);
        assert!(diff.hunks[0].collapsed);
        diff.toggle_hunk(0);
        assert!(!diff.hunks[0].collapsed);
    }

    #[test]
    fn test_css_class() {
        assert_eq!(FileDiff::line_css_class(DiffLineKind::Added), "diff-added");
        assert_eq!(FileDiff::line_css_class(DiffLineKind::Removed), "diff-removed");
        assert_eq!(FileDiff::line_css_class(DiffLineKind::Unchanged), "diff-unchanged");
    }

    #[test]
    fn test_parse_hunk_header() {
        let (old, new) = parse_hunk_header("@@ -10,5 +12,7 @@ fn something()");
        assert_eq!(old, 10);
        assert_eq!(new, 12);
    }

    #[test]
    fn test_empty_diff() {
        let diff = FileDiff::from_unified_diff("empty.rs", "", None);
        assert_eq!(diff.hunks.len(), 0);
        assert_eq!(diff.additions, 0);
        assert_eq!(diff.deletions, 0);
    }
}
