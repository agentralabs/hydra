//! Action detection and command execution helpers.
//! Detects user intent and returns appropriate shell commands.
//!
//! The main detection function lives in `actions_detect.rs` (split for compilation performance).
//! This module contains extraction helpers, formatting, and utility functions.

use super::platform::*;

// Re-export detect_direct_action_command from the split module
pub(crate) use super::actions_detect::detect_direct_action_command;

/// Extract inline commands from <hydra-exec> tags.
pub(crate) fn extract_inline_commands(text: &str) -> Vec<String> {
    let mut commands = Vec::new();
    let mut remaining = text;
    while let Some(start) = remaining.find("<hydra-exec>") {
        let after = &remaining[start + 12..];
        if let Some(end) = after.find("</hydra-exec>") {
            let cmd = after[..end].trim().to_string();
            if !cmd.is_empty() {
                commands.push(cmd);
            }
            remaining = &after[end + 13..];
        } else {
            break;
        }
    }
    commands
}

/// Extract a URL from a command string (for Vision capture).
pub(crate) fn extract_url_from_command(cmd: &str) -> Option<String> {
    for word in cmd.split_whitespace() {
        if word.starts_with("http://") || word.starts_with("https://") {
            // Strip quotes
            let url = word.trim_matches(|c| c == '\'' || c == '"');
            return Some(url.to_string());
        }
    }
    None
}

/// Extract the dependency target from a query like "what depends on loop_runner.rs"
pub(crate) fn extract_dependency_target(text: &str, lower: &str) -> Option<String> {
    // Try extracting after common prepositions/patterns
    let markers = ["depends on ", "imports ", "calls ", "uses ", "references to ", "impact of "];
    for marker in &markers {
        if let Some(pos) = lower.find(marker) {
            let after = text[pos + marker.len()..].trim();
            // Take the first word/token (could be a filename or identifier)
            let token = after.split_whitespace().next().unwrap_or("")
                .trim_matches(|c: char| c == '?' || c == ',' || c == '"' || c == '\'' || c == '`');
            if !token.is_empty() {
                return Some(token.to_string());
            }
        }
    }
    // Try "where is X used" pattern
    if lower.contains("where is") && lower.contains("used") {
        if let Some(pos) = lower.find("where is ") {
            let after = &text[pos + 9..];
            let token = after.split_whitespace().next().unwrap_or("")
                .trim_matches(|c: char| c == '?' || c == ',' || c == '"' || c == '\'' || c == '`');
            if !token.is_empty() && token != "used" && token != "it" {
                return Some(token.to_string());
            }
        }
    }
    // Fallback: look for anything that looks like a filename
    for word in text.split_whitespace() {
        let clean = word.trim_matches(|c: char| c == '?' || c == ',' || c == '"' || c == '\'' || c == '`');
        let exts = [".rs", ".ts", ".js", ".py", ".toml", ".json", ".md", ".css", ".html"];
        if exts.iter().any(|e| clean.ends_with(e)) {
            return Some(clean.to_string());
        }
    }
    None
}

/// Extract a file/directory path from user text.
/// Looks for ~ paths, / paths, and common file extensions.
pub(crate) fn extract_path_from_text(text: &str) -> Option<String> {
    for word in text.split_whitespace() {
        let clean = word.trim_matches(|c: char| c == '?' || c == ',' || c == '"' || c == '\'');
        if clean.starts_with('~') || clean.starts_with('/') {
            return Some(clean.to_string());
        }
        // Match words ending in common file extensions
        if clean.contains('.') && !clean.starts_with("http") {
            let exts = [".md", ".rs", ".json", ".toml", ".yaml", ".yml", ".txt", ".sh", ".py", ".js", ".ts"];
            if exts.iter().any(|e| clean.ends_with(e)) {
                return Some(clean.to_string());
            }
        }
    }
    None
}

/// Shell-escape a path for safe command interpolation.
pub(crate) fn shell_escape(s: &str) -> String {
    if s.contains(' ') || s.contains('(') || s.contains(')') || s.contains('&') {
        format!("'{}'", s.replace('\'', "'\\''"))
    } else {
        s.to_string()
    }
}

/// Extract browser name from text
pub(crate) fn extract_browser_name(lower: &str) -> String {
    if lower.contains("firefox") { "firefox".into() }
    else if lower.contains("safari") { "safari".into() }
    else { "chrome".into() }
}

/// Extract URL from "open google.com" / "go to https://example.com"
pub(crate) fn extract_url_intent(lower: &str, original: &str) -> Option<String> {
    // Match "open X.com", "go to X.com", "visit X.com"
    for prefix in &["open ", "go to ", "visit ", "navigate to ", "browse "] {
        if lower.starts_with(prefix) {
            let rest = original[prefix.len()..].trim();
            let rest_lower = rest.to_lowercase();
            // Strip articles
            let target = strip_articles(&rest_lower);
            if target.starts_with("http://") || target.starts_with("https://")
                || target.contains(".com") || target.contains(".org") || target.contains(".io")
                || target.contains(".dev") || target.contains(".net") || target.contains(".co")
                || target.contains(".app") || target.contains(".me")
            {
                return if target.starts_with("http") {
                    Some(target)
                } else {
                    Some(format!("https://{}", target))
                };
            }
        }
    }
    None
}

