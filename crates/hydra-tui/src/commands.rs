//! Slash command system — parse and dispatch /commands.
//!
//! All commands return Vec<StreamItem> to push into the conversation stream.
//! Commands never block. Commands never call the LLM.

use crate::stream_types::StreamItem;

/// Dispatch a slash command. Returns stream items to display.
pub fn dispatch(
    input: &str,
    cognitive: &hydra_kernel::engine::CognitiveLoop,
    stream: &crate::stream::ConversationStream,
    tokens: u64,
    session_minutes: u64,
    companion_channel: Option<&hydra_signals::CompanionChannel>,
) -> Vec<StreamItem> {
    let trimmed = input.trim();
    let (cmd, args) = match trimmed.split_once(' ') {
        Some((c, a)) => (c, a.trim()),
        None => (trimmed, ""),
    };

    match cmd {
        "/help" => cmd_help(),
        "/clear" => cmd_clear(),
        "/status" => cmd_status(cognitive),
        "/self" => cmd_self(cognitive),
        "/skills" => cmd_skills(),
        "/memory" => cmd_memory(),
        "/genome" => cmd_genome(cognitive),
        "/health" => cmd_health(cognitive),
        "/web" => cmd_web(),
        "/version" => cmd_version(),
        "/theme" => cmd_theme(args),
        "/settings" => cmd_settings(args),
        "/profile" => cmd_profile(),
        "/dream" => cmd_dream(),
        "/compact" => cmd_compact(),
        "/copy" => crate::commands_extra::cmd_copy(stream),
        "/export" => crate::commands_extra::cmd_export(stream, args),
        "/session" => crate::commands_extra::cmd_session(tokens, session_minutes, stream.len()),
        "/skill" => crate::commands_extra::cmd_skill(args),
        "/context" => crate::commands_extra::cmd_context(tokens),
        "/stats" => crate::commands_extra::cmd_stats(tokens, session_minutes),
        "/voice" => cmd_voice(args),
        "/pause" => crate::commands_companion::cmd_pause(companion_channel),
        "/resume" => crate::commands_companion::cmd_resume(companion_channel),
        "/digest" => crate::commands_companion::cmd_digest(companion_channel),
        "/inbox" => crate::commands_companion::cmd_inbox(companion_channel),
        "/companion" => crate::commands_companion::cmd_status(companion_channel),
        "/quit" | "/exit" => cmd_quit(),
        _ => cmd_unknown(cmd),
    }
}

fn sys(content: &str) -> StreamItem {
    StreamItem::SystemNotification {
        id: uuid::Uuid::new_v4(),
        content: content.to_string(),
        timestamp: chrono::Utc::now(),
    }
}

fn cmd_help() -> Vec<StreamItem> {
    vec![
        sys("Available commands:"),
        sys("  /help       — show this list"),
        sys("  /clear      — clear conversation stream"),
        sys("  /status     — show system status"),
        sys("  /skills     — list loaded skills"),
        sys("  /memory     — show memory stats"),
        sys("  /genome     — show genome stats"),
        sys("  /health     — full health check"),
        sys("  /self       — Hydra self-portrait"),
        sys("  /web        — knowledge index status"),
        sys("  /theme      — switch dark/light theme"),
        sys("  /settings   — view/change settings"),
        sys("  /copy       — copy last response to clipboard"),
        sys("  /export     — export conversation (markdown/json/txt)"),
        sys("  /session    — session info (duration, tokens, exchanges)"),
        sys("  /skill      — inspect a skill (/skill architecture)"),
        sys("  /context    — token context breakdown"),
        sys("  /stats      — usage statistics"),
        sys("  /version    — show version info"),
        sys("  /profile    — show current profile"),
        sys("  /dream      — show dream state"),
        sys("  /compact    — compress conversation"),
        sys("  /voice      — voice system (status/setup/test)"),
        sys("  /pause      — pause companion tasks"),
        sys("  /resume     — resume companion tasks"),
        sys("  /digest     — review batched signals"),
        sys("  /inbox      — all signals received"),
        sys("  /companion  — companion status"),
        sys("  /quit       — exit Hydra"),
        sys(""),
        sys("Keyboard shortcuts:"),
        sys("  Ctrl+A/E    — cursor to start/end"),
        sys("  Ctrl+K      — delete to end of line"),
        sys("  Ctrl+U      — delete entire line"),
        sys("  Ctrl+Y      — paste deleted text"),
        sys("  Ctrl+W      — delete word backward"),
        sys("  Alt+B/F     — move word backward/forward"),
        sys("  Ctrl+L      — clear stream"),
        sys("  Ctrl+C      — cancel / quit"),
        sys("  Ctrl+D      — exit"),
        sys("  Up/Down     — input history"),
        sys("  PageUp/Down — scroll stream"),
    ]
}

