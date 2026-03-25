//! Owner guardrail commands — /guardrail and /evolution.
//! Control Hydra's self-governance: pause, resume, kill, review evolution proposals.

use super::registry::{sys, Command, CommandCategory, CommandContext};
use crate::stream_types::StreamItem;

pub fn commands() -> Vec<Command> {
    vec![
        Command {
            name: "guardrail",
            aliases: &["gr"],
            description: "Owner guardrail controls",
            args_help: "[pause | resume | kill | reload | status]",
            category: CommandCategory::System,
            handler: cmd_guardrail,
        },
        Command {
            name: "evolution",
            aliases: &["evo"],
            description: "Review self-evolution proposals",
            args_help: "[approve <id> | reject <id> [reason] | history]",
            category: CommandCategory::System,
            handler: cmd_evolution,
        },
    ]
}

fn cmd_guardrail(args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    let subcmd = args.trim();
    match subcmd {
        "" | "status" => guardrail_status(),
        "pause" => guardrail_pause(),
        "resume" => guardrail_resume(),
        "kill" => guardrail_kill(),
        "reload" => guardrail_reload(),
        _ => vec![sys(&format!("Unknown: /guardrail {subcmd}. Use: pause | resume | kill | reload | status"))],
    }
}

fn guardrail_status() -> Vec<StreamItem> {
    let engine = hydra_kernel::guardrail::GuardrailEngine::new();
    let mut items = vec![sys(&format!("--- GUARDRAIL STATUS ---\n{}", engine.status_summary()))];
    let config = &engine.config;
    items.push(sys(&format!("Forbidden paths: {:?}", config.forbidden_paths)));
    items.push(sys(&format!("Approval threshold: {}", config.require_approval_above)));
    items.push(sys(&format!("Remote kill: {} | Dead-man: {}",
        if config.remote_kill_enabled { "enabled" } else { "disabled" },
        config.dead_man_switch_days.map(|d| format!("{d} days")).unwrap_or("off".into()))));
    let pending = engine.pending_evolutions();
    if !pending.is_empty() {
        items.push(sys(&format!("\n--- PENDING EVOLUTIONS ({}) ---", pending.len())));
        for p in &pending {
            items.push(sys(&format!("  {} | {} | {} entries | blast={} | {}",
                &p.id[..p.id.len().min(12)], p.name, p.entries, p.blast_radius,
                p.proposed_at.format("%Y-%m-%d %H:%M"))));
        }
    }
    items
}

fn guardrail_pause() -> Vec<StreamItem> {
    let mut engine = hydra_kernel::guardrail::GuardrailEngine::new();
    engine.pause();
    vec![sys("Hydra PAUSED. Evolution and proactive initiation suspended.\nUse /guardrail resume to restore.")]
}

fn guardrail_resume() -> Vec<StreamItem> {
    let mut engine = hydra_kernel::guardrail::GuardrailEngine::new();
    engine.resume();
    vec![sys("Hydra RESUMED. All systems active.\nKILL file cleared if present.")]
}

fn guardrail_kill() -> Vec<StreamItem> {
    let kill_path = dirs::home_dir().unwrap_or_default().join(".hydra/KILL");
    let content = format!("kill requested at {}\nreason: owner initiated via /guardrail kill",
        chrono::Utc::now());
    match std::fs::write(&kill_path, content) {
        Ok(()) => vec![sys("KILL SIGNAL sent. Hydra will shut down on next ambient tick.")],
        Err(e) => vec![sys(&format!("Failed to create KILL file: {e}"))],
    }
}

fn guardrail_reload() -> Vec<StreamItem> {
    let config = hydra_kernel::guardrail::config::GuardrailConfig::load();
    vec![sys(&format!("Guardrail config reloaded.\nForbidden: {:?}\nApproval: {}\nDead-man: {}",
        config.forbidden_paths, config.require_approval_above,
        config.dead_man_switch_days.map(|d| format!("{d} days")).unwrap_or("off".into())))]
}

