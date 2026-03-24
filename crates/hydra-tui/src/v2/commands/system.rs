//! System commands — /backup, /skill install, /voice.

use super::registry::{sys, Command, CommandCategory, CommandContext};
use crate::stream_types::StreamItem;

pub fn commands() -> Vec<Command> {
    vec![
        Command {
            name: "backup",
            aliases: &[],
            description: "Create or manage backups",
            args_help: "[list | restore <date>]",
            category: CommandCategory::System,
            handler: cmd_backup,
        },
        Command {
            name: "skill",
            aliases: &[],
            description: "Inspect or install skills",
            args_help: "[install <url> | <name>]",
            category: CommandCategory::System,
            handler: cmd_skill,
        },
        Command {
            name: "voice",
            aliases: &[],
            description: "Voice system controls",
            args_help: "[status | setup | test]",
            category: CommandCategory::Voice,
            handler: cmd_voice,
        },
        Command {
            name: "export",
            aliases: &[],
            description: "Export conversation to file",
            args_help: "[md | json]",
            category: CommandCategory::System,
            handler: cmd_export,
        },
        Command {
            name: "browse",
            aliases: &[],
            description: "Navigate to URL and extract content",
            args_help: "<url>",
            category: CommandCategory::System,
            handler: cmd_browse,
        },
        Command {
            name: "screenshot",
            aliases: &["ss"],
            description: "Capture screen and save to file",
            args_help: "",
            category: CommandCategory::System,
            handler: cmd_screenshot,
        },
        Command {
            name: "spawn",
            aliases: &[],
            description: "Spawn a fleet agent for a task",
            args_help: "<task description>",
            category: CommandCategory::System,
            handler: cmd_spawn,
        },
    ]
}

fn cmd_backup(args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    match args {
        "list" => {
            let backups = hydra_kernel::backup::list_backups();
            if backups.is_empty() {
                return vec![sys("No backups. Run /backup to create one.")];
            }
            let mut items = vec![sys("Backups:")];
            for (name, size) in &backups {
                items.push(sys(&format!("  {name}  ({}KB)", size / 1024)));
            }
            items
        }
        s if s.starts_with("restore") => {
            let date = s.trim_start_matches("restore").trim();
            if date.is_empty() {
                return vec![sys("Usage: /backup restore <YYYY-MM-DD>")];
            }
            let dir = dirs::home_dir()
                .unwrap_or_default()
                .join(format!(".hydra/backups/{date}"));
            match hydra_kernel::backup::restore_backup(&dir) {
                Ok(r) => vec![
                    sys(&format!("Restored {} files from {date}", r.files_restored)),
                    sys(&format!("Hash verified: {}", r.hash_verified)),
                ],
                Err(e) => vec![sys(&format!("Restore failed: {e}"))],
            }
        }
        "" => match hydra_kernel::backup::create_backup() {
            Ok(r) => {
                hydra_kernel::backup::prune_old_backups(30);
                vec![sys(&format!(
                    "Backup: {} ({} files, {}KB)",
                    r.path.display(), r.files_copied, r.total_bytes / 1024
                ))]
            }
            Err(e) => vec![sys(&format!("Backup failed: {e}"))],
        },
        _ => vec![sys("Usage: /backup [list | restore <date>]")],
    }
}

fn cmd_skill(args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    if args.starts_with("install") {
        let url = args.trim_start_matches("install").trim();
        if url.is_empty() {
            return vec![sys("Usage: /skill install <url>")];
        }
        match hydra_genome::skill_loader::install_from_url(url) {
            Ok(name) => vec![sys(&format!("Installed skill: {name}"))],
            Err(e) => vec![sys(&format!("Install failed: {e}"))],
        }
    } else if args.is_empty() {
        vec![sys("Usage: /skill <name> or /skill install <url>")]
    } else {
        vec![sys(&format!("Skill: {args} (use /skills to list all)"))]
    }
}

fn cmd_voice(args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    match args {
        "status" | "" => {
            let caps = hydra_voice::VoiceCapabilities::detect();
            let mut items = vec![sys("Voice system:")];
            for line in &caps.status_lines {
                items.push(sys(&format!("  {line}")));
            }
            items
        }
        "setup" => {
            let mut items = vec![sys("Setting up voice...")];
            match hydra_voice::setup::download_whisper_binary() {
                Ok(p) => items.push(sys(&format!("Binary: {}", p.display()))),
                Err(e) => items.push(sys(&format!("Binary: {e}"))),
            }
            match hydra_voice::setup::download_whisper_model() {
                Ok(p) => items.push(sys(&format!("Model: {}", p.display()))),
                Err(e) => items.push(sys(&format!("Model: {e}"))),
            }
            items.push(sys("Done. Ctrl+V to speak."));
            items
        }
        "test" => {
            let engine = hydra_voice::TtsEngine::detect();
            if engine.is_available() {
                hydra_voice::native_tts::speak_async(&engine, "Hydra voice operational.");
                vec![sys("Speaking test phrase...")]
            } else {
                vec![sys(&format!("No TTS engine. {}", engine.install_hint()))]
            }
        }
        _ => vec![sys("Usage: /voice [status | setup | test]")],
    }
}