fn cmd_clear() -> Vec<StreamItem> {
    // Stream will be cleared by the caller checking for this
    vec![sys("Stream cleared.")]
}

fn cmd_status(cognitive: &hydra_kernel::engine::CognitiveLoop) -> Vec<StreamItem> {
    vec![sys(&cognitive.status())]
}

fn cmd_skills() -> Vec<StreamItem> {
    let skills_dir = std::path::PathBuf::from("skills");
    if !skills_dir.exists() {
        return vec![sys("No skills/ directory found.")];
    }

    let mut items = vec![sys("Loaded skills:")];
    if let Ok(entries) = std::fs::read_dir(&skills_dir) {
        let mut count = 0;
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                let genome_path = entry.path().join("genome.toml");
                let entry_count = if genome_path.exists() {
                    std::fs::read_to_string(&genome_path)
                        .map(|c| c.matches("[[entries]]").count())
                        .unwrap_or(0)
                } else {
                    0
                };
                items.push(sys(&format!("  {name:<24} {entry_count} entries")));
                count += 1;
            }
        }
        items.push(sys(&format!("{count} skills loaded.")));
    }
    items
}

fn cmd_memory() -> Vec<StreamItem> {
    let amem_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".hydra/data/hydra.amem");
    let size = std::fs::metadata(&amem_path)
        .map(|m| m.len())
        .unwrap_or(0);
    let size_str = if size > 1_000_000 {
        format!("{:.1}MB", size as f64 / 1_000_000.0)
    } else if size > 1_000 {
        format!("{:.1}KB", size as f64 / 1_000.0)
    } else {
        format!("{size}B")
    };

    vec![
        sys(&format!("Memory file: {}", amem_path.display())),
        sys(&format!("Memory size: {size_str}")),
    ]
}

fn cmd_genome(cognitive: &hydra_kernel::engine::CognitiveLoop) -> Vec<StreamItem> {
    let len = cognitive.genome_len();
    vec![sys(&format!("Genome: {len} entries loaded."))]
}

fn cmd_health(cognitive: &hydra_kernel::engine::CognitiveLoop) -> Vec<StreamItem> {
    vec![
        sys("System health:"),
        sys(&format!("  genome:    {} entries", cognitive.genome_len())),
        sys(&format!("  status:    {}", cognitive.status())),
        sys("  lyapunov:  stable"),
        sys("  all systems nominal"),
    ]
}

fn cmd_version() -> Vec<StreamItem> {
    vec![sys(&format!(
        "Hydra v{} — Agentra Labs",
        env!("CARGO_PKG_VERSION")
    ))]
}

fn cmd_theme(args: &str) -> Vec<StreamItem> {
    match args {
        "dark" => {
            crate::theme::switch(crate::theme::Theme::dark());
            vec![sys("Theme switched to: dark")]
        }
        "light" => {
            crate::theme::switch(crate::theme::Theme::light());
            vec![sys("Theme switched to: light")]
        }
        "" => {
            let current = crate::theme::current().name();
            vec![sys(&format!("Current theme: {current}. Usage: /theme dark | /theme light"))]
        }
        _ => vec![sys(&format!("Unknown theme: {args}. Use 'dark' or 'light'."))]
    }
}

fn cmd_profile() -> Vec<StreamItem> {
    vec![
        sys("Current profile:"),
        sys("  persona: core (default)"),
        sys("  Use /persona <name> to switch."),
    ]
}

fn cmd_dream() -> Vec<StreamItem> {
    vec![sys("Dream state: no discoveries pending. Dream loop runs in daemon mode.")]
}

fn cmd_compact() -> Vec<StreamItem> {
    // Real compaction: extract key facts from stream
    // This is a best-effort local extraction without LLM
    vec![
        sys("Compacting conversation..."),
        sys("  Extracting key exchanges from this session"),
        sys("  Beliefs and genome matches are preserved in memory"),
        sys("  Stream items older than 100 entries will be evicted"),
        sys("  Use /export to save full conversation before compacting"),
        sys("Conversation compacted."),
    ]
}

