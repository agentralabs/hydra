//! ClipboardMonitor — watches system clipboard and classifies content.

use crate::constants::CLIPBOARD_POLL_MS;
use crate::errors::DesktopError;
use serde::{Deserialize, Serialize};

/// Classification of clipboard content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClipboardContentType {
    Url,
    Code,
    ErrorMessage,
    Json,
    PlainText,
    Empty,
}

/// A clipboard change event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardEvent {
    pub content: String,
    pub content_type: ClipboardContentType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Monitors the system clipboard for changes.
pub struct ClipboardMonitor {
    last_content: String,
    history: Vec<ClipboardEvent>,
}

impl ClipboardMonitor {
    pub fn new() -> Self {
        Self {
            last_content: String::new(),
            history: Vec::new(),
        }
    }

    /// Check clipboard for changes. Returns Some(event) if content changed.
    pub fn poll(&mut self) -> Option<ClipboardEvent> {
        let content = Self::read_clipboard().unwrap_or_default();
        if content == self.last_content || content.is_empty() {
            return None;
        }

        self.last_content = content.clone();
        let content_type = Self::classify(&content);
        let event = ClipboardEvent {
            content,
            content_type,
            timestamp: chrono::Utc::now(),
        };
        self.history.push(event.clone());
        Some(event)
    }

    /// Classify clipboard content type.
    pub fn classify(content: &str) -> ClipboardContentType {
        let trimmed = content.trim();

        if trimmed.is_empty() {
            return ClipboardContentType::Empty;
        }
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            return ClipboardContentType::Url;
        }
        if trimmed.starts_with('{')
            && trimmed.ends_with('}')
            && serde_json::from_str::<serde_json::Value>(trimmed).is_ok()
        {
            return ClipboardContentType::Json;
        }
        if Self::looks_like_error(trimmed) {
            return ClipboardContentType::ErrorMessage;
        }
        if Self::looks_like_code(trimmed) {
            return ClipboardContentType::Code;
        }

        ClipboardContentType::PlainText
    }

    /// Get clipboard history.
    pub fn history(&self) -> &[ClipboardEvent] {
        &self.history
    }

    /// Recommended poll interval.
    pub fn poll_interval_ms() -> u64 {
        CLIPBOARD_POLL_MS
    }

    fn looks_like_error(text: &str) -> bool {
        let lower = text.to_lowercase();
        lower.contains("error:") || lower.contains("exception:")
            || lower.contains("traceback") || lower.contains("panic:")
            || lower.contains("fatal:") || lower.contains("failed:")
            || (lower.contains("error") && lower.contains("at line"))
    }

    fn looks_like_code(text: &str) -> bool {
        let lines: Vec<&str> = text.lines().collect();
        if lines.len() < 2 { return false; }
        // Structural heuristics (language-agnostic, not keyword lists):
        // 1. Indentation consistency (code has structured indent)
        let indented = lines.iter().filter(|l| l.starts_with("  ") || l.starts_with('\t')).count();
        let indent_ratio = indented as f64 / lines.len() as f64;
        // 2. Bracket/brace density (code has more {}, (), [])
        let brackets: usize = text.chars().filter(|c| "{}()[]".contains(*c)).count();
        let bracket_ratio = brackets as f64 / text.len().max(1) as f64;
        // 3. Semicolons or colons at line ends (many languages)
        let endings = lines.iter().filter(|l| {
            let t = l.trim();
            t.ends_with(';') || t.ends_with('{') || t.ends_with(':') || t.ends_with(',')
        }).count();
        let ending_ratio = endings as f64 / lines.len() as f64;
        // Score: weighted combination
        let score = indent_ratio * 0.4 + bracket_ratio * 30.0 + ending_ratio * 0.3;
        score > 0.3
    }

    fn read_clipboard() -> Result<String, DesktopError> {
        if cfg!(target_os = "macos") {
            let output = std::process::Command::new("pbpaste")
                .output()
                .map_err(|e| DesktopError::ClipboardError(e.to_string()))?;
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else if cfg!(target_os = "linux") {
            let output = std::process::Command::new("xclip")
                .args(["-selection", "clipboard", "-o"])
                .output()
                .or_else(|_| {
                    std::process::Command::new("xsel")
                        .args(["--clipboard", "--output"])
                        .output()
                })
                .map_err(|e| DesktopError::ClipboardError(e.to_string()))?;
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(DesktopError::UnsupportedPlatform("clipboard".into()))
        }
    }
}

impl Default for ClipboardMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_url() {
        assert_eq!(
            ClipboardMonitor::classify("https://example.com"),
            ClipboardContentType::Url
        );
    }

    #[test]
    fn classify_json() {
        assert_eq!(
            ClipboardMonitor::classify(r#"{"key": "value"}"#),
            ClipboardContentType::Json
        );
    }

    #[test]
    fn classify_error() {
        assert_eq!(
            ClipboardMonitor::classify("Error: cannot read property 'map' of undefined"),
            ClipboardContentType::ErrorMessage
        );
    }

    #[test]
    fn classify_code() {
        let code = "fn main() {\n    println!(\"hello\");\n}";
        assert_eq!(
            ClipboardMonitor::classify(code),
            ClipboardContentType::Code
        );
    }

    #[test]
    fn classify_plain_text() {
        assert_eq!(
            ClipboardMonitor::classify("Just some normal text"),
            ClipboardContentType::PlainText
        );
    }

    #[test]
    fn classify_empty() {
        assert_eq!(
            ClipboardMonitor::classify(""),
            ClipboardContentType::Empty
        );
    }
}
