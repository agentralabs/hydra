//! Global keyboard shortcuts system.
//!
//! Defines shortcuts and maps key combos to command IDs that the UI dispatches.

use serde::{Deserialize, Serialize};

/// A keyboard shortcut binding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shortcut {
    pub key: String,
    pub modifiers: ShortcutModifiers,
    pub command_id: String,
    pub description: String,
}

/// Modifier keys for a shortcut.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShortcutModifiers {
    pub cmd: bool,
    pub shift: bool,
    pub alt: bool,
    pub ctrl: bool,
}

impl ShortcutModifiers {
    pub fn cmd() -> Self {
        Self { cmd: true, ..Default::default() }
    }

    pub fn cmd_shift() -> Self {
        Self { cmd: true, shift: true, ..Default::default() }
    }

    pub fn none() -> Self {
        Self::default()
    }

    /// Display string for the modifier combination.
    pub fn display(&self) -> String {
        let mut parts = Vec::new();
        if self.cmd { parts.push("Cmd"); }
        if self.ctrl { parts.push("Ctrl"); }
        if self.alt { parts.push("Alt"); }
        if self.shift { parts.push("Shift"); }
        parts.join("+")
    }
}

/// The shortcuts registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutRegistry {
    pub shortcuts: Vec<Shortcut>,
}

impl ShortcutRegistry {
    /// Create the default shortcuts.
    pub fn new() -> Self {
        Self {
            shortcuts: vec![
                Shortcut {
                    key: "k".into(),
                    modifiers: ShortcutModifiers::cmd(),
                    command_id: "command-palette".into(),
                    description: "Open command palette".into(),
                },
                Shortcut {
                    key: "n".into(),
                    modifiers: ShortcutModifiers::cmd(),
                    command_id: "new-session".into(),
                    description: "New session".into(),
                },
                Shortcut {
                    key: "b".into(),
                    modifiers: ShortcutModifiers::cmd(),
                    command_id: "toggle-sidebar".into(),
                    description: "Toggle sidebar".into(),
                },
                Shortcut {
                    key: ",".into(),
                    modifiers: ShortcutModifiers::cmd(),
                    command_id: "open-settings".into(),
                    description: "Open settings".into(),
                },
                Shortcut {
                    key: "1".into(),
                    modifiers: ShortcutModifiers::cmd(),
                    command_id: "mode-companion".into(),
                    description: "Companion mode".into(),
                },
                Shortcut {
                    key: "2".into(),
                    modifiers: ShortcutModifiers::cmd(),
                    command_id: "mode-workspace".into(),
                    description: "Workspace mode".into(),
                },
                Shortcut {
                    key: "3".into(),
                    modifiers: ShortcutModifiers::cmd(),
                    command_id: "mode-immersive".into(),
                    description: "Immersive mode".into(),
                },
                Shortcut {
                    key: "4".into(),
                    modifiers: ShortcutModifiers::cmd(),
                    command_id: "mode-invisible".into(),
                    description: "Invisible mode".into(),
                },
                Shortcut {
                    key: "K".into(),
                    modifiers: ShortcutModifiers::cmd_shift(),
                    command_id: "toggle-kill-switch".into(),
                    description: "Toggle kill switch".into(),
                },
                Shortcut {
                    key: "f".into(),
                    modifiers: ShortcutModifiers::cmd(),
                    command_id: "search-messages".into(),
                    description: "Search in messages".into(),
                },
                Shortcut {
                    key: "z".into(),
                    modifiers: ShortcutModifiers::cmd(),
                    command_id: "undo".into(),
                    description: "Undo last action".into(),
                },
                Shortcut {
                    key: "Z".into(),
                    modifiers: ShortcutModifiers::cmd_shift(),
                    command_id: "redo".into(),
                    description: "Redo last action".into(),
                },
                Shortcut {
                    key: "Escape".into(),
                    modifiers: ShortcutModifiers::none(),
                    command_id: "close-overlay".into(),
                    description: "Close overlay".into(),
                },
            ],
        }
    }

    /// Match a key event to a command ID.
    pub fn match_key(&self, key: &str, cmd: bool, shift: bool, alt: bool, ctrl: bool) -> Option<&str> {
        self.shortcuts.iter().find(|s| {
            s.key == key
                && s.modifiers.cmd == cmd
                && s.modifiers.shift == shift
                && s.modifiers.alt == alt
                && s.modifiers.ctrl == ctrl
        }).map(|s| s.command_id.as_str())
    }

    /// Get display label for a shortcut (e.g. "Cmd+K").
    pub fn display_label(shortcut: &Shortcut) -> String {
        let mods = shortcut.modifiers.display();
        if mods.is_empty() {
            shortcut.key.clone()
        } else {
            format!("{}+{}", mods, shortcut.key)
        }
    }
}

impl Default for ShortcutRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shortcut_registry_creation() {
        let registry = ShortcutRegistry::new();
        assert!(registry.shortcuts.len() >= 12);
    }

    #[test]
    fn test_match_cmd_k() {
        let registry = ShortcutRegistry::new();
        let result = registry.match_key("k", true, false, false, false);
        assert_eq!(result, Some("command-palette"));
    }

    #[test]
    fn test_match_cmd_shift_k() {
        let registry = ShortcutRegistry::new();
        let result = registry.match_key("K", true, true, false, false);
        assert_eq!(result, Some("toggle-kill-switch"));
    }

    #[test]
    fn test_match_escape() {
        let registry = ShortcutRegistry::new();
        let result = registry.match_key("Escape", false, false, false, false);
        assert_eq!(result, Some("close-overlay"));
    }

    #[test]
    fn test_no_match() {
        let registry = ShortcutRegistry::new();
        let result = registry.match_key("x", true, false, false, false);
        assert_eq!(result, None);
    }

    #[test]
    fn test_display_label() {
        let shortcut = Shortcut {
            key: "K".into(),
            modifiers: ShortcutModifiers::cmd_shift(),
            command_id: "test".into(),
            description: "Test".into(),
        };
        assert_eq!(ShortcutRegistry::display_label(&shortcut), "Cmd+Shift+K");
    }

    #[test]
    fn test_display_label_no_modifiers() {
        let shortcut = Shortcut {
            key: "Escape".into(),
            modifiers: ShortcutModifiers::none(),
            command_id: "test".into(),
            description: "Test".into(),
        };
        assert_eq!(ShortcutRegistry::display_label(&shortcut), "Escape");
    }
}
