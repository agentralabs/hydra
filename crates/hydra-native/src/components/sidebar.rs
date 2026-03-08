//! Workspace sidebar component data.

use serde::{Deserialize, Serialize};

/// A collapsible section within the sidebar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidebarSection {
    pub title: String,
    pub items: Vec<SidebarItem>,
    pub collapsed: bool,
}

/// A single item in the sidebar (task or history entry).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidebarItem {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub active: bool,
    pub timestamp: Option<String>,
}

/// The sidebar container with sections and visibility state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sidebar {
    pub sections: Vec<SidebarSection>,
    pub width: u32,
    pub visible: bool,
}

impl Sidebar {
    /// Create a sidebar with default "Today" and "History" sections.
    pub fn new() -> Self {
        Self {
            sections: vec![
                SidebarSection {
                    title: "Today".into(),
                    items: Vec::new(),
                    collapsed: false,
                },
                SidebarSection {
                    title: "History".into(),
                    items: Vec::new(),
                    collapsed: true,
                },
            ],
            width: 260,
            visible: true,
        }
    }

    /// Add a new task to the "Today" section.
    pub fn add_task(&mut self, id: &str, label: &str) {
        if let Some(today) = self.sections.first_mut() {
            // Deactivate any previously active item
            for item in &mut today.items {
                item.active = false;
            }
            today.items.push(SidebarItem {
                id: id.to_owned(),
                label: label.to_owned(),
                icon: "\u{25C9}".into(), // active circle
                active: true,
                timestamp: None,
            });
        }
    }

    /// Mark a task as completed (moves icon to checkmark, deactivates).
    pub fn complete_task(&mut self, id: &str) {
        for section in &mut self.sections {
            for item in &mut section.items {
                if item.id == id {
                    item.icon = "\u{2713}".into(); // checkmark
                    item.active = false;
                    return;
                }
            }
        }
    }

    /// Toggle a section's collapsed state.
    pub fn toggle_section(&mut self, index: usize) {
        if let Some(section) = self.sections.get_mut(index) {
            section.collapsed = !section.collapsed;
        }
    }

    /// Remove a task/session from all sections.
    pub fn remove_task(&mut self, id: &str) {
        for section in &mut self.sections {
            section.items.retain(|item| item.id != id);
        }
    }

    /// Move a task from Today to History (archive).
    pub fn archive_task(&mut self, id: &str) {
        let mut found = None;
        if let Some(today) = self.sections.first_mut() {
            if let Some(pos) = today.items.iter().position(|item| item.id == id) {
                let mut item = today.items.remove(pos);
                item.active = false;
                item.icon = "\u{2713}".into();
                found = Some(item);
            }
        }
        if let Some(item) = found {
            if let Some(history) = self.sections.get_mut(1) {
                history.items.insert(0, item);
            }
        }
    }

    /// Toggle sidebar visibility.
    pub fn toggle_visibility(&mut self) {
        self.visible = !self.visible;
    }

    /// Search all items by label (case-insensitive substring match).
    pub fn search(&self, query: &str) -> Vec<&SidebarItem> {
        let q = query.to_lowercase();
        self.sections
            .iter()
            .flat_map(|s| &s.items)
            .filter(|item| item.label.to_lowercase().contains(&q))
            .collect()
    }

    /// Items in the "Today" section.
    pub fn today_items(&self) -> &[SidebarItem] {
        self.sections
            .first()
            .map(|s| s.items.as_slice())
            .unwrap_or(&[])
    }

    /// Items in the "History" section.
    pub fn history_items(&self) -> &[SidebarItem] {
        self.sections
            .get(1)
            .map(|s| s.items.as_slice())
            .unwrap_or(&[])
    }
}

impl Default for Sidebar {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_sidebar_has_sections() {
        let sb = Sidebar::new();
        assert_eq!(sb.sections.len(), 2);
        assert_eq!(sb.sections[0].title, "Today");
        assert_eq!(sb.sections[1].title, "History");
        assert!(sb.visible);
    }

