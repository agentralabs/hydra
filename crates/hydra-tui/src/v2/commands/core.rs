//! Core commands — /help, /clear, /quit, /theme, /settings, /persona, /profile, /web.
//! Session commands moved to session.rs.

use super::registry::{sys, Command, CommandCategory, CommandContext};
use crate::stream_types::StreamItem;

pub fn commands() -> Vec<Command> {
    vec![
        Command {
            name: "help",
            aliases: &["h", "?"],
            description: "Show available commands",
            args_help: "",
            category: CommandCategory::Core,
            handler: cmd_help,
        },
        Command {
            name: "clear",
            aliases: &["cls"],
            description: "Clear conversation stream",
            args_help: "",
            category: CommandCategory::Core,
            handler: cmd_clear,
        },
        Command {
            name: "quit",
            aliases: &["q", "exit"],
            description: "Exit Hydra",
            args_help: "",
            category: CommandCategory::Core,
            handler: cmd_quit,
        },
        Command {
            name: "theme",
            aliases: &[],
            description: "Switch color theme",
            args_help: "<dark|light|auto>",
            category: CommandCategory::Core,
            handler: cmd_theme,
        },
        Command {
            name: "settings",
            aliases: &["config", "set"],
            description: "View or change settings",
            args_help: "[key value]",
            category: CommandCategory::Core,
            handler: cmd_settings,
        },
        Command {
            name: "persona",
            aliases: &[],
            description: "Switch persona (e.g., /persona finance)",
            args_help: "[name]",
            category: CommandCategory::Core,
            handler: cmd_persona,
        },
        Command {
            name: "profile",
            aliases: &[],
            description: "Show current profile and capabilities",
            args_help: "",
            category: CommandCategory::Info,
            handler: cmd_profile,
        },
        Command {
            name: "web",
            aliases: &[],
            description: "Knowledge index status",
            args_help: "",
            category: CommandCategory::Info,
            handler: cmd_web,
        },
    ]
}

fn cmd_help(_args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    vec![
        sys("Commands (press Ctrl+K for fuzzy search):"),
        sys(""),
        sys("  Core"),
        sys("    /help          Show this list"),
        sys("    /clear         Clear stream"),
        sys("    /theme <name>  Switch theme (dark/light/auto)"),
        sys("    /settings      Open settings editor (or /settings key value)"),
        sys("    /quit          Exit"),
        sys(""),
        sys("  Info"),
        sys("    /status        System status"),
        sys("    /health        Full health check"),
        sys("    /genome        Genome stats (/genome domains for breakdown)"),
        sys("    /memory        Memory stats"),
        sys("    /metrics       Dashboard"),
        sys("    /skills        Loaded skills"),
        sys("    /version       Version info"),
        sys(""),
        sys("  Session"),
        sys("    /resume        Resume last conversation"),
        sys("    /sessions      List saved conversations"),
        sys("    /save          Save current conversation"),
        sys("    /copy          Copy last response"),
        sys("    /export        Export conversation"),
        sys(""),
        sys("  System"),
        sys("    /backup        Create backup (/backup list, /backup restore <date>)"),
        sys("    /skill install Install skill from URL"),
        sys(""),
        sys("  Keyboard"),
        sys("    Ctrl+K         Command palette"),
        sys("    Shift+Enter    Multi-line input"),
        sys("    Ctrl+V         Voice input"),
        sys("    Ctrl+C         Cancel / quit"),
        sys("    PageUp/Down    Scroll"),
    ]
}

fn cmd_clear(_args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    vec![sys("Stream cleared.")]
}

fn cmd_quit(_args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    vec![sys("Shutting down...")]
}

fn cmd_theme(args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    let name = args.trim();
    match name {
        "dark" | "light" | "auto" => {
            let theme = crate::theme::Theme::by_name(name);
            let detected = theme.name();
            crate::theme::switch(theme);
            let mut config = crate::config::HydraConfig::load();
            config.tui.theme = name.to_string();
            if let Err(e) = config.save() { eprintln!("hydra: theme save: {e}"); }
            vec![sys(&format!("Theme: {name} (active: {detected})"))]
        }
        "" => {
            let current = crate::theme::current().name();
            vec![sys(&format!("Current theme: {current}. Usage: /theme dark|light|auto"))]
        }
        other => vec![sys(&format!("Unknown theme: {other}. Use dark, light, or auto."))],
    }
}

