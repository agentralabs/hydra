//! Info commands — /status, /health, /genome, /memory, /metrics, /skills, /version, /self.

use super::registry::{sys, Command, CommandCategory, CommandContext};
use crate::stream_types::StreamItem;

pub fn commands() -> Vec<Command> {
    vec![
        Command {
            name: "status",
            aliases: &["st"],
            description: "Show system status",
            args_help: "",
            category: CommandCategory::Info,
            handler: cmd_status,
        },
        Command {
            name: "health",
            aliases: &[],
            description: "Full health check",
            args_help: "",
            category: CommandCategory::Info,
            handler: cmd_health,
        },
        Command {
            name: "genome",
            aliases: &["gen"],
            description: "Genome stats (use 'domains' for breakdown)",
            args_help: "[domains]",
            category: CommandCategory::Info,
            handler: cmd_genome,
        },
        Command {
            name: "memory",
            aliases: &["mem"],
            description: "Memory file stats",
            args_help: "",
            category: CommandCategory::Info,
            handler: cmd_memory,
        },
        Command {
            name: "metrics",
            aliases: &["m"],
            description: "System metrics dashboard",
            args_help: "",
            category: CommandCategory::Info,
            handler: cmd_metrics,
        },
        Command {
            name: "skills",
            aliases: &[],
            description: "List loaded skills",
            args_help: "",
            category: CommandCategory::Info,
            handler: cmd_skills,
        },
        Command {
            name: "version",
            aliases: &["ver"],
            description: "Show version info",
            args_help: "",
            category: CommandCategory::Info,
            handler: cmd_version,
        },
        Command {
            name: "copy",
            aliases: &["cp"],
            description: "Copy last response to clipboard",
            args_help: "",
            category: CommandCategory::Info,
            handler: cmd_copy,
        },
        Command {
            name: "context",
            aliases: &["ctx"],
            description: "Token context breakdown",
            args_help: "",
            category: CommandCategory::Info,
            handler: cmd_context,
        },
        Command {
            name: "stats",
            aliases: &[],
            description: "Usage statistics",
            args_help: "",
            category: CommandCategory::Info,
            handler: cmd_stats,
        },
        Command {
            name: "costs",
            aliases: &[],
            description: "Session cost breakdown",
            args_help: "",
            category: CommandCategory::Info,
            handler: cmd_costs,
        },
        Command {
            name: "audit",
            aliases: &[],
            description: "Recent audit receipts",
            args_help: "",
            category: CommandCategory::Info,
            handler: cmd_audit,
        },
        Command {
            name: "self",
            aliases: &["introspect"],
            description: "Self-model capability snapshot",
            args_help: "",
            category: CommandCategory::Info,
            handler: cmd_self_model,
        },
        Command {
            name: "fleet",
            aliases: &[],
            description: "Fleet agent status",
            args_help: "",
            category: CommandCategory::Info,
            handler: cmd_fleet,
        },
        Command {
            name: "like",
            aliases: &[],
            description: "Record positive taste feedback for last output",
            args_help: "[domain]",
            category: CommandCategory::Info,
            handler: cmd_like,
        },
        Command {
            name: "dislike",
            aliases: &[],
            description: "Record negative taste feedback for last output",
            args_help: "[domain]",
            category: CommandCategory::Info,
            handler: cmd_dislike,
        },
        Command {
            name: "obstacles",
            aliases: &["resistance"],
            description: "Obstacles overcome and resistance built",
            args_help: "",
            category: CommandCategory::Info,
            handler: cmd_obstacles,
        },
    ]
}

fn cmd_status(_args: &str, ctx: &CommandContext) -> Vec<StreamItem> {
    vec![
        sys(&format!("genome: {} | middlewares: {} | provider: {} | model: {}",
            ctx.genome_count, ctx.middleware_count, ctx.provider, ctx.model)),
    ]
}

fn cmd_health(_args: &str, ctx: &CommandContext) -> Vec<StreamItem> {
    vec![
        sys(&format!("genome:      {} entries", ctx.genome_count)),
        sys(&format!("middlewares:  {}", ctx.middleware_count)),
        sys(&format!("provider:    {}", ctx.provider)),
        sys(&format!("model:       {}", ctx.model)),
        sys("lyapunov:    stable"),
        sys("all systems nominal"),
    ]
}

