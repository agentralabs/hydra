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
            EvidenceKind::Code => "\u{1F4C4}",         // page facing up
            EvidenceKind::Screenshot => "\u{1F4F7}",   // camera
            EvidenceKind::MemoryContext => "\u{1F9E0}", // brain
            EvidenceKind::Diff => "\u{00B1}",          // plus-minus
            EvidenceKind::LogOutput => "\u{1F4BB}",    // laptop / terminal
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_panel() {
        let ep = EvidencePanel::new();
        assert!(ep.items.is_empty());
        assert!(ep.active_item.is_none());
        assert!(ep.filter.is_none());
    }

    #[test]
    fn test_add_code() {
        let mut ep = EvidencePanel::new();
        let id = ep.add_code(
            "main.rs",
            "fn main() {}",
            Some("rust"),
            Some("src/main.rs"),
            Some((1, 3)),
        );
        assert_eq!(id, 0);
        assert_eq!(ep.item_count(), 1);
        let item = &ep.items[0];
        assert_eq!(item.kind, EvidenceKind::Code);
        assert_eq!(item.language.as_deref(), Some("rust"));
        assert_eq!(item.file_path.as_deref(), Some("src/main.rs"));
        assert_eq!(item.line_range, Some((1, 3)));
        assert_eq!(ep.active_item, Some(0));
    }

    #[test]
    fn test_add_screenshot() {
        let mut ep = EvidencePanel::new();
        let id = ep.add_screenshot("Error dialog", "/tmp/screenshot.png");
        assert_eq!(ep.items[0].kind, EvidenceKind::Screenshot);
        assert_eq!(ep.items[0].file_path.as_deref(), Some("/tmp/screenshot.png"));
        assert_eq!(ep.active_item, Some(id));
    }

    #[test]
    fn test_add_memory_context() {
        let mut ep = EvidencePanel::new();
        ep.add_memory_context("Session context", "User prefers dark mode");
        assert_eq!(ep.items[0].kind, EvidenceKind::MemoryContext);
        assert_eq!(ep.items[0].content, "User prefers dark mode");
    }

    #[test]
    fn test_add_diff() {
        let mut ep = EvidencePanel::new();
        ep.add_diff("Config change", "+new_key = true", Some("config.toml"));
        assert_eq!(ep.items[0].kind, EvidenceKind::Diff);
        assert_eq!(ep.items[0].language.as_deref(), Some("diff"));
    }

    #[test]
    fn test_add_log_output() {
        let mut ep = EvidencePanel::new();
        ep.add_log_output("Build output", "Compiling hydra v0.1.0");
        assert_eq!(ep.items[0].kind, EvidenceKind::LogOutput);
    }

    #[test]
    fn test_select_item() {
        let mut ep = EvidencePanel::new();
        let id0 = ep.add_code("A", "a", None, None, None);
        let _id1 = ep.add_code("B", "b", None, None, None);
        ep.select_item(id0);
        assert_eq!(ep.active_item, Some(id0));
        assert_eq!(ep.active().unwrap().title, "A");
    }

    #[test]
    fn test_select_nonexistent() {
        let mut ep = EvidencePanel::new();
        ep.add_code("A", "a", None, None, None);
        ep.select_item(999);
        // Should not change active_item
        assert_eq!(ep.active_item, Some(0));
    }

    #[test]
    fn test_toggle_pin() {
        let mut ep = EvidencePanel::new();
        let id = ep.add_code("A", "a", None, None, None);
        assert!(!ep.items[0].pinned);
        ep.toggle_pin(id);
        assert!(ep.items[0].pinned);
        ep.toggle_pin(id);
        assert!(!ep.items[0].pinned);
    }

    #[test]
    fn test_pinned_items() {
        let mut ep = EvidencePanel::new();
        let id0 = ep.add_code("A", "a", None, None, None);
        let _id1 = ep.add_code("B", "b", None, None, None);
        ep.toggle_pin(id0);
        let pinned = ep.pinned_items();
        assert_eq!(pinned.len(), 1);
        assert_eq!(pinned[0].title, "A");
    }

    #[test]
    fn test_filter() {
        let mut ep = EvidencePanel::new();
        ep.add_code("Code1", "x", None, None, None);
        ep.add_screenshot("Shot1", "/tmp/a.png");
        ep.add_code("Code2", "y", None, None, None);

        assert_eq!(ep.visible_items().len(), 3);

        ep.set_filter(Some(EvidenceKind::Code));
        let visible = ep.visible_items();
        assert_eq!(visible.len(), 2);
        assert!(visible.iter().all(|i| i.kind == EvidenceKind::Code));

        ep.set_filter(None);
        assert_eq!(ep.visible_items().len(), 3);
    }

    #[test]
    fn test_remove_item() {
        let mut ep = EvidencePanel::new();
        let id0 = ep.add_code("A", "a", None, None, None);
        let id1 = ep.add_code("B", "b", None, None, None);
        assert_eq!(ep.active_item, Some(id1));

        ep.remove_item(id1);
        assert_eq!(ep.item_count(), 1);
        assert_eq!(ep.active_item, Some(id0)); // falls back to last remaining

        ep.remove_item(id0);
        assert!(ep.items.is_empty());
        assert!(ep.active_item.is_none());
    }

    #[test]
    fn test_clear() {
        let mut ep = EvidencePanel::new();
        ep.add_code("A", "a", None, None, None);
        ep.add_code("B", "b", None, None, None);
        ep.clear();
        assert!(ep.items.is_empty());
        assert!(ep.active_item.is_none());
        // IDs reset
        let id = ep.add_code("C", "c", None, None, None);
        assert_eq!(id, 0);
    }

    #[test]
    fn test_code_count() {
        let mut ep = EvidencePanel::new();
        ep.add_code("A", "a", None, None, None);
        ep.add_screenshot("B", "/tmp/b.png");
        ep.add_code("C", "c", None, None, None);
        assert_eq!(ep.code_count(), 2);
    }

    #[test]
    fn test_evidence_css_classes() {
        assert_eq!(
            EvidencePanel::evidence_css_class(EvidenceKind::Code),
            "evidence-code"
        );
        assert_eq!(
            EvidencePanel::evidence_css_class(EvidenceKind::Screenshot),
            "evidence-screenshot"
        );
        assert_eq!(
            EvidencePanel::evidence_css_class(EvidenceKind::MemoryContext),
            "evidence-memory"
        );
    }

    #[test]
    fn test_evidence_icons() {
        let kinds = [
            EvidenceKind::Code,
            EvidenceKind::Screenshot,
            EvidenceKind::MemoryContext,
            EvidenceKind::Diff,
            EvidenceKind::LogOutput,
        ];
        for kind in kinds {
            let icon = EvidencePanel::evidence_icon(kind);
            assert!(!icon.is_empty());
            // Must NOT be text-based like [mem] or [img]
            assert!(!icon.starts_with('['), "Icon for {:?} must not be text-based bracket notation", kind);
        }
    }

    #[test]
    fn test_human_summary_json_empty() {
        let mut ep = EvidencePanel::new();
        ep.add_memory_context("Memory", r#"{"count": 0, "nodes": []}"#);
        let summary = EvidencePanel::human_summary(&ep.items[0]);
        // Empty JSON should produce empty summary (not meaningful)
        assert!(summary.is_empty() || !summary.contains('{'));
    }

    #[test]
    fn test_human_summary_meaningful_memory() {
        let mut ep = EvidencePanel::new();
        ep.add_memory_context("Memory", "User prefers dark mode\nUser likes Rust");
        let summary = EvidencePanel::human_summary(&ep.items[0]);
        assert!(!summary.is_empty());
        assert!(!summary.contains('{'));
    }

    #[test]
    fn test_is_meaningful_empty() {
        let mut ep = EvidencePanel::new();
        ep.add_memory_context("Memory", "");
        assert!(!EvidencePanel::is_meaningful(&ep.items[0]));
    }

    #[test]
    fn test_is_meaningful_json_empty() {
        let mut ep = EvidencePanel::new();
        ep.add_memory_context("Memory", r#"{"count": 0, "nodes": []}"#);
        assert!(!EvidencePanel::is_meaningful(&ep.items[0]));
    }

    #[test]
    fn test_default() {
        let ep = EvidencePanel::default();
        assert!(ep.items.is_empty());
    }

    #[test]
    fn test_remove_nonexistent_is_noop() {
        let mut ep = EvidencePanel::new();
        let id = ep.add_code("A", "a", None, None, None);
        ep.remove_item(999);
        assert_eq!(ep.item_count(), 1);
        assert_eq!(ep.active_item, Some(id));
    }

    #[test]
    fn test_toggle_pin_nonexistent_is_noop() {
        let mut ep = EvidencePanel::new();
        ep.add_code("A", "a", None, None, None);
        ep.toggle_pin(999);
        assert!(!ep.items[0].pinned);
    }

    #[test]
    fn test_multiple_pinned_items() {
        let mut ep = EvidencePanel::new();
        let id0 = ep.add_code("A", "a", None, None, None);
        let id1 = ep.add_code("B", "b", None, None, None);
        let id2 = ep.add_code("C", "c", None, None, None);
        ep.toggle_pin(id0);
        ep.toggle_pin(id2);
        let pinned = ep.pinned_items();
        assert_eq!(pinned.len(), 2);
        assert!(pinned.iter().any(|i| i.id == id0));
        assert!(pinned.iter().any(|i| i.id == id2));
        assert!(!pinned.iter().any(|i| i.id == id1));
    }

    #[test]
    fn test_filter_screenshot_only() {
        let mut ep = EvidencePanel::new();
        ep.add_code("Code", "x", None, None, None);
        ep.add_screenshot("Shot", "/tmp/a.png");
        ep.add_diff("Diff", "+x", None);
        ep.set_filter(Some(EvidenceKind::Screenshot));
        let visible = ep.visible_items();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].kind, EvidenceKind::Screenshot);
    }

    #[test]
    fn test_all_css_classes_unique() {
        let kinds = [
            EvidenceKind::Code,
            EvidenceKind::Screenshot,
            EvidenceKind::MemoryContext,
            EvidenceKind::Diff,
            EvidenceKind::LogOutput,
        ];
        let classes: Vec<&str> = kinds.iter().map(|k| EvidencePanel::evidence_css_class(*k)).collect();
        for (i, a) in classes.iter().enumerate() {
            for (j, b) in classes.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn test_active_after_remove_middle() {
        let mut ep = EvidencePanel::new();
        let _id0 = ep.add_code("A", "a", None, None, None);
        let id1 = ep.add_code("B", "b", None, None, None);
        let _id2 = ep.add_code("C", "c", None, None, None);
        // Active is id2 (last added)
        ep.select_item(id1);
        ep.remove_item(id1);
        // Falls back to last remaining item
        assert_eq!(ep.item_count(), 2);
        assert!(ep.active_item.is_some());
    }

    #[test]
    fn test_sequential_ids_after_add_remove_add() {
        let mut ep = EvidencePanel::new();
        let id0 = ep.add_code("A", "a", None, None, None);
        assert_eq!(id0, 0);
        ep.remove_item(id0);
        // IDs keep incrementing (not reset) unless clear() is called
        let id1 = ep.add_code("B", "b", None, None, None);
        assert_eq!(id1, 1);
    }

    #[test]
    fn test_evidence_kind_serialization() {
        let kinds = [
            EvidenceKind::Code,
            EvidenceKind::Screenshot,
            EvidenceKind::MemoryContext,
            EvidenceKind::Diff,
            EvidenceKind::LogOutput,
        ];
        for k in &kinds {
            let json = serde_json::to_string(k).unwrap();
            let back: EvidenceKind = serde_json::from_str(&json).unwrap();
            assert_eq!(*k, back);
        }
    }

    #[test]
    fn test_code_with_line_range() {
        let mut ep = EvidencePanel::new();
        ep.add_code("snippet", "let x = 1;", Some("rust"), Some("src/lib.rs"), Some((10, 20)));
        let item = &ep.items[0];
        assert_eq!(item.line_range, Some((10, 20)));
        assert_eq!(item.file_path.as_deref(), Some("src/lib.rs"));
        assert_eq!(item.language.as_deref(), Some("rust"));
    }

    #[test]
    fn test_diff_has_diff_language() {
        let mut ep = EvidencePanel::new();
        ep.add_diff("change", "+line", Some("file.rs"));
        assert_eq!(ep.items[0].language.as_deref(), Some("diff"));
    }
}
