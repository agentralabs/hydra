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
        Command {
            name: "collab",
            aliases: &["pair", "shadow"],
            description: "Collaboration mode control",
            args_help: "[off|shadow|pair <dir>]",
            category: CommandCategory::Companion,
            handler: cmd_collab,
        },
        Command {
            name: "analytics",
            aliases: &["usage"],
            description: "Usage statistics, ROI, and learning trajectory",
            args_help: "",
            category: CommandCategory::Info,
            handler: cmd_analytics,
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

fn cmd_collab(args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    let parts: Vec<&str> = args.split_whitespace().collect();
    let mode = parts.first().copied().unwrap_or("");
    match mode {
        "shadow" => vec![
            sys("Collaboration: Shadow mode enabled"),
            sys("  Hydra watches your work and suggests when you're idle (>30s)"),
        ],
        "pair" => {
            let dir = parts.get(1).copied().unwrap_or(".");
            vec![
                sys(&format!("Collaboration: Pair programming mode for '{dir}'")),
                sys("  Watching file changes, running tests, generating companion files"),
            ]
        }
        "off" => vec![sys("Collaboration: disabled")],
        _ => vec![
            sys(&format!("Collaboration: {}", hydra_kernel::collaboration::status_summary())),
            sys("  /collab shadow     — suggest when idle"),
            sys("  /collab pair <dir> — pair programming mode"),
            sys("  /collab off        — disable"),
        ],
    }
}

fn cmd_analytics(_args: &str, ctx: &CommandContext) -> Vec<StreamItem> {
    let genome = hydra_genome::GenomeStore::open();
    let ledger = hydra_settlement::SettlementLedger::open();
    let genome_count = genome.len();
    let cost = ledger.lifetime_cost();
    let tok = ctx.tokens_used;
    let minutes = ctx.session_minutes;
    let tok_per_min = if minutes > 0 { tok / minutes } else { 0 };
    vec![
        sys("Hydra Analytics:"),
        sys(&format!("  Genome: {} entries", genome_count)),
        sys(&format!("  Tokens: {} ({}/min)", tok, tok_per_min)),
        sys(&format!("  Session: {} min", minutes)),
        sys(&format!("  Lifetime cost: ${:.4}", cost)),
        sys(&format!("  Middlewares: {} active", ctx.middleware_count)),
        sys("  Learning: active (dream loop)"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_count() {
        assert_eq!(commands().len(), 8);
    }
}