/// Extract the app name from a verb+app intent like "close chrome" or "quit spotify"
pub(crate) fn extract_app_name_from_intent(lower: &str, verbs: &[&str]) -> Option<String> {
    for verb in verbs {
        if let Some(pos) = lower.find(verb) {
            let after = lower[pos + verb.len()..].trim();
            let app = strip_articles(after);
            if !app.is_empty() && app.len() > 1 {
                return Some(app);
            }
        }
    }
    None
}

/// Format command output for chat display.
/// Detects grep-style output and formats it as a clean summary.
/// For non-grep output, truncates to MAX_DISPLAY_LINES.
pub(crate) fn format_command_output(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "Done.".to_string();
    }

    let all_lines: Vec<&str> = trimmed.lines().collect();
    let total = all_lines.len();

    // Detect grep-style output: lines matching "path:N:content" or "path:N: content"
    let grep_count = all_lines.iter()
        .filter(|l| {
            let l = l.trim_start_matches("./");
            let parts: Vec<&str> = l.splitn(3, ':').collect();
            parts.len() >= 2 && parts[0].contains('.') && parts[1].trim().chars().all(|c| c.is_ascii_digit())
        })
        .count();

    // If most lines look like grep output, format as search results
    if grep_count > 3 && grep_count as f64 / total as f64 > 0.5 {
        return format_grep_results(trimmed, &all_lines);
    }

    // Non-grep output: show up to MAX lines
    const MAX_DISPLAY_LINES: usize = 20;
    if total <= MAX_DISPLAY_LINES {
        format!("```\n{}\n```", trimmed)
    } else {
        let shown: Vec<&str> = all_lines[..MAX_DISPLAY_LINES].to_vec();
        let omitted = total - MAX_DISPLAY_LINES;
        format!(
            "```\n{}\n```\n({} more lines, {} total)",
            shown.join("\n"),
            omitted,
            total
        )
    }
}

/// Format grep-style output as a clean, readable summary.
/// Groups results by file, shows top matches, provides counts.
pub(crate) fn format_grep_results(_raw: &str, all_lines: &[&str]) -> String {
    use std::collections::BTreeMap;

    // Parse grep lines into file → Vec<(line_num, content)>
    let mut by_file: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();

    for line in all_lines {
        let clean = line.trim_start_matches("./");
        let parts: Vec<&str> = clean.splitn(3, ':').collect();
        if parts.len() >= 2 && parts[0].contains('.') {
            let is_line_num = parts[1].trim().chars().all(|c| c.is_ascii_digit());
            if is_line_num && parts.len() == 3 {
                let file = parts[0].to_string();
                let line_num = parts[1].to_string();
                let content = parts[2].trim().to_string();
                by_file.entry(file).or_default().push((line_num, content));
            }
        }
    }

    let file_count = by_file.len();
    let match_count: usize = by_file.values().map(|v| v.len()).sum();

    let mut result = format!("Found {} references across {} files.\n\n", match_count, file_count);

    // Show up to 8 files with their top match
    const MAX_FILES: usize = 8;
    const MAX_MATCHES_PER_FILE: usize = 2;
    let mut files_shown = 0;

    for (file, matches) in &by_file {
        if files_shown >= MAX_FILES {
            break;
        }
        // Clean up path for display
        let display_file = file.trim_start_matches("./");
        for (i, (line_num, content)) in matches.iter().enumerate() {
            if i >= MAX_MATCHES_PER_FILE {
                if matches.len() > MAX_MATCHES_PER_FILE {
                    result.push_str(&format!("  ... +{} more in {}\n",
                        matches.len() - MAX_MATCHES_PER_FILE, display_file));
                }
                break;
            }
            // Truncate long content lines
            let display_content = if content.len() > 80 {
                format!("{}...", &content[..77])
            } else {
                content.clone()
            };
            result.push_str(&format!("  {}:{} — {}\n", display_file, line_num, display_content));
        }
        files_shown += 1;
    }

    if file_count > MAX_FILES {
        result.push_str(&format!("\n({} more files — use /search for full results)\n",
            file_count - MAX_FILES));
    }

    result
}

/// Strip <hydra-exec>...</hydra-exec> tags from the response text for clean display.
pub(crate) fn strip_hydra_exec_tags(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut search_from = 0;

    loop {
        let open_tag = "<hydra-exec>";
        let close_tag = "</hydra-exec>";

        match text[search_from..].find(open_tag) {
            Some(pos) => {
                result.push_str(&text[search_from..search_from + pos]);
                let after_open = search_from + pos + open_tag.len();
                match text[after_open..].find(close_tag) {
                    Some(end_pos) => {
                        search_from = after_open + end_pos + close_tag.len();
                    }
                    None => {
                        result.push_str(&text[search_from + pos..]);
                        break;
                    }
                }
            }
            None => {
                result.push_str(&text[search_from..]);
                break;
            }
        }
    }

    result.trim().to_string()
}
