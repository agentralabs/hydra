//! Command registry — single source of truth for all slash commands.
//! Every command has metadata: name, aliases, description, args, category.
//! Fuzzy search for command palette. No scattered dispatch.

use crate::stream_types::StreamItem;
use crate::v2::modal::fuzzy_score;

/// Category for grouping commands.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CommandCategory {
    Core,
    Info,
    Session,
    Companion,
    System,
    Voice,
}

impl CommandCategory {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Core => "Core",
            Self::Info => "Info",
            Self::Session => "Session",
            Self::Companion => "Companion",
            Self::System => "System",
            Self::Voice => "Voice",
        }
    }
}

/// One registered command.
#[derive(Clone)]
pub struct Command {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub description: &'static str,
    pub args_help: &'static str,
    pub category: CommandCategory,
    pub handler: fn(&str, &CommandContext) -> Vec<StreamItem>,
}

impl std::fmt::Debug for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Command")
            .field("name", &self.name)
            .field("category", &self.category)
            .finish()
    }
}

/// Context passed to every command handler.
pub struct CommandContext {
    pub genome_count: usize,
    pub middleware_count: usize,
    pub provider: String,
    pub model: String,
    pub tokens_used: u64,
    pub session_minutes: u64,
    pub stream_len: usize,
    pub last_response: String,
    pub exchanges: Vec<(String, String)>,
}

/// The command registry.
pub struct CommandRegistry {
    commands: Vec<Command>,
}

impl CommandRegistry {
    pub fn new(commands: Vec<Command>) -> Self {
        Self { commands }
    }

    /// Find a command by name or alias.
    pub fn find(&self, name: &str) -> Option<&Command> {
        let clean = name.trim_start_matches('/');
        self.commands.iter().find(|c| {
            c.name == clean || c.aliases.contains(&clean)
        })
    }

    /// Dispatch a slash command string.
    pub fn dispatch(&self, input: &str, ctx: &CommandContext) -> Vec<StreamItem> {
        let trimmed = input.trim();
        let (cmd_str, args) = match trimmed.split_once(' ') {
            Some((c, a)) => (c, a.trim()),
            None => (trimmed, ""),
        };
        let clean = cmd_str.trim_start_matches('/');

        match self.find(clean) {
            Some(cmd) => (cmd.handler)(args, ctx),
            None => vec![sys(&format!(
                "Unknown command: /{clean}. Press Ctrl+K for command palette."
            ))],
        }
    }

    /// Fuzzy search commands. Returns (index, score) sorted by score desc.
    pub fn fuzzy_search(&self, query: &str) -> Vec<(usize, u32)> {
        let mut results: Vec<(usize, u32)> = self
            .commands
            .iter()
            .enumerate()
            .filter_map(|(i, cmd)| {
                let name_score = fuzzy_score(query, cmd.name);
                let desc_score = fuzzy_score(query, cmd.description) / 2;
                let alias_score = cmd
                    .aliases
                    .iter()
                    .map(|a| fuzzy_score(query, a))
                    .max()
                    .unwrap_or(0);
                let best = name_score.max(desc_score).max(alias_score);
                if best > 0 {
                    Some((i, best))
                } else {
                    None
                }
            })
            .collect();
        results.sort_by(|a, b| b.1.cmp(&a.1));
        results
    }

    /// Get all commands.
    pub fn all(&self) -> &[Command] {
        &self.commands
    }

    /// Get commands by category.
    pub fn by_category(&self, category: CommandCategory) -> Vec<&Command> {
        self.commands
            .iter()
            .filter(|c| c.category == category)
            .collect()
    }

    pub fn count(&self) -> usize {
        self.commands.len()
    }
}

/// Helper to create a system notification item.
pub fn sys(content: &str) -> StreamItem {
    StreamItem::SystemNotification {
        id: uuid::Uuid::new_v4(),
        content: content.to_string(),
        timestamp: chrono::Utc::now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_handler(_args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
        vec![sys("test output")]
    }

    fn test_registry() -> CommandRegistry {
        CommandRegistry::new(vec![
            Command {
                name: "help",
                aliases: &["h", "?"],
                description: "Show available commands",
                args_help: "",
                category: CommandCategory::Core,
                handler: test_handler,
            },
            Command {
                name: "status",
                aliases: &["st"],
                description: "Show system status",
                args_help: "",
                category: CommandCategory::Info,
                handler: test_handler,
            },
            Command {
                name: "health",
                aliases: &[],
                description: "Full health check",
                args_help: "",
                category: CommandCategory::Info,
                handler: test_handler,
            },
        ])
    }

    fn test_ctx() -> CommandContext {
        CommandContext {
            genome_count: 390, middleware_count: 9,
            provider: "anthropic".into(), model: "sonnet".into(),
            tokens_used: 0, session_minutes: 0, stream_len: 0,
            last_response: String::new(), exchanges: Vec::new(),
        }
    }

    #[test]
    fn find_by_name() {
        let reg = test_registry();
        assert!(reg.find("help").is_some());
        assert!(reg.find("status").is_some());
        assert!(reg.find("nonexistent").is_none());
    }

    #[test]
    fn find_by_alias() {
        let reg = test_registry();
        assert!(reg.find("h").is_some());
        assert!(reg.find("st").is_some());
    }

    #[test]
    fn dispatch_known_command() {
        let reg = test_registry();
        let ctx = test_ctx();
        let result = reg.dispatch("/help", &ctx);
        assert!(!result.is_empty());
    }

    #[test]
    fn dispatch_unknown_command() {
        let reg = test_registry();
        let ctx = test_ctx();
        let result = reg.dispatch("/xyz", &ctx);
        assert!(!result.is_empty());
        // Should contain "Unknown command"
    }

    #[test]
    fn fuzzy_search_matches() {
        let reg = test_registry();
        let results = reg.fuzzy_search("hlth");
        assert!(!results.is_empty());
        // health should be the top match
        let top_cmd = &reg.all()[results[0].0];
        assert_eq!(top_cmd.name, "health");
    }

    #[test]
    fn fuzzy_search_empty_returns_all() {
        let reg = test_registry();
        let results = reg.fuzzy_search("");
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn by_category() {
        let reg = test_registry();
        let info = reg.by_category(CommandCategory::Info);
        assert_eq!(info.len(), 2); // status + health
    }
}
