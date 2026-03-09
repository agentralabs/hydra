/// Slash command registry — power-user fast path to all Hydra handlers.
///
/// Same handlers as the cognitive loop's intent classifier, just without
/// the 150-token classification overhead.

/// A registered slash command.
#[derive(Clone, Debug)]
pub struct SlashCommand {
    pub name: &'static str,
    pub description: &'static str,
    pub category: CommandCategory,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CommandCategory {
    System,
    Conversation,
    Settings,
    Control,
    Debug,
}

/// All available slash commands.
pub const COMMANDS: &[SlashCommand] = &[
    // ── System ──
    SlashCommand { name: "/sisters",  description: "Show sister diagnostic table", category: CommandCategory::System },
    SlashCommand { name: "/fix",      description: "Repair offline sisters",       category: CommandCategory::System },
    SlashCommand { name: "/scan",     description: "Run Omniscience scan",         category: CommandCategory::System },
    SlashCommand { name: "/repair",   description: "Run self-repair specs",        category: CommandCategory::System },
    SlashCommand { name: "/memory",   description: "Show memory stats",            category: CommandCategory::System },
    SlashCommand { name: "/goals",    description: "Show active planning goals",   category: CommandCategory::System },
    SlashCommand { name: "/beliefs",  description: "Show current belief store",    category: CommandCategory::System },
    SlashCommand { name: "/receipts", description: "Show recent action receipts",  category: CommandCategory::System },
    SlashCommand { name: "/health",   description: "Full system health dashboard", category: CommandCategory::System },
    SlashCommand { name: "/status",   description: "System status summary",        category: CommandCategory::System },

    // ── Conversation ──
    SlashCommand { name: "/clear",    description: "Clear conversation history",   category: CommandCategory::Conversation },
    SlashCommand { name: "/compact",  description: "Compact conversation",         category: CommandCategory::Conversation },
    SlashCommand { name: "/history",  description: "Show conversation history",    category: CommandCategory::Conversation },

    // ── Settings ──
    SlashCommand { name: "/model",    description: "Switch LLM model",             category: CommandCategory::Settings },
    SlashCommand { name: "/voice",    description: "Toggle voice input (STT)",     category: CommandCategory::Settings },
    SlashCommand { name: "/sidebar",  description: "Toggle sidebar",               category: CommandCategory::Settings },
    SlashCommand { name: "/theme",    description: "Switch color theme",           category: CommandCategory::Settings },
    SlashCommand { name: "/config",   description: "Open settings panel",          category: CommandCategory::Settings },

    // ── Control ──
    SlashCommand { name: "/trust",    description: "Show/set trust level",         category: CommandCategory::Control },
    SlashCommand { name: "/approve",  description: "Approve pending action",       category: CommandCategory::Control },
    SlashCommand { name: "/deny",     description: "Deny pending action",          category: CommandCategory::Control },
    SlashCommand { name: "/kill",     description: "Kill current execution",       category: CommandCategory::Control },

    // ── Debug ──
    SlashCommand { name: "/log",      description: "Show recent logs",             category: CommandCategory::Debug },
    SlashCommand { name: "/debug",    description: "Toggle debug mode",            category: CommandCategory::Debug },
    SlashCommand { name: "/tokens",   description: "Show token usage stats",       category: CommandCategory::Debug },

    // ── Meta ──
    SlashCommand { name: "/help",     description: "Show all commands",            category: CommandCategory::Debug },
    SlashCommand { name: "/quit",     description: "Exit Hydra",                   category: CommandCategory::Debug },
];

/// Dropdown state for slash command autocomplete.
#[derive(Clone, Debug)]
pub struct CommandDropdown {
    /// Whether the dropdown is currently visible.
    pub visible: bool,
    /// Filtered commands matching the current input.
    pub filtered: Vec<&'static SlashCommand>,
    /// Currently selected index in the filtered list.
    pub selected: usize,
}

impl Default for CommandDropdown {
    fn default() -> Self {
        Self {
            visible: false,
            filtered: Vec::new(),
            selected: 0,
        }
    }
}

impl CommandDropdown {
    /// Update the filtered list based on current input.
    /// Called on every keystroke when input starts with "/".
    pub fn update_filter(&mut self, input: &str) {
        if !input.starts_with('/') || input.contains(' ') {
            self.visible = false;
            self.filtered.clear();
            self.selected = 0;
            return;
        }

        self.filtered = COMMANDS
            .iter()
            .filter(|cmd| cmd.name.starts_with(input))
            .collect();

        self.visible = !self.filtered.is_empty();
        // Clamp selection
        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len().saturating_sub(1);
        }
    }

    /// Move selection up.
    pub fn select_prev(&mut self) {
        if !self.filtered.is_empty() {
            self.selected = self.selected.saturating_sub(1);
        }
    }

    /// Move selection down.
    pub fn select_next(&mut self) {
        if !self.filtered.is_empty() && self.selected + 1 < self.filtered.len() {
            self.selected += 1;
        }
    }

    /// Get the currently selected command name, if any.
    pub fn selected_command(&self) -> Option<&'static str> {
        self.filtered.get(self.selected).map(|cmd| cmd.name)
    }

    /// Close the dropdown.
    pub fn close(&mut self) {
        self.visible = false;
        self.filtered.clear();
        self.selected = 0;
    }

    /// Max items to show in dropdown.
    pub fn display_count(&self) -> usize {
        self.filtered.len().min(10)
    }
}
