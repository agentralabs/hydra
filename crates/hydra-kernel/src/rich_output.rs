//! O22 Rich Output — classify LLM responses into structured visual types.
//! Detects tables, charts, code blocks, timelines from text patterns.
//! Classifier runs on every response; renderers in TUI view/rich.rs.

// ── Types ──

/// Rich output types that can be rendered visually in the TUI.
#[derive(Debug, Clone)]
pub enum RichOutput {
    Text(String),
    Table { headers: Vec<String>, rows: Vec<Vec<String>> },
    Chart { title: String, labels: Vec<String>, values: Vec<f64>, unit: String },
    Timeline { events: Vec<(String, String)> },
    Progress { tasks: Vec<(String, f64)> },
    CodeBlock { language: String, content: String },
    /// Unified diff with color-coded add/remove lines.
    Diff { hunks: Vec<DiffHunk> },
}

/// A hunk in a unified diff.
#[derive(Debug, Clone)]
pub struct DiffHunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
}

/// A single line in a diff hunk.
#[derive(Debug, Clone)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub content: String,
}

/// Kind of diff line.
#[derive(Debug, Clone, PartialEq)]
pub enum DiffLineKind { Add, Remove, Context, Header }

impl RichOutput {
    pub fn type_label(&self) -> &'static str {
        match self {
            Self::Text(_) => "text", Self::Table { .. } => "table",
            Self::Chart { .. } => "chart", Self::Timeline { .. } => "timeline",
            Self::Progress { .. } => "progress", Self::CodeBlock { .. } => "code",
            Self::Diff { .. } => "diff",
        }
    }
}

// ── Classifier ──

/// Classify LLM response text into the best rich output type.
pub fn classify_output(text: &str) -> RichOutput {
    // 0. Unified diff?
    if let Some(hunks) = extract_diff(text) {
        return RichOutput::Diff { hunks };
    }
    // 1. Markdown table?
    if let Some((headers, rows)) = extract_table(text) {
        return RichOutput::Table { headers, rows };
    }
    // 2. Code block?
    if let Some((lang, code)) = extract_code_block(text) {
        return RichOutput::CodeBlock { language: lang, content: code };
    }
    // 3. Numeric trend?
    if let Some((labels, values)) = extract_numeric_pairs(text) {
        return RichOutput::Chart {
            title: String::new(), labels, values, unit: String::new(),
        };
    }
    // 4. Timeline / sequential events?
    if let Some(events) = extract_timeline(text) {
        return RichOutput::Timeline { events };
    }
    // Default: plain text
    RichOutput::Text(text.to_string())
}

// ── Extractors ──

/// Extract markdown table: lines with `|` separators.
pub fn extract_table(text: &str) -> Option<(Vec<String>, Vec<Vec<String>>)> {
    let lines: Vec<&str> = text.lines().filter(|l| l.contains('|') && l.trim().len() > 3).collect();
    if lines.len() < 2 { return None; }
    let headers: Vec<String> = lines[0].split('|')
        .map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    if headers.len() < 2 { return None; }
    // Skip separator line (---|---)
    let data_start = if lines.get(1).map(|l| l.contains("---")).unwrap_or(false) { 2 } else { 1 };
    let rows: Vec<Vec<String>> = lines[data_start..].iter()
        .map(|l| l.split('|').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
        .filter(|r: &Vec<String>| r.len() >= headers.len().saturating_sub(1))
        .collect();
    if rows.is_empty() { return None; }
    Some((headers, rows))
}

/// Extract fenced code block: ```language ... ```
pub fn extract_code_block(text: &str) -> Option<(String, String)> {
    let start = text.find("```")?;
    let after_fence = &text[start + 3..];
    let lang_end = after_fence.find('\n')?;
    let lang = after_fence[..lang_end].trim().to_string();
    let rest = &after_fence[lang_end + 1..];
    let end = rest.find("```")?;
    let code = rest[..end].trim().to_string();
    if code.len() < 10 { return None; }
    Some((if lang.is_empty() { "text".into() } else { lang }, code))
}

/// Extract label: value numeric pairs (e.g., "January: $120K").
pub fn extract_numeric_pairs(text: &str) -> Option<(Vec<String>, Vec<f64>)> {
    let mut labels = Vec::new();
    let mut values = Vec::new();
    for line in text.lines() {
        let parts: Vec<&str> = line.splitn(2, ':').collect();
        if parts.len() == 2 {
            let label = parts[0].trim();
            let val_str = parts[1].trim().replace(['$', ',', 'K', 'M', '%'], "");
            if let Ok(val) = val_str.trim().parse::<f64>() {
                labels.push(label.to_string());
                values.push(val);
            }
        }
    }
    if labels.len() >= 3 { Some((labels, values)) } else { None }
}

/// Extract timeline events: lines starting with timestamps or numbered steps.
fn extract_timeline(text: &str) -> Option<Vec<(String, String)>> {
    let mut events = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        // Match "14:30 — description" or "1. description" or "Step 1: description"
        if let Some(pos) = trimmed.find('—').or_else(|| trimmed.find(" - ")) {
            let time = trimmed[..pos].trim().to_string();
            let desc = trimmed[pos + 1..].trim().trim_start_matches('-').trim().to_string();
            if !time.is_empty() && !desc.is_empty() { events.push((time, desc)); }
        }
    }
    if events.len() >= 3 { Some(events) } else { None }
}

