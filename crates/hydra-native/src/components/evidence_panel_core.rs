//! Evidence panel component data — code preview, screenshots, memory context.

use serde::{Deserialize, Serialize};

/// The type of evidence being displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceKind {
    /// A code block with optional language and file path.
    Code,
    /// A screenshot or image reference.
    Screenshot,
    /// A memory context entry retrieved from agentic-memory.
    MemoryContext,
    /// A diff showing changes made.
    Diff,
    /// A log or terminal output snippet.
    LogOutput,
}

/// A single piece of evidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceItem {
    pub id: usize,
    pub kind: EvidenceKind,
    pub title: String,
    pub content: String,
    pub language: Option<String>,
    pub file_path: Option<String>,
    pub line_range: Option<(usize, usize)>,
    pub timestamp: Option<String>,
    pub pinned: bool,
}

/// The evidence panel view model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidencePanel {
    pub items: Vec<EvidenceItem>,
    next_id: usize,
    pub active_item: Option<usize>,
    pub filter: Option<EvidenceKind>,
}

impl EvidencePanel {
    /// Create an empty evidence panel.
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            next_id: 0,
            active_item: None,
            filter: None,
        }
    }

    /// Add a code evidence item.
    pub fn add_code(
        &mut self,
        title: &str,
        content: &str,
        language: Option<&str>,
        file_path: Option<&str>,
        line_range: Option<(usize, usize)>,
    ) -> usize {
        self.add_item(
            EvidenceKind::Code,
            title,
            content,
            language,
            file_path,
            line_range,
        )
    }

    /// Add a screenshot placeholder.
    pub fn add_screenshot(&mut self, title: &str, path: &str) -> usize {
        self.add_item(
            EvidenceKind::Screenshot,
            title,
            path, // content holds the image path
            None,
            Some(path),
            None,
        )
    }

    /// Add a memory context entry.
    pub fn add_memory_context(&mut self, title: &str, content: &str) -> usize {
        self.add_item(EvidenceKind::MemoryContext, title, content, None, None, None)
    }

    /// Add a diff evidence item.
    pub fn add_diff(&mut self, title: &str, diff_content: &str, file_path: Option<&str>) -> usize {
        self.add_item(
            EvidenceKind::Diff,
            title,
            diff_content,
            Some("diff"),
            file_path,
            None,
        )
    }

    /// Add a log output evidence item.
    pub fn add_log_output(&mut self, title: &str, output: &str) -> usize {
        self.add_item(EvidenceKind::LogOutput, title, output, None, None, None)
    }

    /// Generic item addition.
    fn add_item(
        &mut self,
        kind: EvidenceKind,
        title: &str,
        content: &str,
        language: Option<&str>,
        file_path: Option<&str>,
        line_range: Option<(usize, usize)>,
    ) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.items.push(EvidenceItem {
            id,
            kind,
            title: title.to_owned(),
            content: content.to_owned(),
            language: language.map(|s| s.to_owned()),
            file_path: file_path.map(|s| s.to_owned()),
            line_range,
            timestamp: None,
            pinned: false,
        });
        // Auto-select the newly added item
        self.active_item = Some(id);
        id
    }

    /// Select an item by id.
    pub fn select_item(&mut self, id: usize) {
        if self.items.iter().any(|i| i.id == id) {
            self.active_item = Some(id);
        }
    }

    /// Get the currently active item.
    pub fn active(&self) -> Option<&EvidenceItem> {
        self.active_item
            .and_then(|id| self.items.iter().find(|i| i.id == id))
    }

    /// Toggle the pinned state of an item.
    pub fn toggle_pin(&mut self, id: usize) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.pinned = !item.pinned;
        }
    }

    /// Return items matching the current filter, or all items if no filter is set.
    pub fn visible_items(&self) -> Vec<&EvidenceItem> {
        match self.filter {
            Some(kind) => self.items.iter().filter(|i| i.kind == kind).collect(),
            None => self.items.iter().collect(),
        }
    }

    /// Return only pinned items.
    pub fn pinned_items(&self) -> Vec<&EvidenceItem> {
        self.items.iter().filter(|i| i.pinned).collect()
    }

    /// Set the evidence kind filter. Pass `None` to show all.
    pub fn set_filter(&mut self, kind: Option<EvidenceKind>) {
        self.filter = kind;
    }

    /// Remove an item by id.
    pub fn remove_item(&mut self, id: usize) {
        self.items.retain(|i| i.id != id);
        if self.active_item == Some(id) {
            self.active_item = self.items.last().map(|i| i.id);
        }
    }

    /// Total number of evidence items.
    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    /// Number of code items.
    pub fn code_count(&self) -> usize {
        self.items
            .iter()
            .filter(|i| i.kind == EvidenceKind::Code)
            .count()
    }

    /// Clear all items.
    pub fn clear(&mut self) {
        self.items.clear();
        self.next_id = 0;
        self.active_item = None;
    }

    /// CSS class for an evidence kind.
    pub fn evidence_css_class(kind: EvidenceKind) -> &'static str {
        match kind {
            EvidenceKind::Code => "evidence-code",
            EvidenceKind::Screenshot => "evidence-screenshot",
            EvidenceKind::MemoryContext => "evidence-memory",
            EvidenceKind::Diff => "evidence-diff",
            EvidenceKind::LogOutput => "evidence-log",
        }
    }

    /// Icon for an evidence kind.
    pub fn evidence_icon(kind: EvidenceKind) -> &'static str {
        match kind {
            EvidenceKind::Code => "\u{2630}",        // trigram (file)
            EvidenceKind::Screenshot => "\u{25A3}",   // white square with rounded corners
            EvidenceKind::MemoryContext => "\u{2261}", // identical to (context)
            EvidenceKind::Diff => "\u{00B1}",         // plus-minus
            EvidenceKind::LogOutput => "\u{25B8}",    // right-pointing small triangle (terminal)
        }
    }

    /// Human-readable summary of evidence content (never JSON).
    pub fn human_summary(item: &EvidenceItem) -> String {
        let content = &item.content;

        // If content looks like JSON, extract a summary instead
        if content.trim_start().starts_with('{') || content.trim_start().starts_with('[') {
            // Try to parse and summarize
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(content) {
                return Self::summarize_json(&val);
            }
        }

        // For memory context, show a brief summary
        if item.kind == EvidenceKind::MemoryContext {
            let lines: Vec<&str> = content.lines().collect();
            if lines.is_empty() || content.trim().is_empty() {
                return String::new(); // Empty = not meaningful
            }
            if lines.len() <= 3 {
                return content.to_string();
            }
            return format!("{} relevant memories", lines.len());
        }

        // For code, show file path or truncated preview
        if item.kind == EvidenceKind::Code {
            if let Some(ref path) = item.file_path {
                return format!("File: {}", path);
            }
        }

        // Default: return content as-is (but truncated)
        if content.len() > 200 {
            format!("{}...", &content[..200])
        } else {
            content.to_string()
        }
    }

    /// Summarize a JSON value into human-readable text
    fn summarize_json(val: &serde_json::Value) -> String {
        match val {
            serde_json::Value::Object(map) => {
                if let Some(count) = map.get("count") {
                    if count.as_u64() == Some(0) {
                        return String::new(); // Empty result = not meaningful
                    }
                    if let Some(c) = count.as_u64() {
                        return format!("{} items found", c);
                    }
                }
                if let Some(nodes) = map.get("nodes") {
                    if let Some(arr) = nodes.as_array() {
                        if arr.is_empty() {
                            return String::new();
                        }
                        return format!("{} results", arr.len());
                    }
                }
                format!("{} fields", map.len())
            }
            serde_json::Value::Array(arr) => {
                if arr.is_empty() {
                    String::new()
                } else {
                    format!("{} items", arr.len())
                }
            }
            _ => val.to_string(),
        }
    }

    /// Whether an evidence item has meaningful content to show.
    pub fn is_meaningful(item: &EvidenceItem) -> bool {
        let summary = Self::human_summary(item);
        !summary.is_empty()
    }
}

impl Default for EvidencePanel {
    fn default() -> Self {
        Self::new()
    }
}
