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
    Session,
    Model,
    Code,
    Config,
    Integration,
    Agent,
    Hydra,
    Developer,
    System,
    Control,
    Debug,
}

/// All available slash commands.
pub const COMMANDS: &[SlashCommand] = &[
    // ── Session Management (Hydra) ──
    SlashCommand { name: "/help",     description: "List all available commands",      category: CommandCategory::Session },
    SlashCommand { name: "/exit",     description: "Exit the session",                 category: CommandCategory::Session },
    SlashCommand { name: "/clear",    description: "Clear conversation history",       category: CommandCategory::Session },
    SlashCommand { name: "/compact",  description: "Compress conversation",            category: CommandCategory::Session },
    SlashCommand { name: "/resume",   description: "Resume previous session",          category: CommandCategory::Session },
    SlashCommand { name: "/continue", description: "Alias for /resume",                category: CommandCategory::Session },
    SlashCommand { name: "/rename",   description: "Name current session",             category: CommandCategory::Session },
    SlashCommand { name: "/fork",     description: "Branch conversation",              category: CommandCategory::Session },
    SlashCommand { name: "/export",   description: "Export conversation to file",      category: CommandCategory::Session },
    SlashCommand { name: "/context",  description: "Visualize context window usage",   category: CommandCategory::Session },
    SlashCommand { name: "/history",  description: "Show conversation history",        category: CommandCategory::Session },
    SlashCommand { name: "/copy",     description: "Copy last response to clipboard",   category: CommandCategory::Session },

    // ── Model & Cost (Hydra §5.2) ──
    SlashCommand { name: "/model",    description: "Switch model",                     category: CommandCategory::Model },
    SlashCommand { name: "/cost",     description: "Show token usage for session",     category: CommandCategory::Model },
    SlashCommand { name: "/tokens",   description: "Show token usage stats",           category: CommandCategory::Model },
    SlashCommand { name: "/usage",    description: "Check progress against budget",    category: CommandCategory::Model },
    SlashCommand { name: "/fast",     description: "Toggle Fast Mode (2.5x speed)",    category: CommandCategory::Model },

    // ── Code & Review (Hydra §5.3) ──
    SlashCommand { name: "/diff",     description: "Show uncommitted changes",         category: CommandCategory::Code },
    SlashCommand { name: "/rewind",   description: "Rewind conversation and/or code",  category: CommandCategory::Code },
    SlashCommand { name: "/review",   description: "Request code review",              category: CommandCategory::Code },
    SlashCommand { name: "/todos",    description: "List tracked TODO items",           category: CommandCategory::Code },
    SlashCommand { name: "/add-dir",  description: "Add additional working directory",  category: CommandCategory::Code },

    // ── Configuration (Hydra §5.4) ──
    SlashCommand { name: "/config",         description: "Open settings interface",           category: CommandCategory::Config },
    SlashCommand { name: "/memory",         description: "Set memory capture mode (all/facts/none)",  category: CommandCategory::Config },
    SlashCommand { name: "/init",           description: "Initialize project with HYDRA.md",  category: CommandCategory::Config },
    SlashCommand { name: "/doctor",         description: "Health check (API, MCP, perms)",    category: CommandCategory::Config },
    SlashCommand { name: "/sidebar",        description: "Toggle sidebar",                    category: CommandCategory::Config },
    SlashCommand { name: "/vim",            description: "Toggle vim mode",                   category: CommandCategory::Config },
    SlashCommand { name: "/terminal-setup", description: "Install terminal keybindings",      category: CommandCategory::Config },
    SlashCommand { name: "/login",          description: "Switch accounts",                   category: CommandCategory::Config },
    SlashCommand { name: "/logout",         description: "Sign out",                          category: CommandCategory::Config },
    SlashCommand { name: "/keybindings",    description: "Edit keybinding configuration",     category: CommandCategory::Config },
    SlashCommand { name: "/output-style",   description: "Change output formatting style",    category: CommandCategory::Config },

    // ── Developer ──
    SlashCommand { name: "/files",    description: "List project files (tree view)",   category: CommandCategory::Developer },
    SlashCommand { name: "/open",     description: "Read and display file content",    category: CommandCategory::Developer },
    SlashCommand { name: "/edit",     description: "Open file in $EDITOR",             category: CommandCategory::Developer },
    SlashCommand { name: "/search",   description: "Search code (regex or semantic)",  category: CommandCategory::Developer },
    SlashCommand { name: "/symbols",  description: "Show functions/structs in file",   category: CommandCategory::Developer },
    SlashCommand { name: "/impact",   description: "Show what depends on a file",      category: CommandCategory::Developer },
    SlashCommand { name: "/git",      description: "Git status/log/commit/push/pr",    category: CommandCategory::Developer },
    SlashCommand { name: "/test",     description: "Run project tests",                category: CommandCategory::Developer },
    SlashCommand { name: "/build",    description: "Build the project",                category: CommandCategory::Developer },
    SlashCommand { name: "/run",      description: "Run the project",                  category: CommandCategory::Developer },
    SlashCommand { name: "/lint",     description: "Run linter",                       category: CommandCategory::Developer },
    SlashCommand { name: "/fmt",      description: "Format code",                      category: CommandCategory::Developer },
    SlashCommand { name: "/deps",     description: "Show/update dependencies",         category: CommandCategory::Developer },
    SlashCommand { name: "/bench",    description: "Run benchmarks",                   category: CommandCategory::Developer },
    SlashCommand { name: "/doc",      description: "Generate/open docs",               category: CommandCategory::Developer },
    SlashCommand { name: "/deploy",    description: "Deploy to configured target",      category: CommandCategory::Developer },
    SlashCommand { name: "/test-repo", description: "Run full repo test suite",        category: CommandCategory::Developer },

    // ── Integrations (Hydra §5.5) ──
    SlashCommand { name: "/mcp",                description: "Manage MCP server connections",  category: CommandCategory::Integration },
    SlashCommand { name: "/ide",                description: "Manage IDE integrations",         category: CommandCategory::Integration },
    SlashCommand { name: "/install-github-app", description: "Set up GitHub Actions",           category: CommandCategory::Integration },
    SlashCommand { name: "/hooks",              description: "View/manage hook configurations", category: CommandCategory::Integration },
    SlashCommand { name: "/plugin",             description: "Manage plugins",                  category: CommandCategory::Integration },
    SlashCommand { name: "/remote-control",     description: "Enable control from web UI",      category: CommandCategory::Integration },
    SlashCommand { name: "/remote",             description: "Connect to remote session",       category: CommandCategory::Integration },
    SlashCommand { name: "/ssh",               description: "Connect to remote machine via SSH", category: CommandCategory::Integration },
    SlashCommand { name: "/ssh-exec",          description: "Execute command on remote machine", category: CommandCategory::Integration },
    SlashCommand { name: "/ssh-upload",        description: "Upload file to remote machine",    category: CommandCategory::Integration },
    SlashCommand { name: "/ssh-download",      description: "Download file from remote machine", category: CommandCategory::Integration },
    SlashCommand { name: "/ssh-disconnect",    description: "Disconnect from remote machine",   category: CommandCategory::Integration },
    SlashCommand { name: "/ssh-list",          description: "List active SSH connections",      category: CommandCategory::Integration },

    // ── Agents & Skills (Hydra §5.6) ──
    SlashCommand { name: "/agents",   description: "Manage custom AI subagents",       category: CommandCategory::Agent },
    SlashCommand { name: "/skills",   description: "List available skills",             category: CommandCategory::Agent },
    SlashCommand { name: "/commands", description: "List all slash commands",            category: CommandCategory::Agent },
    SlashCommand { name: "/plan",     description: "Enter plan mode",                   category: CommandCategory::Agent },
    SlashCommand { name: "/bashes",   description: "List background processes",         category: CommandCategory::Agent },
    SlashCommand { name: "/tasks",       description: "View active/interrupted tasks",    category: CommandCategory::Agent },
    SlashCommand { name: "/resume-task", description: "Resume an interrupted task",      category: CommandCategory::Agent },
    SlashCommand { name: "/cancel-task", description: "Cancel and clean up a task",      category: CommandCategory::Agent },

    // ── Swarm (P9) ──
    SlashCommand { name: "/swarm",          description: "Agent swarm management (spawn/status/assign/kill)", category: CommandCategory::Agent },
    SlashCommand { name: "/swarm-spawn",    description: "Spawn N local agents",              category: CommandCategory::Agent },
    SlashCommand { name: "/swarm-status",   description: "Show all agent statuses",            category: CommandCategory::Agent },
    SlashCommand { name: "/swarm-assign",   description: "Distribute goal across idle agents", category: CommandCategory::Agent },
    SlashCommand { name: "/swarm-results",  description: "Show aggregated agent results",      category: CommandCategory::Agent },
    SlashCommand { name: "/swarm-kill",     description: "Terminate a specific agent",         category: CommandCategory::Agent },
    SlashCommand { name: "/swarm-kill-all", description: "Terminate all agents",               category: CommandCategory::Agent },
    SlashCommand { name: "/swarm-scale",    description: "Scale swarm to N agents",            category: CommandCategory::Agent },

    // ── Sister Improve (P10) ──
    SlashCommand { name: "/improve-sister", description: "Improve a sister codebase",        category: CommandCategory::Agent },

    // ── Email ──
    SlashCommand { name: "/email",       description: "Send email (usage: /email <to> <subject>)", category: CommandCategory::System },
    SlashCommand { name: "/email-setup", description: "Configure SMTP email settings",             category: CommandCategory::Config },

    // ── System ──
    SlashCommand { name: "/sisters",  description: "Show sister status",               category: CommandCategory::System },
    SlashCommand { name: "/sister",   description: "Detailed view of one sister",      category: CommandCategory::System },
    SlashCommand { name: "/health",   description: "Full system health dashboard",     category: CommandCategory::System },
    SlashCommand { name: "/status",   description: "System status summary",            category: CommandCategory::System },
    SlashCommand { name: "/fix",      description: "Repair offline sisters",           category: CommandCategory::System },
    SlashCommand { name: "/scan",     description: "Run Omniscience scan",             category: CommandCategory::System },
    SlashCommand { name: "/repair",   description: "Run self-repair specs",            category: CommandCategory::System },

    // ── Hydra-Exclusive (§5.7) ──
    SlashCommand { name: "/version",    description: "Hydra version, sisters, autonomy", category: CommandCategory::Hydra },
    SlashCommand { name: "/beliefs",    description: "Show current beliefs",             category: CommandCategory::Hydra },
    SlashCommand { name: "/goals",      description: "Show persistent goals",            category: CommandCategory::Hydra },
    SlashCommand { name: "/receipts",   description: "Show action receipt ledger",       category: CommandCategory::Hydra },
    SlashCommand { name: "/autonomy",   description: "Set autonomy level (1-5)",         category: CommandCategory::Hydra },
    SlashCommand { name: "/env",        description: "Show environment profile",         category: CommandCategory::Hydra },
    SlashCommand { name: "/dream",      description: "Show Dream State activity",        category: CommandCategory::Hydra },
    SlashCommand { name: "/obstacles",  description: "Show obstacle history",            category: CommandCategory::Hydra },
    SlashCommand { name: "/threat",     description: "Show threat correlation",          category: CommandCategory::Hydra },
    SlashCommand { name: "/implement",  description: "Trigger SelfImplement pipeline",   category: CommandCategory::Hydra },

    // ── Control ──
    SlashCommand { name: "/trust",      description: "Show/set trust level",             category: CommandCategory::Control },
    SlashCommand { name: "/approve",    description: "Approve pending action",           category: CommandCategory::Control },
    SlashCommand { name: "/deny",       description: "Deny pending action",              category: CommandCategory::Control },
    SlashCommand { name: "/kill",       description: "Kill current execution",           category: CommandCategory::Control },
    SlashCommand { name: "/diagnostics",description: "Show last consolidation report",   category: CommandCategory::Control },

    // ── Debug ──
    SlashCommand { name: "/log",        description: "Show recent logs",                 category: CommandCategory::Debug },
    SlashCommand { name: "/debug",      description: "Toggle debug mode",                category: CommandCategory::Debug },
    SlashCommand { name: "/quit",       description: "Exit Hydra",                       category: CommandCategory::Debug },
];

