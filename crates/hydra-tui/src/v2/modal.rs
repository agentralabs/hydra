//! Modal system — overlays for command palette, config editor, keybindings.
//! Modals capture all input while open. Esc closes any modal.

use crate::v2::config_schema::all_schemas;

/// Active modal state.
#[derive(Debug, Clone)]
pub enum Modal {
    /// Fuzzy command palette (Ctrl+K).
    CommandPalette {
        query: String,
        filtered: Vec<usize>,
        selected: usize,
    },
    /// Inline config editor.
    ConfigEditor {
        entries: Vec<ConfigEntry>,
        selected: usize,
        editing: bool,
        draft: String,
        error: Option<String>,
    },
    /// Keybinding editor.
    KeybindingEditor {
        bindings: Vec<KeybindingEntry>,
        selected: usize,
        recording: bool,
    },
    /// Session list (/sessions or /resume).
    SessionList {
        sessions: Vec<SessionEntry>,
        selected: usize,
    },
    /// Confirmation dialog.
    Confirm {
        message: String,
        selected_yes: bool,
    },
}

/// One entry in the config editor.
#[derive(Debug, Clone)]
pub struct ConfigEntry {
    pub key: String,
    pub section: String,
    pub description: String,
    pub current_value: String,
    pub default_value: String,
    pub value_type: String,
    pub options: Vec<String>,
}

/// One entry in the keybinding editor.
#[derive(Debug, Clone)]
pub struct KeybindingEntry {
    pub action_name: String,
    pub key_display: String,
    pub context: String,
    pub is_default: bool,
}

/// One entry in the session list.
#[derive(Debug, Clone)]
pub struct SessionEntry {
    pub id: String,
    pub date: String,
    pub exchange_count: usize,
    pub preview: String,
}

impl Modal {
    /// Create a new command palette.
    pub fn palette() -> Self {
        Self::CommandPalette {
            query: String::new(),
            filtered: Vec::new(),
            selected: 0,
        }
    }

    /// Create a new config editor from current config.
    pub fn config_editor(current_values: &std::collections::HashMap<String, String>) -> Self {
        let entries: Vec<ConfigEntry> = all_schemas()
            .into_iter()
            .map(|schema| {
                let key = format!("{}.{}", schema.section, schema.key);
                let current = current_values
                    .get(&key)
                    .cloned()
                    .unwrap_or_else(|| schema.default.to_string());
                let options = match &schema.value_type {
                    crate::v2::config_schema::ValueType::Enum(opts) => {
                        opts.iter().map(|s| s.to_string()).collect()
                    }
                    crate::v2::config_schema::ValueType::Bool => {
                        vec!["true".into(), "false".into()]
                    }
                    _ => vec![],
                };
                ConfigEntry {
                    key: schema.key.to_string(),
                    section: schema.section.to_string(),
                    description: schema.description.to_string(),
                    current_value: current,
                    default_value: schema.default.to_string(),
                    value_type: format!("{:?}", schema.value_type),
                    options,
                }
            })
            .collect();

        Self::ConfigEditor {
            entries,
            selected: 0,
            editing: false,
            draft: String::new(),
            error: None,
        }
    }

    /// Navigate up in any modal.
    pub fn navigate_up(&mut self) {
        match self {
            Self::CommandPalette { selected, .. } => {
                if *selected > 0 {
                    *selected -= 1;
                }
            }
            Self::ConfigEditor { selected, .. } => {
                if *selected > 0 {
                    *selected -= 1;
                }
            }
            Self::KeybindingEditor { selected, .. } => {
                if *selected > 0 {
                    *selected -= 1;
                }
            }
            Self::SessionList { selected, .. } => {
                if *selected > 0 {
                    *selected -= 1;
                }
            }
            Self::Confirm { selected_yes, .. } => {
                *selected_yes = !*selected_yes;
            }
        }
    }