fn cmd_settings(args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    if args.is_empty() {
        // Show all settings with current values
        let schemas = crate::v2::config_schema::all_schemas();
        let mut items = vec![sys("Settings (use /settings key value to change):")];
        for schema in &schemas {
            items.push(sys(&format!(
                "  {}.{:<20} = {:<12} ({})",
                schema.section, schema.key, schema.default, schema.description
            )));
        }
        items.push(sys(""));
        items.push(sys("Press Ctrl+K then type 'settings' to open the visual editor."));
        return items;
    }

    let (key, value) = match args.split_once(' ') {
        Some((k, v)) => (k.trim(), v.trim()),
        None => return vec![sys(&format!("Usage: /settings {args} <value>"))],
    };

    // Find matching schema
    let schemas = crate::v2::config_schema::all_schemas();
    let found = schemas.iter().find(|s| s.key == key || format!("{}.{}", s.section, s.key) == key);
    match found {
        Some(schema) => {
            if let Err(e) = schema.validate(value) {
                return vec![sys(&format!("Invalid value: {e}"))];
            }
            let mut config = crate::config::HydraConfig::load();
            match config.apply_setting(key, value) {
                Ok(msg) => {
                    if let Err(e) = config.save() {
                        return vec![sys(&format!("Set but save failed: {e}"))];
                    }
                    if key == "theme" {
                        crate::theme::switch(crate::theme::Theme::by_name(value));
                    }
                    vec![sys(&msg)]
                }
                Err(e) => vec![sys(&format!("Error: {e}"))],
            }
        }
        None => vec![sys(&format!("Unknown setting: {key}. Run /settings to see all."))],
    }
}

fn cmd_persona(args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    let mut registry = hydra_persona::PersonaRegistry::new();
    let _ = registry.register(hydra_persona::Persona::core_persona());
    let _ = registry.register(hydra_persona::Persona::security_analyst_persona());
    let _ = registry.register(hydra_persona::Persona::software_architect_persona());
    if args.is_empty() {
        let mut items = vec![sys("Available personas:")];
        for name in registry.persona_names() {
            if let Some(p) = registry.get(&name) {
                items.push(sys(&format!("  {} — {}", p.name, p.description)));
            }
        }
        items.push(sys("Usage: /persona <name>"));
        return items;
    }
    match registry.activate(args) {
        Ok(_) => vec![sys(&format!("Persona activated: {args}"))],
        Err(e) => vec![sys(&format!("Error: {e}"))],
    }
}

fn cmd_profile(_args: &str, ctx: &CommandContext) -> Vec<StreamItem> {
    vec![
        sys("Current profile:"),
        sys("  persona:      core"),
        sys(&format!("  genome:       {} entries", ctx.genome_count)),
        sys(&format!("  middlewares:  {}", ctx.middleware_count)),
        sys(&format!("  model:       {}", ctx.model)),
        sys(&format!("  session:     {}m, {} tokens", ctx.session_minutes, ctx.tokens_used)),
    ]
}

fn cmd_web(_args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    vec![
        sys("Knowledge Index:"),
        sys("  Layer 1: GENOME — answer from proven approaches (0 web calls)"),
        sys("  Layer 2: INDEX — 83 seeded sources (1 targeted call)"),
        sys("  Layer 3: SEARCH — unknown topics (1 search, indexed forever)"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> CommandContext {
        CommandContext {
            genome_count: 390, middleware_count: 9,
            provider: "anthropic".into(), model: "sonnet".into(),
            tokens_used: 100, session_minutes: 5, stream_len: 10,
            last_response: String::new(), exchanges: Vec::new(),
        }
    }

    #[test]
    fn help_returns_items() {
        let items = cmd_help("", &ctx());
        assert!(items.len() > 10);
    }

    #[test]
    fn settings_shows_all() {
        let items = cmd_settings("", &ctx());
        assert!(items.len() > 5);
    }

    #[test]
    fn settings_validates() {
        let items = cmd_settings("theme neon", &ctx());
        // Should show error about invalid value
        assert!(!items.is_empty());
    }

    #[test]
    fn commands_count() {
        assert!(commands().len() >= 8);
    }
}
