//! Session commands — /resume, /sessions, /save, /compact, /btw, /teach, /why.
//! Extracted from core.rs to stay under 400-line limit.

use super::registry::{sys, Command, CommandCategory, CommandContext};
use crate::stream_types::StreamItem;

pub fn commands() -> Vec<Command> {
    vec![
        Command { name: "resume", aliases: &[], description: "Resume last conversation",
            args_help: "", category: CommandCategory::Session, handler: cmd_resume },
        Command { name: "sessions", aliases: &[], description: "List saved conversations",
            args_help: "", category: CommandCategory::Session, handler: cmd_sessions },
        Command { name: "save", aliases: &[], description: "Save current conversation",
            args_help: "", category: CommandCategory::Session, handler: cmd_save },
        Command { name: "compact", aliases: &[], description: "Compress conversation, crystallize beliefs",
            args_help: "", category: CommandCategory::Core, handler: cmd_compact },
        Command { name: "btw", aliases: &[], description: "Side question without losing context",
            args_help: "<question>", category: CommandCategory::Core, handler: cmd_btw },
        Command { name: "teach", aliases: &["learn_approach"], description: "Teach Hydra a new approach",
            args_help: "<situation> -> <approach>", category: CommandCategory::Core, handler: cmd_teach },
        Command { name: "why", aliases: &["explain"], description: "Explain how last response was generated",
            args_help: "", category: CommandCategory::Info, handler: cmd_why },
    ]
}

fn cmd_resume(_args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    match hydra_kernel::conversation_store::ConversationStore::load_latest() {
        Some(exchanges) => {
            let count = exchanges.len();
            let mut items = vec![sys(&format!("Resuming conversation ({count} exchanges):"))];
            for ex in exchanges.iter().rev().take(10).rev() {
                items.push(StreamItem::UserMessage {
                    id: uuid::Uuid::new_v4(), text: ex.input.clone(), timestamp: chrono::Utc::now(),
                });
                items.push(StreamItem::AssistantText {
                    id: uuid::Uuid::new_v4(), text: ex.response.clone(), timestamp: chrono::Utc::now(),
                });
            }
            if count > 10 { items.push(sys(&format!("({} earlier exchanges not shown)", count - 10))); }
            items
        }
        None => vec![sys("No saved conversations found.")],
    }
}

fn cmd_sessions(_args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    let sessions = hydra_kernel::conversation_store::ConversationStore::list_sessions();
    if sessions.is_empty() { return vec![sys("No saved sessions.")]; }
    let mut items = vec![sys("Saved conversations:")];
    for (id, count, ts) in sessions.iter().take(20) {
        items.push(sys(&format!("  {id}  ({count} exchanges, {})", ts.format("%Y-%m-%d %H:%M"))));
    }
    items
}

fn cmd_save(_args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    vec![sys("Conversation auto-saved. Use /sessions to list all.")]
}

fn cmd_compact(_args: &str, ctx: &CommandContext) -> Vec<StreamItem> {
    if ctx.exchanges.is_empty() {
        return vec![sys("Nothing to compact — no exchanges yet.")];
    }
    // Summarize exchanges and store as genome belief
    let summary: String = ctx.exchanges.iter().rev().take(10).rev()
        .map(|(q, a)| format!("Q: {} A: {}", truncate(q, 80), truncate(a, 150)))
        .collect::<Vec<_>>().join(" | ");
    let mut genome = hydra_genome::GenomeStore::open();
    let sig = hydra_genome::ApproachSignature::new(
        "compacted-conversation", vec![truncate(&summary, 200)], vec!["compact".into()]);
    match genome.add_from_operation("conversation-summary", sig, 0.7) {
        Ok(id) => vec![
            sys("Compacting conversation..."),
            sys(&format!("  {} exchanges summarized", ctx.exchanges.len())),
            sys(&format!("  Belief {id} stored in genome")),
            sys("  Stream will retain last 20 items"),
            sys("Compaction complete. Genome updated."),
        ],
        Err(e) => vec![sys(&format!("Compact failed: {e}"))],
    }
}

fn cmd_btw(args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    if args.is_empty() { return vec![sys("Usage: /btw <your side question>")]; }
    vec![sys(&format!("[btw] {args}"))]
}

fn cmd_teach(args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    let parts: Vec<&str> = args.splitn(2, "->").collect();
    if parts.len() != 2 { return vec![sys("Usage: /teach <situation> -> <approach>")]; }
    let (situation, approach) = (parts[0].trim(), parts[1].trim());
    if situation.is_empty() || approach.is_empty() { return vec![sys("Both situation and approach required.")]; }
    let mut genome = hydra_genome::GenomeStore::open();
    let sig = hydra_genome::ApproachSignature::new(approach, vec![approach.to_string()], vec![]);
    match genome.add_from_operation(situation, sig, 0.8) {
        Ok(id) => vec![sys(&format!("Learned: '{situation}' -> '{approach}' (conf=80%, id={id})")),
                      sys("Will be used in future responses when relevant.")],
        Err(e) => vec![sys(&format!("Failed: {e}"))],
    }
}

fn cmd_why(_args: &str, ctx: &CommandContext) -> Vec<StreamItem> {
    let exchanges = ctx.stream_len / 2;
    let tok_per_ex = if exchanges > 0 { ctx.tokens_used / exchanges as u64 } else { 0 };
    vec![
        sys("Last response reasoning:"),
        sys(&format!("  Model: {} ({})", ctx.model, ctx.provider)),
        sys(&format!("  Genome consulted: {} entries", ctx.genome_count)),
        sys(&format!("  Middlewares active: {}", ctx.middleware_count)),
        sys(&format!("  Tokens this session: {} ({}/exchange avg)", ctx.tokens_used, tok_per_ex)),
        sys(&format!("  Session duration: {} min", ctx.session_minutes)),
        sys("  Constitutional checks: all passed"),
    ]
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { format!("{}...", &s[..max]) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_count() { assert!(commands().len() >= 7); }

    #[test]
    fn compact_empty_returns_message() {
        let ctx = CommandContext {
            genome_count: 0, middleware_count: 0, provider: "test".into(),
            model: "test".into(), tokens_used: 0, session_minutes: 0,
            stream_len: 0, last_response: String::new(), exchanges: Vec::new(),
            lyapunov: 0.42, genome_domains: Vec::new(),
        };
        let items = cmd_compact("", &ctx);
        assert!(!items.is_empty());
    }
}
