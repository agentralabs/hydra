//! O22 Rich Output Renderers — ASCII charts, tables, timelines, progress bars.
//! All rendering uses Unicode box-drawing characters. Pure terminal. No external tools.

// ── Table Renderer ──

/// Render a table with box-drawing borders. Auto-sizes columns.
pub fn render_table(headers: &[String], rows: &[Vec<String>], _width: u16) -> Vec<String> {
    if headers.is_empty() { return vec![]; }
    // Calculate column widths
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < widths.len() { widths[i] = widths[i].max(cell.len()); }
        }
    }
    let mut lines = Vec::new();
    // Top border
    let top: String = widths.iter().map(|w| "─".repeat(w + 2)).collect::<Vec<_>>().join("┬");
    lines.push(format!("┌{top}┐"));
    // Headers
    let header_line: String = headers.iter().enumerate()
        .map(|(i, h)| format!(" {:<width$} ", h, width = widths.get(i).copied().unwrap_or(0)))
        .collect::<Vec<_>>().join("│");
    lines.push(format!("│{header_line}│"));
    // Separator
    let sep: String = widths.iter().map(|w| "─".repeat(w + 2)).collect::<Vec<_>>().join("┼");
    lines.push(format!("├{sep}┤"));
    // Rows
    for row in rows {
        let row_line: String = row.iter().enumerate()
            .map(|(i, cell)| format!(" {:<width$} ", cell, width = widths.get(i).copied().unwrap_or(0)))
            .collect::<Vec<_>>().join("│");
        lines.push(format!("│{row_line}│"));
    }
    // Bottom border
    let bot: String = widths.iter().map(|w| "─".repeat(w + 2)).collect::<Vec<_>>().join("┴");
    lines.push(format!("└{bot}┘"));
    lines
}

// ── Chart Renderer ──

/// Render a vertical bar chart with ASCII bars.
pub fn render_chart(title: &str, labels: &[String], values: &[f64], unit: &str, width: u16) -> Vec<String> {
    if values.is_empty() { return vec![]; }
    let max_val = values.iter().cloned().fold(0.0_f64, f64::max);
    if max_val == 0.0 { return vec![format!("  {} (no data)", title)]; }
    let bar_area = (width as usize).saturating_sub(12);
    let bar_width = (bar_area / values.len().max(1)).max(3);
    let mut lines = Vec::new();
    if !title.is_empty() { lines.push(format!("  {title}{}", if unit.is_empty() { String::new() } else { format!(" ({unit})") })); }
    // Y-axis with bars
    let steps = 5usize;
    for step in (0..=steps).rev() {
        let threshold = max_val * step as f64 / steps as f64;
        let label = format!("{:>6.0} │", threshold);
        let bars: String = values.iter()
            .map(|v| if *v >= threshold { "█".repeat(bar_width.saturating_sub(1)) + " " }
                 else { " ".repeat(bar_width) })
            .collect();
        lines.push(format!("{label}{bars}"));
    }
    // X-axis
    lines.push(format!("       └{}", "─".repeat(bar_width * values.len())));
    // Labels
    let label_line: String = labels.iter()
        .map(|l| format!("{:<width$}", &l[..l.len().min(bar_width - 1)], width = bar_width))
        .collect();
    lines.push(format!("        {label_line}"));
    lines
}

// ── Timeline Renderer ──

/// Render a timeline of events with connector lines.
pub fn render_timeline(events: &[(String, String)]) -> Vec<String> {
    let mut lines = Vec::new();
    for (i, (time, desc)) in events.iter().enumerate() {
        let marker = if i == 0 { "◆" } else { "◇" };
        lines.push(format!("  {marker} {time} — {desc}"));
        if i < events.len() - 1 { lines.push("  │".into()); }
    }
    lines
}

// ── Progress Renderer ──

/// Render progress bars for tasks.
pub fn render_progress(tasks: &[(String, f64)], width: u16) -> Vec<String> {
    let bar_len = (width as usize).saturating_sub(30).max(10);
    tasks.iter().map(|(name, pct)| {
        let filled = (pct * bar_len as f64) as usize;
        let empty = bar_len.saturating_sub(filled);
        let bar = format!("[{}{}]", "█".repeat(filled), "░".repeat(empty));
        let label = if name.len() > 20 { &name[..20] } else { name };
        format!("  {:<20} {} {:>3.0}%", label, bar, pct * 100.0)
    }).collect()
}

// ── Diff Renderer ──

/// Render unified diff hunks with semantic coloring hints.
/// Returns lines that the stream renderer will color-code based on prefix (+/-/@@).
pub fn render_diff(hunks: &[hydra_kernel::rich_output::DiffHunk]) -> Vec<String> {
    let mut lines = Vec::new();
    for hunk in hunks {
        lines.push(hunk.header.clone());
        for dl in &hunk.lines {
            lines.push(dl.content.clone());
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_table_format() {
        let headers = vec!["Name".into(), "Age".into()];
        let rows = vec![vec!["Alice".into(), "30".into()], vec!["Bob".into(), "25".into()]];
        let lines = render_table(&headers, &rows, 80);
        assert!(lines.len() >= 5);
        assert!(lines[0].contains("┌"));
        assert!(lines[1].contains("Name"));
        assert!(lines.last().unwrap().contains("┘"));
    }

    #[test]
    fn render_chart_format() {
        let labels = vec!["Jan".into(), "Feb".into(), "Mar".into()];
        let values = vec![100.0, 150.0, 200.0];
        let lines = render_chart("Revenue", &labels, &values, "$K", 60);
        assert!(!lines.is_empty());
        assert!(lines[0].contains("Revenue"));
    }

    #[test]
    fn render_timeline_format() {
        let events = vec![
            ("14:00".into(), "Deploy started".into()),
            ("14:05".into(), "Health check passed".into()),
            ("14:10".into(), "Deploy complete".into()),
        ];
        let lines = render_timeline(&events);
        assert!(lines.iter().any(|l| l.contains("◆")));
        assert!(lines.iter().any(|l| l.contains("Deploy")));
    }

    #[test]
    fn render_progress_format() {
        let tasks = vec![("Frontend".into(), 0.8), ("Backend".into(), 0.5)];
        let lines = render_progress(&tasks, 60);
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("█"));
        assert!(lines[0].contains("80%"));
    }
}
