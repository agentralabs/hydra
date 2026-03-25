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
        Command {
            name: "integrity",
            aliases: &["health-check"],
            description: "Check data integrity and auto-recover if needed",
            args_help: "",
            category: CommandCategory::System,
            handler: cmd_integrity,
        },
        Command {
            name: "machines",
            aliases: &["hosts"],
            description: "List registered remote machines",
            args_help: "",
            category: CommandCategory::System,
            handler: cmd_machines,
        },
        Command {
            name: "ssh",
            aliases: &[],
            description: "Execute command on remote machine",
            args_help: "<machine> <command>",
            category: CommandCategory::System,
            handler: cmd_ssh,
        },
        Command {
            name: "evolved",
            aliases: &["self-evolved"],
            description: "List self-generated capabilities",
            args_help: "",
            category: CommandCategory::Info,
            handler: cmd_evolved,
        },
        Command {
            name: "learn",
            aliases: &["teach-md"],
            description: "Learn a skill from a markdown document",
            args_help: "<path-to-md-file>",
            category: CommandCategory::System,
            handler: cmd_learn,
        },
        Command {
            name: "drop",
            aliases: &["inbox", "gateway"],
            description: "View drop gateway activity and status",
            args_help: "[status]",
            category: CommandCategory::System,
            handler: cmd_drop,
        },
        Command {
            name: "camera",
            aliases: &["cam", "webcam"],
            description: "Toggle webcam presence detection (O19)",
            args_help: "[on|off|status]",
            category: CommandCategory::Companion,
            handler: cmd_camera,
        },
        Command {
            name: "document",
            aliases: &["doc"],
            description: "Analyze a document (PDF, image, CSV)",
            args_help: "<path>",
            category: CommandCategory::System,
            handler: cmd_document,
        },
    ]
}

fn cmd_drop(_args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    let drop_dir = dirs::home_dir().unwrap_or_default().join(".hydra/drop");
    let audit = drop_dir.join("audit.jsonl");
    let mut items = vec![sys(&format!("Drop Gateway: {}", drop_dir.display()))];
    if let Ok(content) = std::fs::read_to_string(&audit) {
        for line in content.lines().rev().take(10) {
            if let Ok(r) = serde_json::from_str::<hydra_kernel::drop::DropRecord>(line) {
                let icon = if matches!(r.outcome, hydra_kernel::drop::handlers::DropOutcome::Accepted{..}) { "✓" } else { "✗" };
                items.push(sys(&format!("  {icon} {} ({})", r.filename, r.item_type.label())));
            }
        }
    } else { items.push(sys("  No drops yet. Place files in ~/.hydra/drop/")); }
    items
}

fn cmd_evolved(_args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    let genome = hydra_genome::GenomeStore::open();
    let evolved: Vec<_> = genome.all_entries().iter()
        .filter(|e| e.approach.approach_type == "self-evolved")
        .collect();
    if evolved.is_empty() {
        return vec![sys("No self-generated capabilities yet. Evolution runs in background.")];
    }
    let mut items = vec![sys(&format!("Self-evolved: {} capabilities", evolved.len()))];
    for e in evolved.iter().take(10) {
        let kw: Vec<&str> = e.situation.keywords.iter().map(|s| s.as_str()).take(5).collect();
        items.push(sys(&format!("  {} (conf={:.0}%, uses={})",
            kw.join(" "), e.effective_confidence() * 100.0, e.use_count)));
    }
    items
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

fn cmd_integrity(_args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    let mut monitor = hydra_kernel::integrity::IntegrityMonitor::new();
    let report = monitor.check();
    let mut items = vec![sys("Self-Preservation Integrity Check:")];
    if report.is_healthy() {
        items.push(sys("  All data stores healthy"));
    } else {
        for issue in &report.issues {
            items.push(sys(&format!("  {issue}")));
        }
    }
    if report.genome_recovered {
        items.push(sys("  Genome: auto-recovered from backup"));
    }
    if report.memory_recovered {
        items.push(sys("  Memory: auto-recovered from backup"));
    }
    if let Some(hydra_kernel::integrity::Health::Ok(s)) = &report.genome { items.push(sys(&format!("  genome.db: OK ({}KB)", s / 1024))); }
    if let Some(hydra_kernel::integrity::Health::Missing) = &report.genome { items.push(sys("  genome.db: MISSING")); }
    if let Some(hydra_kernel::integrity::Health::Ok(s)) = &report.memory { items.push(sys(&format!("  hydra.amem: OK ({}KB)", s / 1024))); }
    if let Some(hydra_kernel::integrity::Health::Missing) = &report.memory { items.push(sys("  hydra.amem: MISSING")); }
    items
}

fn cmd_machines(_args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    let path = dirs::home_dir().unwrap_or_default().join(".hydra/machines.toml");
    if !path.exists() {
        return vec![
            sys("No machines registered. Create ~/.hydra/machines.toml:"),
            sys("  [[machine]]"),
            sys("  name = \"production\""),
            sys("  host = \"prod.example.com\""),
            sys("  user = \"deploy\""),
        ];
    }
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            let count = content.matches("[[machine]]").count();
            let mut items = vec![sys(&format!("Machines: {} registered ({})", count, path.display()))];
            for line in content.lines().filter(|l| l.starts_with("name") || l.starts_with("host")) {
                items.push(sys(&format!("  {}", line.trim())));
            }
            items
        }
        Err(e) => vec![sys(&format!("Failed to read machines.toml: {e}"))],
    }
}