/// Dropdown state for slash command autocomplete.
#[derive(Clone, Debug)]
pub struct CommandDropdown {
    pub visible: bool,
    pub filtered: Vec<&'static SlashCommand>,
    pub selected: usize,
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
    const MAX_VISIBLE: usize = 12;

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
        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len().saturating_sub(1);
        }
        self.clamp_scroll();
    }

    pub fn select_prev(&mut self) {
        if !self.filtered.is_empty() {
            self.selected = self.selected.saturating_sub(1);
            if self.selected < self.scroll {
                self.scroll = self.selected;
            }
        }
    }

    pub fn select_next(&mut self) {
        if !self.filtered.is_empty() && self.selected + 1 < self.filtered.len() {
            self.selected += 1;
            if self.selected >= self.scroll + Self::MAX_VISIBLE {
                self.scroll = self.selected + 1 - Self::MAX_VISIBLE;
            }
        }
    }

    pub fn selected_command(&self) -> Option<&'static str> {
        self.filtered.get(self.selected).map(|cmd| cmd.name)
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.filtered.clear();
        self.selected = 0;
        self.scroll = 0;
    }

    pub fn display_count(&self) -> usize {
        self.filtered.len().min(Self::MAX_VISIBLE)
    }

    pub fn visible_items(&self) -> &[&'static SlashCommand] {
        let end = (self.scroll + Self::MAX_VISIBLE).min(self.filtered.len());
        &self.filtered[self.scroll..end]
    }

    fn clamp_scroll(&mut self) {
        let max_scroll = self.filtered.len().saturating_sub(Self::MAX_VISIBLE);
        if self.scroll > max_scroll {
            self.scroll = max_scroll;
        }
    }
}