fn cmd_evolution(args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    let parts: Vec<&str> = args.trim().splitn(3, ' ').collect();
    match parts.first().copied().unwrap_or("") {
        "" => evolution_list(),
        "approve" => {
            let id = parts.get(1).unwrap_or(&"");
            if id.is_empty() { return vec![sys("Usage: /evolution approve <id>")]; }
            evolution_approve(id)
        }
        "reject" => {
            let id = parts.get(1).unwrap_or(&"");
            let reason = parts.get(2).unwrap_or(&"owner rejected");
            if id.is_empty() { return vec![sys("Usage: /evolution reject <id> [reason]")]; }
            evolution_reject(id, reason)
        }
        "history" => evolution_history(),
        other => vec![sys(&format!("Unknown: /evolution {other}. Use: approve <id> | reject <id> | history"))],
    }
}

fn evolution_list() -> Vec<StreamItem> {
    let pending = hydra_kernel::guardrail::evolution_gate::load_pending();
    if pending.is_empty() {
        return vec![sys("No pending evolution proposals. Hydra has nothing awaiting your approval.")];
    }
    let mut items = vec![sys(&format!("--- PENDING EVOLUTION PROPOSALS ({}) ---", pending.len()))];
    for p in &pending {
        items.push(sys(&format!(
            "  ID: {}\n  Name: {} | Domain: {} | Entries: {} | Blast: {}\n  Path: {}\n  Proposed: {}\n",
            p.id, p.name, p.domain, p.entries, p.blast_radius, p.skill_path,
            p.proposed_at.format("%Y-%m-%d %H:%M"))));
    }
    items.push(sys("Use: /evolution approve <id> or /evolution reject <id> [reason]"));
    items
}

fn evolution_approve(id: &str) -> Vec<StreamItem> {
    // Find matching proposal (partial ID match)
    let pending = hydra_kernel::guardrail::evolution_gate::load_pending();
    let matched = pending.iter().find(|p| p.id.starts_with(id));
    match matched {
        Some(p) => {
            let full_id = p.id.clone();
            let name = p.name.clone();
            if hydra_kernel::guardrail::evolution_gate::approve(&full_id) {
                hydra_kernel::guardrail::audit::record_quick(
                    hydra_kernel::guardrail::audit::AuditEventType::EvolutionApproved,
                    &format!("Approved: {name} ({full_id})"));
                vec![sys(&format!("APPROVED: {name}\nHydra will load this skill on the next evolution cycle."))]
            } else {
                vec![sys(&format!("Failed to approve {id}"))]
            }
        }
        None => vec![sys(&format!("No pending proposal matching '{id}'"))],
    }
}

fn evolution_reject(id: &str, reason: &str) -> Vec<StreamItem> {
    let pending = hydra_kernel::guardrail::evolution_gate::load_pending();
    let matched = pending.iter().find(|p| p.id.starts_with(id));
    match matched {
        Some(p) => {
            let full_id = p.id.clone();
            let name = p.name.clone();
            if hydra_kernel::guardrail::evolution_gate::reject(&full_id, reason) {
                hydra_kernel::guardrail::audit::record_quick(
                    hydra_kernel::guardrail::audit::AuditEventType::EvolutionRejected,
                    &format!("Rejected: {name} — {reason}"));
                vec![sys(&format!("REJECTED: {name}\nReason: {reason}"))]
            } else {
                vec![sys(&format!("Failed to reject {id}"))]
            }
        }
        None => vec![sys(&format!("No pending proposal matching '{id}'"))],
    }
}

fn evolution_history() -> Vec<StreamItem> {
    let log = hydra_kernel::guardrail::audit::AuditLog::new();
    let entries = log.recent(20);
    if entries.is_empty() {
        return vec![sys("No guardrail audit history yet.")];
    }
    let mut items = vec![sys("--- GUARDRAIL AUDIT HISTORY (last 20) ---")];
    for entry in entries {
        items.push(sys(&format!("  {} | {:?} | {} | {}",
            entry.timestamp.format("%m-%d %H:%M"), entry.event_type,
            entry.actor, entry.details)));
    }
    items
}