    #[test]
    fn test_add_task() {
        let mut sb = Sidebar::new();
        sb.add_task("t1", "Write email");
        assert_eq!(sb.today_items().len(), 1);
        assert!(sb.today_items()[0].active);
        assert_eq!(sb.today_items()[0].label, "Write email");
    }

    #[test]
    fn test_complete_task() {
        let mut sb = Sidebar::new();
        sb.add_task("t1", "Write email");
        sb.complete_task("t1");
        assert!(!sb.today_items()[0].active);
        assert_eq!(sb.today_items()[0].icon, "\u{2713}");
    }

    #[test]
    fn test_toggle_section() {
        let mut sb = Sidebar::new();
        assert!(!sb.sections[0].collapsed);
        sb.toggle_section(0);
        assert!(sb.sections[0].collapsed);
        sb.toggle_section(0);
        assert!(!sb.sections[0].collapsed);
    }

    #[test]
    fn test_toggle_visibility() {
        let mut sb = Sidebar::new();
        assert!(sb.visible);
        sb.toggle_visibility();
        assert!(!sb.visible);
    }

    #[test]
    fn test_search() {
        let mut sb = Sidebar::new();
        sb.add_task("t1", "Write email");
        sb.add_task("t2", "Read docs");
        sb.add_task("t3", "Write tests");
        let results = sb.search("write");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_add_task_deactivates_previous() {
        let mut sb = Sidebar::new();
        sb.add_task("t1", "First");
        sb.add_task("t2", "Second");
        assert!(!sb.today_items()[0].active);
        assert!(sb.today_items()[1].active);
    }

    #[test]
    fn test_complete_nonexistent_task_is_noop() {
        let mut sb = Sidebar::new();
        sb.add_task("t1", "Task");
        sb.complete_task("nonexistent");
        assert!(sb.today_items()[0].active); // unchanged
    }

    #[test]
    fn test_toggle_section_out_of_bounds_is_noop() {
        let mut sb = Sidebar::new();
        let collapsed_before = sb.sections[0].collapsed;
        sb.toggle_section(99);
        assert_eq!(sb.sections[0].collapsed, collapsed_before);
    }

    #[test]
    fn test_history_section_starts_collapsed() {
        let sb = Sidebar::new();
        assert!(!sb.sections[0].collapsed); // Today
        assert!(sb.sections[1].collapsed);   // History
    }

    #[test]
    fn test_history_items_initially_empty() {
        let sb = Sidebar::new();
        assert!(sb.history_items().is_empty());
    }

    #[test]
    fn test_search_case_insensitive() {
        let mut sb = Sidebar::new();
        sb.add_task("t1", "Write EMAIL");
        let results = sb.search("email");
        assert_eq!(results.len(), 1);
        let results2 = sb.search("EMAIL");
        assert_eq!(results2.len(), 1);
    }

    #[test]
    fn test_search_no_results() {
        let mut sb = Sidebar::new();
        sb.add_task("t1", "Write email");
        let results = sb.search("deploy");
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_empty_query_returns_all() {
        let mut sb = Sidebar::new();
        sb.add_task("t1", "Write email");
        sb.add_task("t2", "Read docs");
        let results = sb.search("");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_default_width() {
        let sb = Sidebar::new();
        assert_eq!(sb.width, 260);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut sb = Sidebar::new();
        sb.add_task("t1", "Write email");
        sb.complete_task("t1");
        let json = serde_json::to_string(&sb).unwrap();
        let back: Sidebar = serde_json::from_str(&json).unwrap();
        assert_eq!(back.sections.len(), 2);
        assert_eq!(back.today_items().len(), 1);
        assert!(!back.today_items()[0].active);
    }

    #[test]
    fn test_multiple_tasks_only_last_active() {
        let mut sb = Sidebar::new();
        sb.add_task("t1", "First");
        sb.add_task("t2", "Second");
        sb.add_task("t3", "Third");
        let items = sb.today_items();
        assert!(!items[0].active);
        assert!(!items[1].active);
        assert!(items[2].active);
    }
}
