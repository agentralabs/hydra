//! Extra slash commands — /copy, /export, /session, /skill, /context, /stats.
//!
//! Split from commands.rs to stay under 400 lines per file.

use crate::stream::ConversationStream;
use crate::stream_types::StreamItem;

fn sys(content: &str) -> StreamItem {
    StreamItem::SystemNotification {
        id: uuid::Uuid::new_v4(),
        content: content.to_string(),
        timestamp: chrono::Utc::now(),
    }
}

/// /copy — copy last assistant response to system clipboard.
pub fn cmd_copy(stream: &ConversationStream) -> Vec<StreamItem> {
    // Find last AssistantText
    let last_response = stream
        .items()
        .iter()
        .rev()
        .find_map(|item| match item {
            StreamItem::AssistantText { text, .. } => Some(text.clone()),
            _ => None,
        });

    match last_response {
        Some(text) => {
            // Try to copy using platform commands
            let copied = copy_to_clipboard(&text);
            if copied {
                let preview: String = text.chars().take(60).collect();
                vec![sys(&format!("Copied to clipboard: \"{preview}...\""))]
            } else {
                vec![sys("Failed to copy — clipboard not available")]
            }
        }
        None => vec![sys("Nothing to copy — no assistant response yet")],
    }
}

/// /export — export conversation to file.
pub fn cmd_export(stream: &ConversationStream, args: &str) -> Vec<StreamItem> {
    let format = if args.is_empty() { "markdown" } else { args };

    let export_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".hydra")
        .join("exports");
    let _ = std::fs::create_dir_all(&export_dir);

    let timestamp = chrono::Local::now().format("%Y-%m-%d-%H%M%S");
    let (ext, content) = match format {
        "json" => ("json", export_json(stream)),
        "txt" => ("txt", export_txt(stream)),
        _ => ("md", export_markdown(stream)),
    };

    let filename = format!("hydra-session-{timestamp}.{ext}");
    let path = export_dir.join(&filename);

    match std::fs::write(&path, content) {
        Ok(()) => vec![sys(&format!("Exported to: {}", path.display()))],
        Err(e) => vec![sys(&format!("Export failed: {e}"))],
    }
}

/// /session — show session information.
pub fn cmd_session(
    tokens: u64,
    session_minutes: u64,
    stream_len: usize,
) -> Vec<StreamItem> {
    let exchanges = stream_len / 3; // rough: user + response + receipt per exchange
    vec![
        sys("Session info:"),
        sys(&format!("  Duration:   {}m", session_minutes)),
        sys(&format!("  Exchanges:  ~{exchanges}")),
        sys(&format!("  Tokens:     {tokens}")),
        sys(&format!("  Stream:     {stream_len} items")),
    ]
}

/// /skill <name> — inspect a specific skill's genome entries.
pub fn cmd_skill(args: &str) -> Vec<StreamItem> {
    if args.is_empty() {
        return vec![sys("Usage: /skill <name> (e.g., /skill architecture)")];
    }

    let skill_dir = std::path::PathBuf::from("skills").join(args);
    let genome_path = skill_dir.join("genome.toml");

    if !genome_path.exists() {
        return vec![sys(&format!("Skill not found: {args}"))];
    }

    let content = match std::fs::read_to_string(&genome_path) {
        Ok(c) => c,
        Err(e) => return vec![sys(&format!("Failed to read skill: {e}"))],
    };

    let mut items = vec![sys(&format!("Skill: {args}"))];
    let mut entry_count = 0;

    for line in content.lines() {
        if line.starts_with("situation") {
            let situation = line
                .split_once('=')
                .map(|(_, v)| v.trim().trim_matches('"'))
                .unwrap_or("");
            let short: String = situation.chars().take(70).collect();
            items.push(sys(&format!("  • {short}")));
            entry_count += 1;
        }
    }

    items.push(sys(&format!("{entry_count} entries in {args}")));
    items
}