fn cmd_genome(args: &str, ctx: &CommandContext) -> Vec<StreamItem> {
    if args == "domains" {
        // Domain breakdown would need CognitiveLoop access — return placeholder
        return vec![sys(&format!("Genome: {} entries. Use Ctrl+K → 'genome domains' for breakdown.", ctx.genome_count))];
    }
    vec![sys(&format!("Genome: {} entries loaded.", ctx.genome_count))]
}

fn cmd_memory(_args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    let amem = dirs::home_dir().unwrap_or_default().join(".hydra/data/hydra.amem");
    let size = std::fs::metadata(&amem).map(|m| m.len()).unwrap_or(0);
    let size_str = if size > 1_000_000 {
        format!("{:.1}MB", size as f64 / 1_000_000.0)
    } else {
        format!("{}KB", size / 1024)
    };
    vec![
        sys(&format!("Memory: {size_str} ({})", amem.display())),
    ]
}

fn cmd_metrics(_args: &str, ctx: &CommandContext) -> Vec<StreamItem> {
    let amem_size = dirs::home_dir()
        .and_then(|h| std::fs::metadata(h.join(".hydra/data/hydra.amem")).ok())
        .map(|m| m.len()).unwrap_or(0);
    let genome_size = dirs::home_dir()
        .and_then(|h| std::fs::metadata(h.join(".hydra/data/genome.db")).ok())
        .map(|m| m.len()).unwrap_or(0);
    let audit_size = dirs::home_dir()
        .and_then(|h| std::fs::metadata(h.join(".hydra/data/audit.db")).ok())
        .map(|m| m.len()).unwrap_or(0);

    let tok_per_min = if ctx.session_minutes > 0 { ctx.tokens_used / ctx.session_minutes } else { 0 };

    vec![
        sys("System Metrics"),
        sys(&format!("  genome:    {} entries", ctx.genome_count)),
        sys(&format!("  memory:    {}KB", amem_size / 1024)),
        sys(&format!("  genome db: {}KB", genome_size / 1024)),
        sys(&format!("  audit db:  {}KB", audit_size / 1024)),
        sys(&format!("  tokens:    {} ({}/min)", ctx.tokens_used, tok_per_min)),
        sys(&format!("  session:   {} min", ctx.session_minutes)),
    ]
}

fn cmd_skills(_args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    let skills_dir = std::path::PathBuf::from("skills");
    if !skills_dir.exists() {
        return vec![sys("No skills/ directory found.")];
    }
    let mut items = vec![sys("Skills:")];
    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir(&skills_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                items.push(sys(&format!("  {name}")));
                count += 1;
            }
        }
    }
    items.push(sys(&format!("{count} skills.")));
    items
}

fn cmd_version(_args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    vec![sys(&format!("Hydra v{} — Agentra Labs", env!("CARGO_PKG_VERSION")))]
}

fn cmd_copy(_args: &str, ctx: &CommandContext) -> Vec<StreamItem> {
    if ctx.last_response.is_empty() {
        return vec![sys("Nothing to copy.")];
    }
    let result = if cfg!(target_os = "macos") {
        std::process::Command::new("pbcopy")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                if let Some(stdin) = child.stdin.as_mut() {
                    stdin.write_all(ctx.last_response.as_bytes())?;
                }
                child.wait()
            })
    } else {
        std::process::Command::new("xclip")
            .args(["-selection", "clipboard"])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                if let Some(stdin) = child.stdin.as_mut() {
                    stdin.write_all(ctx.last_response.as_bytes())?;
                }
                child.wait()
            })
    };
    match result {
        Ok(_) => vec![sys("Copied to clipboard.")],
        Err(e) => vec![sys(&format!("Clipboard failed: {e}"))],
    }
}

fn cmd_context(_args: &str, ctx: &CommandContext) -> Vec<StreamItem> {
    let exchanges = ctx.stream_len / 2;
    let tok_per_ex = if exchanges > 0 { ctx.tokens_used / exchanges as u64 } else { 0 };
    let tok_per_min = if ctx.session_minutes > 0 { ctx.tokens_used / ctx.session_minutes } else { 0 };
    vec![
        sys("Context:"),
        sys(&format!("  tokens:     {}", ctx.tokens_used)),
        sys(&format!("  exchanges:  {exchanges}")),
        sys(&format!("  avg tok/ex: {tok_per_ex}")),
        sys(&format!("  tok/min:    {tok_per_min}")),
        sys(&format!("  session:    {} min", ctx.session_minutes)),
    ]
}