    /// Navigate down in any modal.
    pub fn navigate_down(&mut self) {
        match self {
            Self::CommandPalette { selected, filtered, .. } => {
                if *selected + 1 < filtered.len() {
                    *selected += 1;
                }
            }
            Self::ConfigEditor { selected, entries, .. } => {
                if *selected + 1 < entries.len() {
                    *selected += 1;
                }
            }
            Self::KeybindingEditor { selected, bindings, .. } => {
                if *selected + 1 < bindings.len() {
                    *selected += 1;
                }
            }
            Self::SessionList { selected, sessions, .. } => {
                if *selected + 1 < sessions.len() {
                    *selected += 1;
                }
            }
            Self::Confirm { selected_yes, .. } => {
                *selected_yes = !*selected_yes;
            }
        }
    }

    /// Type a character (palette search, config edit).
    pub fn type_char(&mut self, ch: char) {
        match self {
            Self::CommandPalette { query, .. } => query.push(ch),
            Self::ConfigEditor { editing, draft, .. } if *editing => draft.push(ch),
            _ => {}
        }
    }

    /// Backspace in modal.
    pub fn backspace(&mut self) {
        match self {
            Self::CommandPalette { query, .. } => { query.pop(); }
            Self::ConfigEditor { editing, draft, .. } if *editing => { draft.pop(); }
            _ => {}
        }
    }

    /// Get the modal title for rendering.
    pub fn title(&self) -> &str {
        match self {
            Self::CommandPalette { .. } => "Command Palette",
            Self::ConfigEditor { .. } => "Settings",
            Self::KeybindingEditor { .. } => "Keybindings",
            Self::SessionList { .. } => "Sessions",
            Self::Confirm { .. } => "Confirm",
        }
    }

    /// Whether this modal is a palette (needs fuzzy filtering).
    pub fn is_palette(&self) -> bool {
        matches!(self, Self::CommandPalette { .. })
    }
}

/// Fuzzy match: returns score (0 = no match, higher = better).
pub fn fuzzy_score(query: &str, target: &str) -> u32 {
    if query.is_empty() {
        return 1; // empty query matches everything
    }
    let query_lower = query.to_lowercase();
    let target_lower = target.to_lowercase();

    // Exact prefix match = highest score
    if target_lower.starts_with(&query_lower) {
        return 1000 + 100u32.saturating_sub(target.len() as u32);
    }

    // Substring match = high score
    if target_lower.contains(&query_lower) {
        return 500 + 100u32.saturating_sub(target.len() as u32);
    }

    // Character-skip fuzzy match
    let mut query_chars = query_lower.chars().peekable();
    let mut score = 0u32;
    let mut consecutive = 0u32;

    for ch in target_lower.chars() {
        if let Some(&q) = query_chars.peek() {
            if ch == q {
                score += 10 + consecutive * 5;
                consecutive += 1;
                query_chars.next();
            } else {
                consecutive = 0;
            }
        }
    }

    // All query chars must be consumed for a match
    if query_chars.peek().is_some() {
        0 // no match
    } else {
        score
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuzzy_exact_prefix() {
        assert!(fuzzy_score("hel", "help") > fuzzy_score("hel", "health"));
    }

    #[test]
    fn fuzzy_substring() {
        assert!(fuzzy_score("genom", "genome") > 0);
    }

    #[test]
    fn fuzzy_skip() {
        assert!(fuzzy_score("hlth", "health") > 0);
    }

    #[test]
    fn fuzzy_no_match() {
        assert_eq!(fuzzy_score("xyz", "health"), 0);
    }

    #[test]
    fn fuzzy_empty_query() {
        assert!(fuzzy_score("", "anything") > 0);
    }

    #[test]
    fn modal_navigate() {
        let mut modal = Modal::palette();
        if let Modal::CommandPalette { ref mut filtered, .. } = modal {
            *filtered = vec![0, 1, 2, 3, 4];
        }
        modal.navigate_down();
        if let Modal::CommandPalette { selected, .. } = &modal {
            assert_eq!(*selected, 1);
        }
        modal.navigate_up();
        if let Modal::CommandPalette { selected, .. } = &modal {
            assert_eq!(*selected, 0);
        }
    }

    #[test]
    fn modal_type_and_backspace() {
        let mut modal = Modal::palette();
        modal.type_char('h');
        modal.type_char('e');
        if let Modal::CommandPalette { query, .. } = &modal {
            assert_eq!(query, "he");
        }
        modal.backspace();
        if let Modal::CommandPalette { query, .. } = &modal {
            assert_eq!(query, "h");
        }
    }
}
