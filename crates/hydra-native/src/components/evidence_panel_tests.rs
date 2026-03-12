//! Tests for the EvidencePanel.

#[cfg(test)]
mod tests {
    use crate::components::evidence_panel::{EvidenceKind, EvidencePanel};

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