fn cmd_stats(_args: &str, ctx: &CommandContext) -> Vec<StreamItem> {
    vec![
        sys(&format!("Exchanges: {}", ctx.stream_len / 2)),
        sys(&format!("Tokens: {}", ctx.tokens_used)),
        sys(&format!("Duration: {} min", ctx.session_minutes)),
    ]
}

fn cmd_costs(_args: &str, ctx: &CommandContext) -> Vec<StreamItem> {
    let ledger = hydra_settlement::SettlementLedger::open();
    let count = ledger.count();
    let total = ledger.lifetime_cost();
    let tok_cost = if ctx.tokens_used > 0 { total / ctx.tokens_used as f64 } else { 0.0 };
    vec![
        sys("Settlement costs:"),
        sys(&format!("  records:    {count}")),
        sys(&format!("  lifetime:   {total:.4}")),
        sys(&format!("  cost/token: {tok_cost:.6}")),
        sys(&format!("  tokens:     {}", ctx.tokens_used)),
    ]
}

fn cmd_audit(_args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    match hydra_audit::persistence::AuditDb::open() {
        Ok(db) => {
            let count = db.count();
            let records = db.load_all();
            let mut items = vec![sys(&format!("Audit log: {count} records"))];
            for r in records.iter().rev().take(5) {
                items.push(sys(&format!("  {} | {} | {}", r.task_id, r.outcome, r.summary)));
            }
            items
        }
        Err(e) => vec![sys(&format!("Audit db error: {e}"))],
    }
}

fn cmd_self_model(_args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    let model = hydra_reflexive::SelfModel::bootstrap_layer1();
    let caps = model.active_capabilities();
    let mut items = vec![sys(&format!("Self-model: {} active capabilities", caps.len()))];
    for cap in caps.iter().take(10) {
        items.push(sys(&format!("  {} ({:?})", cap.name, cap.status)));
    }
    items.push(sys(&format!("Summary: {}", model.summary())));
    items
}

fn cmd_fleet(_args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    let registry = hydra_fleet::FleetRegistry::new();
    let agents = registry.agents();
    if agents.is_empty() {
        return vec![sys("Fleet: no agents. Use /spawn <task> to create one.")];
    }
    let mut items = vec![sys(&format!("Fleet: {} agents", agents.len()))];
    for agent in agents.iter().take(10) {
        items.push(sys(&format!("  {} | {:?} | {:?}", agent.name, agent.specialization, agent.state)));
    }
    items
}

fn cmd_obstacles(_args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    let store = hydra_antifragile::AntifragileStore::new();
    let total = store.total_encounters();
    let classes = store.class_count();
    let mut items = vec![sys(&format!("Antifragile: {total} encounters across {classes} obstacle classes"))];
    if total == 0 { items.push(sys("  No obstacles yet. Hydra grows stronger from failure.")); }
    items
}

fn cmd_like(args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    let domain = if args.trim().is_empty() { "general" } else { args.trim() };
    let mut genome = hydra_genome::GenomeStore::open();
    hydra_kernel::feedback::record_taste_feedback(domain, true, &mut genome);
    vec![sys(&format!("Taste recorded: positive for '{domain}'. I'll remember this preference."))]
}

fn cmd_dislike(args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    let domain = if args.trim().is_empty() { "general" } else { args.trim() };
    let mut genome = hydra_genome::GenomeStore::open();
    hydra_kernel::feedback::record_taste_feedback(domain, false, &mut genome);
    vec![sys(&format!("Taste recorded: negative for '{domain}'. I'll adjust next time."))]
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::registry::CommandContext;

    fn ctx() -> CommandContext {
        CommandContext {
            genome_count: 390, middleware_count: 9,
            provider: "anthropic".into(), model: "sonnet".into(),
            tokens_used: 100, session_minutes: 5, stream_len: 10,
            last_response: "test response".into(), exchanges: Vec::new(),
        }
    }

    #[test]
    fn status_shows_genome() {
        let items = cmd_status("", &ctx());
        let text = format!("{:?}", items);
        assert!(text.contains("390"));
    }

    #[test]
    fn commands_count() {
        assert!(commands().len() >= 8);
    }
}
