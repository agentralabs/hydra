//! Rich formatting for structured output — progress steps, test results, completion summaries.
//! Separated from render.rs for file size. Called from render_rich_content_inner.

use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};
use crate::tui::theme;

/// Try to format a line as a structured element. Returns true if handled.
pub fn try_format_structured(line: &str, lines: &mut Vec<Line<'static>>) -> bool {
    // Progress steps: "Scaffolding...", "Verifying...", "Running tests..."
    if (line.ends_with("...") || line.ends_with("...\""))
        && !line.starts_with(' ') && !line.starts_with("●")
        && line.len() < 120
    {
        lines.push(Line::from(vec![
            Span::styled("  ● ", Style::default().fg(theme::HYDRA_CYAN)),
            Span::styled(
                line.trim_end_matches('"').to_string(),
                Style::default().fg(theme::HYDRA_CYAN),
            ),
        ]));
        return true;
    }

    // Section headers: "Project Creation Complete", "Build Results", etc.
    if !line.starts_with(' ') && !line.starts_with('[') && !line.starts_with("●")
        && line.len() < 60
        && line.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
        && !line.contains(':')
        && !line.contains("test result")
    {
        let words: Vec<&str> = line.split_whitespace().collect();
        if words.len() >= 2 && words.len() <= 6 && words.iter().all(|w| w.len() < 20) {
            lines.push(Line::default());
            lines.push(Line::from(Span::styled(
                format!("  {}", line),
                Style::default().fg(theme::HYDRA_BLUE).add_modifier(Modifier::BOLD),
            )));
            return true;
        }
    }

    // Test result lines: "test result: ok. N passed; M failed..."
    if line.starts_with("test result:") {
        let is_pass = line.contains("0 failed");
        let color = if is_pass { theme::HYDRA_GREEN } else { theme::HYDRA_RED };
        let icon = if is_pass { "  ✓ " } else { "  ✗ " };
        // Extract just the key numbers
        let summary = summarize_test_line(line);
        lines.push(Line::from(vec![
            Span::styled(icon, Style::default().fg(color).add_modifier(Modifier::BOLD)),
            Span::styled(summary, Style::default().fg(color)),
        ]));
        return true;
    }

    // Pass/fail markers: "[pass]", "[fail]", "[ok]", "[error]"
    if line.starts_with("[pass]") || line.starts_with("[ok]") {
        lines.push(Line::from(vec![
            Span::styled("  ✓ ", Style::default().fg(theme::HYDRA_GREEN).add_modifier(Modifier::BOLD)),
            Span::styled(
                line.trim_start_matches("[pass]").trim_start_matches("[ok]").trim().to_string(),
                Style::default().fg(theme::HYDRA_GREEN),
            ),
        ]));
        return true;
    }
    if line.starts_with("[fail]") || line.starts_with("[error]") {
        lines.push(Line::from(vec![
            Span::styled("  ✗ ", Style::default().fg(theme::HYDRA_RED).add_modifier(Modifier::BOLD)),
            Span::styled(
                line.trim_start_matches("[fail]").trim_start_matches("[error]").trim().to_string(),
                Style::default().fg(theme::HYDRA_RED),
            ),
        ]));
        return true;
    }

    // Path output: "Project at: `/path/to/project`"
    if line.starts_with("Project at:") || line.starts_with("Output at:") || line.starts_with("Created at:") {
        let label = line.split(':').next().unwrap_or("");
        let path = line[label.len()+1..].trim().trim_matches('`').trim_matches('\'');
        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", label), Style::default().fg(theme::HYDRA_DIM)),
            Span::styled(path.to_string(), Style::default().fg(theme::HYDRA_BLUE).add_modifier(Modifier::BOLD)),
        ]));
        return true;
    }

    // "Read spec:" lines
    if line.starts_with("Read spec:") {
        lines.push(Line::from(vec![
            Span::styled("  ● ", Style::default().fg(theme::HYDRA_GREEN)),
            Span::styled(line.to_string(), Style::default().fg(theme::HYDRA_DIM)),
        ]));
        return true;
    }

    // "New project creation requested" and similar status
    if line.contains("requested") || line.contains("approved") || line.contains("completed") {
        if line.len() < 80 && !line.starts_with(' ') {
            let color = if line.contains("fail") { theme::HYDRA_RED } else { theme::HYDRA_GREEN };
            lines.push(Line::from(Span::styled(
                format!("  {}", line), Style::default().fg(color),
            )));
            return true;
        }
    }

    false
}

/// Summarize a "test result:" line into a compact form.
fn summarize_test_line(line: &str) -> String {
    // "test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s"
    // → "3 passed, 0 failed (0.00s)"
    let mut passed = "";
    let mut failed = "";
    let mut time = "";
    for part in line.split(';') {
        let p = part.trim();
        if p.contains("passed") { passed = p.split_whitespace().next().unwrap_or("0"); }
        if p.contains("failed") { failed = p.split_whitespace().next().unwrap_or("0"); }
        if p.contains("finished in") {
            time = p.trim_start_matches("finished in").trim();
        }
    }
    if passed.is_empty() { passed = "0"; }
    if failed.is_empty() { failed = "0"; }
    if !time.is_empty() {
        format!("{} passed, {} failed ({})", passed, failed, time)
    } else {
        format!("{} passed, {} failed", passed, failed)
    }
}

/// Detect tool result blocks that should be collapsed to a one-liner.
pub fn should_collapse_tool_result(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.contains("Found") && (trimmed.contains("reference") || trimmed.contains("match")) {
        return Some(format!("Searched codebase — {}", trimmed));
    }
    if (trimmed.contains("files analyzed") || trimmed.contains("Scanned"))
        && trimmed.chars().any(|c| c.is_ascii_digit()) {
        return Some(format!("Analysis complete — {}", trimmed));
    }
    if (trimmed.contains("Compiling") || trimmed.contains("Finished"))
        && (trimmed.contains("target") || trimmed.contains("release") || trimmed.contains("debug")) {
        return Some("Build successful".to_string());
    }
    if trimmed.contains("Generated") && trimmed.contains("blueprint") {
        return Some("Blueprint generated".to_string());
    }
    None
}
