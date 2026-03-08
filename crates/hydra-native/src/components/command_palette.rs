//! Command palette (Cmd+K) — fuzzy search overlay for quick actions.

use serde::{Deserialize, Serialize};

/// A command that can be executed from the palette.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaletteCommand {
    pub id: String,
    pub label: String,
    pub shortcut: Option<String>,
    pub category: CommandCategory,
}

/// Category for grouping commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandCategory {
    Session,
    Navigation,
    Settings,
    Mode,
    Action,
}

impl CommandCategory {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Session => "Sessions",
            Self::Navigation => "Navigation",
            Self::Settings => "Settings",
            Self::Mode => "Modes",
            Self::Action => "Actions",
        }
    }
}

/// The command palette view model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandPalette {
    pub query: String,
    pub commands: Vec<PaletteCommand>,
    pub selected_index: usize,
    pub recent_ids: Vec<String>,
}

impl CommandPalette {
    /// Create a new command palette with default commands.
    pub fn new() -> Self {
        Self {
            query: String::new(),
            commands: Self::default_commands(),
            selected_index: 0,
            recent_ids: Vec::new(),
        }
    }

    /// Default set of available commands.
    fn default_commands() -> Vec<PaletteCommand> {
        vec![
            PaletteCommand {
                id: "new-session".into(),
                label: "New Session".into(),
                shortcut: Some("Cmd+N".into()),
                category: CommandCategory::Session,
            },
            PaletteCommand {
                id: "toggle-sidebar".into(),
                label: "Toggle Sidebar".into(),
                shortcut: Some("Cmd+B".into()),
                category: CommandCategory::Navigation,
            },
            PaletteCommand {
                id: "open-settings".into(),
                label: "Open Settings".into(),
                shortcut: Some("Cmd+,".into()),
                category: CommandCategory::Settings,
            },
            PaletteCommand {
                id: "mode-companion".into(),
                label: "Switch to Companion Mode".into(),
                shortcut: Some("Cmd+1".into()),
                category: CommandCategory::Mode,
            },
            PaletteCommand {
                id: "mode-workspace".into(),
                label: "Switch to Workspace Mode".into(),
                shortcut: Some("Cmd+2".into()),
                category: CommandCategory::Mode,
            },
            PaletteCommand {
                id: "mode-immersive".into(),
                label: "Switch to Immersive Mode".into(),
                shortcut: Some("Cmd+3".into()),
                category: CommandCategory::Mode,
            },
            PaletteCommand {
                id: "mode-invisible".into(),
                label: "Switch to Invisible Mode".into(),
                shortcut: Some("Cmd+4".into()),
                category: CommandCategory::Mode,
            },
            PaletteCommand {
                id: "clear-chat".into(),
                label: "Clear Current Chat".into(),
                shortcut: None,
                category: CommandCategory::Action,
            },
            PaletteCommand {
                id: "toggle-kill-switch".into(),
                label: "Toggle Kill Switch".into(),
                shortcut: Some("Cmd+Shift+K".into()),
                category: CommandCategory::Action,
            },
            PaletteCommand {
                id: "search-messages".into(),
                label: "Search in Messages".into(),
                shortcut: Some("Cmd+F".into()),
                category: CommandCategory::Navigation,
            },
            PaletteCommand {
                id: "view-features".into(),
                label: "View Features & Capabilities".into(),
                shortcut: None,
                category: CommandCategory::Navigation,
            },
            PaletteCommand {
                id: "view-receipts".into(),
                label: "View Receipt Audit Log".into(),
                shortcut: None,
                category: CommandCategory::Navigation,
            },
        ]
    }

    /// Filter commands by fuzzy matching the query.
    pub fn filtered(&self) -> Vec<&PaletteCommand> {
        if self.query.is_empty() {
            // Show recent first, then all
            let mut result: Vec<&PaletteCommand> = Vec::new();
            for recent_id in &self.recent_ids {
                if let Some(cmd) = self.commands.iter().find(|c| c.id == *recent_id) {
                    result.push(cmd);
                }
            }
            for cmd in &self.commands {
                if !self.recent_ids.contains(&cmd.id) {
                    result.push(cmd);
                }
            }
            result
        } else {
            let query_lower = self.query.to_lowercase();
            self.commands
                .iter()
                .filter(|cmd| {
                    cmd.label.to_lowercase().contains(&query_lower)
                        || cmd.id.to_lowercase().contains(&query_lower)
                })
                .collect()
        }
    }

    /// Set the search query.
    pub fn set_query(&mut self, query: &str) {
        self.query = query.to_string();
        self.selected_index = 0;
    }

    /// Move selection up.
    pub fn select_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Move selection down.
    pub fn select_down(&mut self) {
        let max = self.filtered().len().saturating_sub(1);
        if self.selected_index < max {
            self.selected_index += 1;
        }
    }

    /// Get the currently selected command ID.
    pub fn selected_command_id(&self) -> Option<String> {
        self.filtered()
            .get(self.selected_index)
            .map(|c| c.id.clone())
    }

    /// Record a command as recently used.
    pub fn record_usage(&mut self, id: &str) {
        self.recent_ids.retain(|r| r != id);
        self.recent_ids.insert(0, id.to_string());
        if self.recent_ids.len() > 5 {
            self.recent_ids.truncate(5);
        }
    }

    /// Reset the palette state.
    pub fn reset(&mut self) {
        self.query.clear();
        self.selected_index = 0;
    }
}

impl Default for CommandPalette {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_palette_creation() {
        let palette = CommandPalette::new();
        assert!(palette.commands.len() >= 10);
        assert!(palette.query.is_empty());
    }

    #[test]
    fn test_palette_filter_empty_returns_all() {
        let palette = CommandPalette::new();
        let filtered = palette.filtered();
        assert_eq!(filtered.len(), palette.commands.len());
    }

    #[test]
    fn test_palette_filter_query() {
        let mut palette = CommandPalette::new();
        palette.set_query("settings");
        let filtered = palette.filtered();
        assert!(filtered.iter().any(|c| c.id == "open-settings"));
        assert!(filtered.len() < palette.commands.len());
    }

    #[test]
    fn test_palette_navigation() {
        let mut palette = CommandPalette::new();
        assert_eq!(palette.selected_index, 0);
        palette.select_down();
        assert_eq!(palette.selected_index, 1);
        palette.select_up();
        assert_eq!(palette.selected_index, 0);
        palette.select_up(); // Can't go below 0
        assert_eq!(palette.selected_index, 0);
    }

    #[test]
    fn test_palette_selected_command() {
        let palette = CommandPalette::new();
        let id = palette.selected_command_id();
        assert!(id.is_some());
    }

    #[test]
    fn test_palette_recent_usage() {
        let mut palette = CommandPalette::new();
        palette.record_usage("open-settings");
        palette.record_usage("new-session");
        assert_eq!(palette.recent_ids[0], "new-session");
        assert_eq!(palette.recent_ids[1], "open-settings");
        // Recent should appear first in filtered
        let filtered = palette.filtered();
        assert_eq!(filtered[0].id, "new-session");
        assert_eq!(filtered[1].id, "open-settings");
    }

    #[test]
    fn test_palette_reset() {
        let mut palette = CommandPalette::new();
        palette.set_query("test");
        palette.select_down();
        palette.reset();
        assert!(palette.query.is_empty());
        assert_eq!(palette.selected_index, 0);
    }
}
