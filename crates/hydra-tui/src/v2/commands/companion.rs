//! Companion commands — /pause, /digest, /inbox, /companion.

use super::registry::{sys, Command, CommandCategory, CommandContext};
use crate::stream_types::StreamItem;

pub fn commands() -> Vec<Command> {
    vec![
        Command {
            name: "pause",
            aliases: &[],
            description: "Pause companion tasks",
            args_help: "",
            category: CommandCategory::Companion,
            handler: cmd_pause,
        },
        Command {
            name: "digest",
            aliases: &[],
            description: "Review batched signals",
            args_help: "",
            category: CommandCategory::Companion,
            handler: cmd_digest,
        },
        Command {
            name: "inbox",
            aliases: &[],
            description: "All signals received",
            args_help: "",
            category: CommandCategory::Companion,
            handler: cmd_inbox,
        },
        Command {
            name: "companion",
            aliases: &[],
            description: "Companion status",
            args_help: "",
            category: CommandCategory::Companion,
            handler: cmd_companion,
        },
        Command {
            name: "signal",
            aliases: &[],
            description: "Manage signal sources (add/mute/list)",
            args_help: "add|mute|list <source>",
            category: CommandCategory::Companion,
            handler: cmd_signal,
        },
        Command {
            name: "later",
            aliases: &[],
            description: "Defer current signal to next digest",
            args_help: "",
            category: CommandCategory::Companion,
            handler: cmd_later,
        },
    ]
}

fn cmd_pause(_args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    vec![sys("Companion paused. Use /resume to restart.")]
}

fn cmd_digest(_args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    vec![sys("Reviewing signals...")]
}

fn cmd_inbox(_args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    vec![sys("Loading inbox...")]
}

fn cmd_companion(_args: &str, ctx: &CommandContext) -> Vec<StreamItem> {
    vec![
        sys("Companion: active"),
        sys(&format!("  Provider: {} | Model: {}", ctx.provider, ctx.model)),
        sys(&format!("  Session: {} min | Tokens: {}", ctx.session_minutes, ctx.tokens_used)),
    ]
}

fn cmd_signal(args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    match args.split_once(' ').map(|(cmd, rest)| (cmd, rest.trim())).unwrap_or((args, "")) {
        ("add", source) if !source.is_empty() => vec![sys(&format!("Signal source added: {source}"))],
        ("mute", source) if !source.is_empty() => vec![sys(&format!("Signal source muted: {source}"))],
        ("list", _) | ("", _) => vec![
            sys("Signal sources:"),
            sys("  (none configured — use /signal add <source>)"),
        ],
        _ => vec![sys("Usage: /signal add|mute|list <source>")],
    }
}

fn cmd_later(_args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    vec![sys("Signal deferred to next /digest.")]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_count() {
        assert_eq!(commands().len(), 6);
    }
}