/// /context — show token context breakdown.
pub fn cmd_context(tokens: u64) -> Vec<StreamItem> {
    // Estimate based on known model limits
    let model_limit: u64 = 200_000;
    let used_pct = (tokens as f64 / model_limit as f64 * 100.0) as u64;
    let free = model_limit.saturating_sub(tokens);
    let bar_len = 20;
    let filled = (used_pct as usize * bar_len / 100).min(bar_len);
    let bar: String = "█".repeat(filled) + &"░".repeat(bar_len - filled);

    vec![
        sys(&format!("Context: {tokens}/{model_limit} tokens ({used_pct}%)")),
        sys(&format!("  [{bar}]")),
        sys(&format!("  Messages:    ~{tokens} tokens")),
        sys(&format!("  Free:        ~{free} tokens")),
    ]
}

/// /stats — session history and usage summary.
pub fn cmd_stats(tokens: u64, session_minutes: u64) -> Vec<StreamItem> {
    let genome_count = hydra_genome::GenomeStore::open().len();
    let audit_count = hydra_audit::AuditEngine::open().record_count();

    vec![
        sys("◈ HYDRA STATS"),
        sys(&format!("  Genome:      {} proven approaches", genome_count)),
        sys(&format!("  Audit:       {} records", audit_count)),
        sys(&format!("  Session:     {}m, {} tokens", session_minutes, tokens)),
        sys(&format!(
            "  Memory:      {}",
            if dirs::home_dir()
                .unwrap_or_default()
                .join(".hydra/data/hydra.amem")
                .exists()
            {
                "persistent (.amem loaded)"
            } else {
                "none"
            }
        )),
    ]
}

// --- Helpers ---

fn copy_to_clipboard(text: &str) -> bool {
    // macOS: pbcopy, Linux: xclip or xsel
    let result = if cfg!(target_os = "macos") {
        std::process::Command::new("pbcopy")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                if let Some(stdin) = child.stdin.as_mut() {
                    stdin.write_all(text.as_bytes())?;
                }
                child.wait()
            })
    } else {
        std::process::Command::new("xclip")
            .arg("-selection")
            .arg("clipboard")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                if let Some(stdin) = child.stdin.as_mut() {
                    stdin.write_all(text.as_bytes())?;
                }
                child.wait()
            })
    };

    result.map(|s| s.success()).unwrap_or(false)
}

fn export_markdown(stream: &ConversationStream) -> String {
    let mut out = String::from("# Hydra Conversation Export\n\n");
    for item in stream.items() {
        match item {
            StreamItem::UserMessage { text, .. } => {
                out.push_str(&format!("**You:** {text}\n\n"));
            }
            StreamItem::AssistantText { text, .. } => {
                out.push_str(&format!("{text}\n\n"));
            }
            StreamItem::SystemNotification { content, .. } => {
                out.push_str(&format!("_{content}_\n\n"));
            }
            StreamItem::ToolDot { tool_name, .. } => {
                out.push_str(&format!("• {tool_name}\n"));
            }
            _ => {}
        }
    }
    out
}

fn export_json(stream: &ConversationStream) -> String {
    // StreamItem doesn't derive Serialize — use text export with JSON structure
    let mut entries = Vec::new();
    for item in stream.items() {
        match item {
            StreamItem::UserMessage { text, timestamp, .. } => {
                entries.push(format!(
                    "  {{\"role\": \"user\", \"text\": {:?}, \"timestamp\": \"{}\"}}",
                    text, timestamp
                ));
            }
            StreamItem::AssistantText { text, timestamp, .. } => {
                entries.push(format!(
                    "  {{\"role\": \"assistant\", \"text\": {:?}, \"timestamp\": \"{}\"}}",
                    text, timestamp
                ));
            }
            _ => {}
        }
    }
    format!("[\n{}\n]", entries.join(",\n"))
}

fn export_txt(stream: &ConversationStream) -> String {
    let mut out = String::new();
    for item in stream.items() {
        match item {
            StreamItem::UserMessage { text, .. } => {
                out.push_str(&format!("YOU: {text}\n\n"));
            }
            StreamItem::AssistantText { text, .. } => {
                out.push_str(&format!("HYDRA: {text}\n\n"));
            }
            StreamItem::SystemNotification { content, .. } => {
                out.push_str(&format!("[{content}]\n"));
            }
            _ => {}
        }
    }
    out
}
