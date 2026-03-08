//! Tabbed settings page container (Step 3.3).
//!
//! Provides the tab model and navigation logic for the settings panel.
//! Each tab maps to a dedicated settings sub-component.

use serde::{Deserialize, Serialize};

/// Available tabs in the settings panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SettingsTab {
    General,
    Models,
    Sisters,
    Voice,
    Policies,
    Behavior,
    Notifications,
    Advanced,
}

impl SettingsTab {
    /// Human-readable label for the tab.
    pub fn label(&self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Models => "Models",
            Self::Sisters => "Sisters",
            Self::Voice => "Voice",
            Self::Policies => "Policies",
            Self::Behavior => "Behavior",
            Self::Notifications => "Notifications",
            Self::Advanced => "Advanced",
        }
    }

    /// Icon character for the tab.
    pub fn icon(&self) -> &'static str {
        match self {
            Self::General => "\u{2699}",       // gear
            Self::Models => "\u{1F4BB}",       // cpu (laptop as proxy)
            Self::Sisters => "\u{1F517}",      // link
            Self::Voice => "\u{1F3A4}",        // mic
            Self::Policies => "\u{1F6E1}",     // shield
            Self::Behavior => "\u{1F9E0}",     // brain
            Self::Notifications => "\u{1F514}", // bell
            Self::Advanced => "\u{1F527}",     // wrench
        }
    }

    /// All tabs in display order.
    pub fn all() -> Vec<SettingsTab> {
        vec![
            Self::General,
            Self::Models,
            Self::Sisters,
            Self::Voice,
            Self::Policies,
            Self::Behavior,
            Self::Notifications,
            Self::Advanced,
        ]
    }
}

/// Container state for the tabbed settings page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabbedSettings {
    pub active_tab: SettingsTab,
    pub search_query: String,
}

impl TabbedSettings {
    /// Create a new tabbed settings container starting on General.
    pub fn new() -> Self {
        Self {
            active_tab: SettingsTab::General,
            search_query: String::new(),
        }
    }

    /// Switch to a specific tab.
    pub fn switch_tab(&mut self, tab: SettingsTab) {
        self.active_tab = tab;
    }

    /// Advance to the next tab, wrapping around.
    pub fn next_tab(&mut self) {
        let tabs = SettingsTab::all();
        let idx = tabs.iter().position(|t| *t == self.active_tab).unwrap_or(0);
        self.active_tab = tabs[(idx + 1) % tabs.len()];
    }

    /// Move to the previous tab, wrapping around.
    pub fn prev_tab(&mut self) {
        let tabs = SettingsTab::all();
        let idx = tabs.iter().position(|t| *t == self.active_tab).unwrap_or(0);
        self.active_tab = if idx == 0 {
            tabs[tabs.len() - 1]
        } else {
            tabs[idx - 1]
        };
    }

    /// Total number of tabs.
    pub fn tab_count() -> usize {
        8
    }
}

impl Default for TabbedSettings {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_starts_on_general() {
        let ts = TabbedSettings::new();
        assert_eq!(ts.active_tab, SettingsTab::General);
        assert!(ts.search_query.is_empty());
    }

    #[test]
    fn test_tab_count() {
        assert_eq!(TabbedSettings::tab_count(), 8);
        assert_eq!(SettingsTab::all().len(), 8);
    }

    #[test]
    fn test_switch_tab() {
        let mut ts = TabbedSettings::new();
        ts.switch_tab(SettingsTab::Behavior);
        assert_eq!(ts.active_tab, SettingsTab::Behavior);
    }

    #[test]
    fn test_next_tab_cycles() {
        let mut ts = TabbedSettings::new();
        assert_eq!(ts.active_tab, SettingsTab::General);
        ts.next_tab();
        assert_eq!(ts.active_tab, SettingsTab::Models);

        // Cycle to end and wrap.
        ts.switch_tab(SettingsTab::Advanced);
        ts.next_tab();
        assert_eq!(ts.active_tab, SettingsTab::General);
    }

    #[test]
    fn test_prev_tab_cycles() {
        let mut ts = TabbedSettings::new();
        // At General, prev should wrap to Advanced.
        ts.prev_tab();
        assert_eq!(ts.active_tab, SettingsTab::Advanced);

        ts.prev_tab();
        assert_eq!(ts.active_tab, SettingsTab::Notifications);
    }

    #[test]
    fn test_labels_and_icons() {
        for tab in SettingsTab::all() {
            assert!(!tab.label().is_empty());
            assert!(!tab.icon().is_empty());
        }
        assert_eq!(SettingsTab::General.label(), "General");
        assert_eq!(SettingsTab::Advanced.label(), "Advanced");
    }

    #[test]
    fn test_serde_roundtrip() {
        let mut ts = TabbedSettings::new();
        ts.switch_tab(SettingsTab::Voice);
        ts.search_query = "timeout".to_string();

        let json = serde_json::to_string(&ts).unwrap();
        let restored: TabbedSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.active_tab, SettingsTab::Voice);
        assert_eq!(restored.search_query, "timeout");
    }
}