fn cmd_export(_args: &str, ctx: &CommandContext) -> Vec<StreamItem> {
    if ctx.exchanges.is_empty() {
        return vec![sys("Nothing to export.")];
    }
    let dir = dirs::home_dir().unwrap_or_default().join(".hydra/data");
    let _ = std::fs::create_dir_all(&dir);
    let ts = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let path = dir.join(format!("export-{ts}.md"));
    let mut content = String::from("# Hydra Conversation Export\n\n");
    for (user, assistant) in &ctx.exchanges {
        content.push_str(&format!("## you\n{user}\n\n## hydra\n{assistant}\n\n---\n\n"));
    }
    match std::fs::write(&path, &content) {
        Ok(_) => vec![sys(&format!("Exported {} exchanges to {}", ctx.exchanges.len(), path.display()))],
        Err(e) => vec![sys(&format!("Export failed: {e}"))],
    }
}

fn cmd_browse(args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    if args.is_empty() {
        return vec![sys("Usage: /browse <url>")];
    }
    let raw = args.split_whitespace().next().unwrap_or(args).trim();
    let url = if raw.starts_with("http://") || raw.starts_with("https://") {
        raw.to_string()
    } else {
        format!("https://{raw}")
    };
    match reqwest::blocking::get(&url) {
        Ok(resp) => {
            let status = resp.status();
            let content_type = resp.headers().get("content-type")
                .and_then(|v| v.to_str().ok()).unwrap_or("unknown").to_string();
            match resp.text() {
                Ok(body) => {
                    let text = hydra_browser::PageAnalyzer::extract_text(&body);
                    let preview = if text.len() > 500 { format!("{}...", &text[..500]) } else { text.clone() };
                    let lines: Vec<&str> = preview.lines().filter(|l| !l.trim().is_empty()).take(15).collect();
                    let mut items = vec![
                        sys(&format!("  {url} ({status}, {content_type})")),
                    ];
                    for line in lines { items.push(sys(&format!("  {}", line.trim()))); }
                    items.push(sys(&format!("  ({} chars total)", text.len())));
                    items
                }
                Err(e) => vec![sys(&format!("Read failed: {e}"))],
            }
        }
        Err(e) => vec![sys(&format!("Fetch failed: {e}"))],
    }
}

fn cmd_screenshot(_args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    match hydra_desktop::ScreenCapture::capture_full() {
        Ok((bytes, info)) => {
            let path = dirs::home_dir().unwrap_or_default()
                .join(format!(".hydra/data/screenshot-{}.png", chrono::Utc::now().format("%H%M%S")));
            match std::fs::write(&path, &bytes) {
                Ok(_) => vec![
                    sys(&format!("Screenshot: {}x{} ({}KB)", info.width, info.height, info.bytes_len / 1024)),
                    sys(&format!("Saved to: {}", path.display())),
                ],
                Err(e) => vec![sys(&format!("Screenshot captured but save failed: {e}"))],
            }
        }
        Err(e) => vec![sys(&format!("Screenshot failed: {e}"))],
    }
}

fn cmd_spawn(args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    let args = args.trim();
    if args.is_empty() {
        return vec![sys("Usage: /spawn <task description>")];
    }
    let name = format!("agent-{}", &uuid::Uuid::new_v4().to_string()[..8]);
    let request = hydra_fleet::SpawnRequest {
        name: name.clone(),
        specialization: hydra_fleet::AgentSpecialization::Generalist,
        causal_root: args.to_string(),
        requester_trust_score: 1.0,
        requester_tier: hydra_trust::TrustTier::Platinum,
    };
    match hydra_fleet::check_spawn(&request) {
        Ok(result) if result.permitted => {
            let mut registry = hydra_fleet::FleetRegistry::new();
            match registry.spawn(&name, hydra_fleet::AgentSpecialization::Generalist, args, 1.0, hydra_trust::TrustTier::Platinum) {
                Ok(agent_id) => vec![
                    sys(&format!("Agent {name} spawned (id: {})", &agent_id.to_string()[..8])),
                    sys(&format!("  Task: {args}")),
                ],
                Err(e) => vec![sys(&format!("Spawn failed: {e}"))],
            }
        }
        Ok(result) => {
            let reason = result.rejection_reason.unwrap_or_else(|| "constitutional check failed".into());
            vec![sys(&format!("Spawn denied: {reason}"))]
        }
        Err(e) => vec![sys(&format!("Spawn check failed: {e}"))],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_count() {
        assert_eq!(commands().len(), 7);
    }
}
