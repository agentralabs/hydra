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
    Developer,
    System,
    Conversation,
    Settings,
    Control,
    Debug,
}

/// All available slash commands.
pub const COMMANDS: &[SlashCommand] = &[
    // ── Developer ──
    SlashCommand { name: "/files",    description: "List project files (tree view)",       category: CommandCategory::Developer },
    SlashCommand { name: "/open",     description: "Read and display file content",        category: CommandCategory::Developer },
    SlashCommand { name: "/edit",     description: "Open file in $EDITOR",                 category: CommandCategory::Developer },
    SlashCommand { name: "/search",   description: "Search code (regex or semantic)",      category: CommandCategory::Developer },
    SlashCommand { name: "/symbols",  description: "Show functions/structs/types in file", category: CommandCategory::Developer },
    SlashCommand { name: "/impact",   description: "Show what depends on a file",          category: CommandCategory::Developer },
    SlashCommand { name: "/diff",     description: "Show uncommitted changes",             category: CommandCategory::Developer },
    SlashCommand { name: "/git",      description: "Git status/log/commit/push/pr",        category: CommandCategory::Developer },
    SlashCommand { name: "/test",     description: "Run project tests",                    category: CommandCategory::Developer },
    SlashCommand { name: "/build",    description: "Build the project",                    category: CommandCategory::Developer },
    SlashCommand { name: "/run",      description: "Run the project",                      category: CommandCategory::Developer },
    SlashCommand { name: "/lint",     description: "Run linter",                           category: CommandCategory::Developer },
    SlashCommand { name: "/fmt",      description: "Format code",                          category: CommandCategory::Developer },
    SlashCommand { name: "/deps",     description: "Show/update dependencies",             category: CommandCategory::Developer },
    SlashCommand { name: "/bench",    description: "Run benchmarks",                       category: CommandCategory::Developer },
    SlashCommand { name: "/doc",      description: "Generate/open docs",                   category: CommandCategory::Developer },
    SlashCommand { name: "/deploy",   description: "Deploy to configured target",          category: CommandCategory::Developer },
    SlashCommand { name: "/init",     description: "Initialize Hydra in a new project",    category: CommandCategory::Developer },

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
    /// Scroll offset — first visible item index.
    pub scroll: usize,
}

impl Default for CommandDropdown {
    fn default() -> Self {
        Self {
            visible: false,
            filtered: Vec::new(),
            selected: 0,
            scroll: 0,
        }
    }
}

impl CommandDropdown {
    /// Max visible items in the dropdown.
    const MAX_VISIBLE: usize = 12;

    /// Update the filtered list based on current input.
    /// Called on every keystroke when input starts with "/".
    pub fn update_filter(&mut self, input: &str) {
        if !input.starts_with('/') || input.contains(' ') {
            self.visible = false;
            self.filtered.clear();
            self.selected = 0;
            self.scroll = 0;
            return;
        }

        self.filtered = COMMANDS
            .iter()
            .filter(|cmd| cmd.name.starts_with(input))
            .collect();

        self.visible = !self.filtered.is_empty();
        // Clamp selection and scroll
        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len().saturating_sub(1);
        }
        self.clamp_scroll();
    }

    /// Move selection up.
    pub fn select_prev(&mut self) {
        if !self.filtered.is_empty() {
            self.selected = self.selected.saturating_sub(1);
            // Scroll up if selection goes above visible window
            if self.selected < self.scroll {
                self.scroll = self.selected;
            }
        }
    }

    /// Move selection down.
    pub fn select_next(&mut self) {
        if !self.filtered.is_empty() && self.selected + 1 < self.filtered.len() {
            self.selected += 1;
            // Scroll down if selection goes below visible window
            if self.selected >= self.scroll + Self::MAX_VISIBLE {
                self.scroll = self.selected + 1 - Self::MAX_VISIBLE;
            }
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
        self.scroll = 0;
    }

    /// Number of visible items to render.
    pub fn display_count(&self) -> usize {
        self.filtered.len().min(Self::MAX_VISIBLE)
    }

    /// The visible slice of items (accounts for scroll offset).
    pub fn visible_items(&self) -> &[&'static SlashCommand] {
        let end = (self.scroll + Self::MAX_VISIBLE).min(self.filtered.len());
        &self.filtered[self.scroll..end]
    }

    /// Keep scroll in valid range.
    fn clamp_scroll(&mut self) {
        let max_scroll = self.filtered.len().saturating_sub(Self::MAX_VISIBLE);
        if self.scroll > max_scroll {
            self.scroll = max_scroll;
        }
    }
}