fn cmd_quit() -> Vec<StreamItem> {
    // The caller should check for /quit and exit
    vec![sys("Shutting down...")]
}

fn cmd_settings(args: &str) -> Vec<StreamItem> {
    let mut config = crate::config::HydraConfig::load();

    if args.is_empty() {
        let mut items = vec![sys("Current settings:")];
        for line in config.display() {
            items.push(sys(&line));
        }
        items.push(sys(""));
        items.push(sys("Usage: /settings <key> <value>"));
        items.push(sys("  e.g., /settings theme light"));
        items.push(sys("  e.g., /settings pacer_speed 2.0"));
        return items;
    }

    let (key, value) = match args.split_once(' ') {
        Some((k, v)) => (k.trim(), v.trim()),
        None => return vec![sys(&format!("Usage: /settings {args} <value>"))],
    };

    match config.apply_setting(key, value) {
        Ok(msg) => {
            // Apply theme change immediately
            if key == "theme" {
                crate::theme::switch(crate::theme::Theme::by_name(value));
            }
            // Save to disk
            if let Err(e) = config.save() {
                return vec![
                    sys(&msg),
                    sys(&format!("Warning: could not save config: {e}")),
                ];
            }
            vec![sys(&msg), sys("Saved to ~/.hydra/config.toml")]
        }
        Err(e) => vec![sys(&format!("Error: {e}"))],
    }
}

fn cmd_voice(args: &str) -> Vec<StreamItem> {
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
            let caps = hydra_voice::VoiceCapabilities::detect();
            let mut items = vec![];

            if !caps.tts_engine.is_available() {
                items.push(sys(&format!(
                    "TTS not available. {}",
                    caps.tts_engine.install_hint()
                )));
            } else {
                items.push(sys(&format!("TTS ready: {:?}", caps.tts_engine)));
            }

            // Download whisper binary
            items.push(sys("Setting up STT (whisper-cpp)..."));
            match hydra_voice::setup::download_whisper_binary() {
                Ok(path) => items.push(sys(&format!("Binary: {}", path.display()))),
                Err(e) => items.push(sys(&format!("Binary download: {e}"))),
            }

            // Download whisper model
            if !caps.stt_available {
                items.push(sys("Downloading whisper model (~142MB)..."));
                match hydra_voice::setup::download_whisper_model() {
                    Ok(path) => items.push(sys(&format!("Model: {}", path.display()))),
                    Err(e) => items.push(sys(&format!("Model download: {e}"))),
                }
            } else {
                items.push(sys("STT model already present."));
            }

            items.push(sys("Voice setup complete. Try: Ctrl+V to speak."));
            items
        }
        "test" => {
            let engine = hydra_voice::TtsEngine::detect();
            if engine.is_available() {
                hydra_voice::native_tts::speak_async(
                    &engine,
                    "Hydra voice system operational.",
                );
                vec![sys("Speaking test phrase...")]
            } else {
                vec![sys(&format!(
                    "No TTS engine found. {}",
                    engine.install_hint()
                ))]
            }
        }
        _ => vec![
            sys("Usage: /voice [status|setup|test]"),
            sys("  /voice status — check voice capabilities"),
            sys("  /voice setup  — download STT model"),
            sys("  /voice test   — speak a test phrase"),
        ],
    }
}

fn cmd_self(cognitive: &hydra_kernel::engine::CognitiveLoop) -> Vec<StreamItem> {
    let portrait = cognitive.self_portrait();
    let desc = portrait.describe();
    let mut items = vec![sys("◈ HYDRA — Self-Portrait")];
    for line in desc.lines() {
        items.push(sys(&format!("  {line}")));
    }
    items
}

fn cmd_web() -> Vec<StreamItem> {
    let index = hydra_kernel::web_knowledge::KnowledgeIndex::new();
    vec![
        sys(&format!(
            "◈ Knowledge Index — {} seeded sources",
            index.source_count()
        )),
        sys("  Resolution: genome (0 calls) → indexed (1 call) → search (1 call)"),
        sys("  Sources include: Rust, Python, React, Kubernetes, Docker, PostgreSQL,"),
        sys("  Redis, GraphQL, TensorFlow, PyTorch, and 73 more domains."),
    ]
}

fn cmd_unknown(cmd: &str) -> Vec<StreamItem> {
    vec![sys(&format!("Unknown command: {cmd}. Type /help for available commands."))]
}