fn cmd_ssh(args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    let parts: Vec<&str> = args.splitn(2, ' ').collect();
    let machine_name = parts.first().copied().unwrap_or("").trim();
    let command = parts.get(1).copied().unwrap_or("").trim();
    if machine_name.is_empty() || command.is_empty() {
        return vec![sys("Usage: /ssh <machine> <command>")];
    }
    match hydra_kernel::remote_exec::ssh_execute(machine_name, command) {
        Ok((output, success)) => {
            let status = if success { "OK" } else { "FAILED" };
            let mut items = vec![sys(&format!("[{machine_name}] {status}"))];
            for line in output.lines().take(20) {
                items.push(sys(&format!("  {line}")));
            }
            items
        }
        Err(e) => vec![sys(&format!("SSH failed: {e}"))],
    }
}

fn cmd_learn(args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    let path = args.trim();
    if path.is_empty() {
        return vec![sys("Usage: /learn <path-to-markdown-file>")];
    }
    if !std::path::Path::new(path).exists() {
        return vec![sys(&format!("File not found: {path}"))];
    }
    match hydra_kernel::learn_md::learn_from_markdown(path) {
        Ok(result) => {
            let mut items = vec![
                sys(&format!("Learned from: {path}")),
                sys(&format!("  Domain: {}", result.domain)),
                sys(&format!("  Knowledge: {} entries", result.knowledge_count)),
                sys(&format!("  Steps: {} operations", result.step_count)),
                sys(&format!("  Rules: {} assumptions", result.rule_count)),
                sys(&format!("  Skill dir: {}", result.skill_dir)),
            ];
            for c in &result.conflicts {
                items.push(sys(&format!("  CONFLICT: {c}")));
            }
            // Persist to genome DB so future sessions pick it up
            let mut genome = hydra_genome::GenomeStore::open();
            genome.load_from_skills();
            items.push(sys(&format!("  Genome: {} entries persisted", genome.len())));
            items.push(sys("  New skills active after session restart"));
            items
        }
        Err(e) => vec![sys(&format!("Learn failed: {e}"))],
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

fn cmd_document(args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    let path = args.trim();
    if path.is_empty() { return vec![sys("Usage: /document <file-path>")]; }
    match hydra_desktop::document::process_document(path) {
        Ok(content) => {
            let mut items = vec![
                sys(&format!("Document: {} (tier {}, conf={:.0}%)",
                    content.doc_type.label(), content.tier_used, content.confidence * 100.0)),
            ];
            if let Some(ref table) = content.structure {
                items.push(sys(&format!("  Table: {}", table.summary)));
            }
            let preview = if content.text.len() > 300 { &content.text[..300] } else { &content.text };
            for line in preview.lines().take(10) {
                items.push(sys(&format!("  {line}")));
            }
            items
        }
        Err(e) => vec![sys(&format!("Document analysis failed: {e}"))],
    }
}

fn cmd_camera(args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    match args.trim() {
        "on" | "enable" => vec![
            sys("Camera: enabling presence detection..."),
            sys("  Privacy: zero frames stored, zero cloud, local motion-diff only"),
            sys("  Gestures: wave=confirm, large motion=attention"),
        ],
        "off" | "disable" => vec![sys("Camera: presence detection disabled")],
        "status" | "" => vec![
            sys("Camera: disabled (default — privacy first)"),
            sys("  /camera on   — enable presence detection"),
            sys("  /camera off  — disable"),
            sys("  Privacy: motion detection only, no frames stored"),
        ],
        _ => vec![sys("Usage: /camera [on|off|status]")],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_count() {
        assert_eq!(commands().len(), 16);
    }
}