/// Extract unified diff hunks from text.
pub fn extract_diff(text: &str) -> Option<Vec<DiffHunk>> {
    let lines: Vec<&str> = text.lines().collect();
    // Must have at least one @@ header
    if !lines.iter().any(|l| l.starts_with("@@")) { return None; }
    let mut hunks = Vec::new();
    let mut current: Option<DiffHunk> = None;
    for line in &lines {
        if line.starts_with("@@") {
            if let Some(h) = current.take() { if !h.lines.is_empty() { hunks.push(h); } }
            current = Some(DiffHunk { header: line.to_string(), lines: Vec::new() });
        } else if let Some(ref mut h) = current {
            let kind = if line.starts_with('+') { DiffLineKind::Add }
                else if line.starts_with('-') { DiffLineKind::Remove }
                else { DiffLineKind::Context };
            h.lines.push(DiffLine { kind, content: line.to_string() });
        }
    }
    if let Some(h) = current { if !h.lines.is_empty() { hunks.push(h); } }
    // Require at least 1 hunk with 2+ changed lines
    let has_changes = hunks.iter().any(|h| h.lines.iter().filter(|l| l.kind != DiffLineKind::Context).count() >= 2);
    if hunks.is_empty() || !has_changes { None } else { Some(hunks) }
}

/// Label for genome recording.
pub fn output_type_label(output: &RichOutput) -> &'static str { output.type_label() }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_plain_text() {
        let r = classify_output("Hello, this is a simple response.");
        assert!(matches!(r, RichOutput::Text(_)));
    }

    #[test]
    fn classify_markdown_table() {
        let text = "| Name | Age |\n|---|---|\n| Alice | 30 |\n| Bob | 25 |";
        let r = classify_output(text);
        assert!(matches!(r, RichOutput::Table { .. }));
    }

    #[test]
    fn classify_code_block() {
        let text = "Here's the code:\n```rust\nfn main() {\n    println!(\"hello\");\n}\n```";
        let r = classify_output(text);
        assert!(matches!(r, RichOutput::CodeBlock { .. }));
    }

    #[test]
    fn extract_table_from_markdown() {
        let text = "| Col A | Col B | Col C |\n|---|---|---|\n| 1 | 2 | 3 |\n| 4 | 5 | 6 |";
        let (headers, rows) = extract_table(text).unwrap();
        assert_eq!(headers.len(), 3);
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn extract_numeric_pairs_from_text() {
        let text = "January: $100K\nFebruary: $120K\nMarch: $145K\nApril: $160K";
        let (labels, values) = extract_numeric_pairs(text).unwrap();
        assert_eq!(labels.len(), 4);
        assert!((values[0] - 100.0).abs() < 0.1);
    }

    #[test]
    fn classify_diff() {
        let text = "@@ -1,3 +1,4 @@\n context line\n-removed line\n+added line\n+another add";
        let r = classify_output(text);
        assert!(matches!(r, RichOutput::Diff { .. }));
    }

    #[test]
    fn plain_plus_no_diff() {
        let text = "Adding +5 to the score gives +10 total";
        let r = classify_output(text);
        assert!(!matches!(r, RichOutput::Diff { .. }));
    }
}
