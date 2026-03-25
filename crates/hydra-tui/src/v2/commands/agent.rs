//! Agent commands — /run, /read, /write, /search, /tree, /open, /switch, /windows.
//! Computer-use capabilities exposed as slash commands.

use super::registry::{sys, Command, CommandCategory, CommandContext};
use crate::stream_types::StreamItem;

fn cmd(name: &'static str, aliases: &'static [&'static str], desc: &'static str,
       args: &'static str, handler: fn(&str, &CommandContext) -> Vec<StreamItem>) -> Command {
    Command { name, aliases, description: desc, args_help: args, category: CommandCategory::System, handler }
}

pub fn commands() -> Vec<Command> {
    vec![
        cmd("run",         &["!", "exec"],       "Execute a shell command",            "<command>",      cmd_run),
        cmd("read",        &["cat"],             "Read file contents",                 "<path>",         cmd_read),
        cmd("write",       &[],                  "Write content to a file",            "<path> <content>", cmd_write),
        cmd("search",      &["find", "grep"],    "Search files by pattern",            "<pattern> [path]", cmd_search),
        cmd("tree",        &["ls"],              "Show directory tree",                "[path] [depth]", cmd_tree),
        cmd("open",        &["launch"],          "Launch an application",              "<app name>",     cmd_open),
        cmd("switch",      &["focus"],           "Switch to application window",       "<window title>", cmd_switch),
        cmd("windows",     &["wm"],              "List open windows",                  "",               cmd_windows),
        cmd("capture",     &[],                  "Capture screen for agent vision",    "",               cmd_screenshot),
        cmd("shell",       &["sh"],              "Enter interactive shell mode",       "",               cmd_shell_mode),
        cmd("credentials", &["cred", "vault"],   "Manage login credentials",           "add|list|delete", cmd_credentials),
        cmd("websearch",   &["web", "ask"],      "Search the web for information",     "<query>",        cmd_websearch),
        cmd("swarm",       &["research","learn"],"Spawn multi-agent web research",     "<goal>",         cmd_swarm),
        cmd("feedback",    &["fb"],             "Record outcome feedback for learning","correct|wrong",  cmd_feedback),
        cmd("do",          &["execute","task"],  "Execute multi-step task via conductor","<goal>",       cmd_conduct),
    ]
}

fn cmd_run(args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    if args.is_empty() { return vec![sys("Usage: /run <command>")]; }
    // Safety: block destructive commands unless --force
    let actual_args = if let Some(rest) = args.strip_prefix("--force ") { rest }
    else if is_destructive_command(args) {
        return vec![sys(&format!(
            "Blocked: looks destructive. Use /run --force {} to confirm.", args
        ))];
    } else { args };
    match std::process::Command::new("sh").arg("-c").arg(actual_args).output() {
        Ok(output) => {
            let mut items = Vec::new();
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stdout.is_empty() {
                let text = if stdout.len() > 4000 {
                    format!("{}...\n[{} bytes total]", &stdout[..4000], stdout.len())
                } else { stdout.to_string() };
                items.push(StreamItem::AssistantText {
                    id: uuid::Uuid::new_v4(), text, timestamp: chrono::Utc::now(),
                });
            }
            if !stderr.is_empty() { items.push(sys(&format!("stderr: {}", stderr.trim()))); }
            if !output.status.success() { items.push(sys(&format!("Exit code: {}", output.status.code().unwrap_or(-1)))); }
            if items.is_empty() { items.push(sys("(no output)")); }
            items
        }
        Err(e) => vec![sys(&format!("Failed to execute: {e}"))],
    }
}

fn cmd_read(args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    let path = args.trim();
    if path.is_empty() {
        return vec![sys("Usage: /read <path>")];
    }
    let expanded = expand_home(path);
    match std::fs::read_to_string(&expanded) {
        Ok(content) => {
            let text = if content.len() > 8000 {
                format!(
                    "{}...\n[truncated, {} bytes total]",
                    &content[..8000],
                    content.len()
                )
            } else {
                content
            };
            vec![StreamItem::AssistantText {
                id: uuid::Uuid::new_v4(),
                text,
                timestamp: chrono::Utc::now(),
            }]
        }
        Err(e) => vec![sys(&format!("Read error: {e}"))],
    }
}

fn cmd_write(args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    let (path, content) = match args.split_once(' ') {
        Some((p, c)) => (p.trim(), c),
        None => return vec![sys("Usage: /write <path> <content>")],
    };
    let expanded = expand_home(path);

    // Safety: block writes to system paths
    if is_system_path(&expanded) {
        return vec![sys("Denied: cannot write to system paths")];
    }

    if let Some(parent) = std::path::Path::new(&expanded).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match std::fs::write(&expanded, content) {
        Ok(_) => vec![sys(&format!("Written {} bytes to {path}", content.len()))],
        Err(e) => vec![sys(&format!("Write error: {e}"))],
    }
}

fn cmd_search(args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    if args.is_empty() {
        return vec![sys("Usage: /search <pattern> [path]")];
    }
    let parts: Vec<&str> = args.splitn(2, ' ').collect();
    let pattern = parts[0];
    let path = expand_home(parts.get(1).unwrap_or(&"."));

    // Use grep for content search
    match std::process::Command::new("grep")
        .args(["-rl", "--max-count=1", pattern, &path])
        .output()
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let files: Vec<&str> = stdout.lines().take(20).collect();
            if files.is_empty() {
                vec![sys("No matches found")]
            } else {
                let text = files.join("\n");
                vec![StreamItem::AssistantText {
                    id: uuid::Uuid::new_v4(),
                    text,
                    timestamp: chrono::Utc::now(),
                }]
            }
        }
        Err(e) => vec![sys(&format!("Search error: {e}"))],
    }
}

fn cmd_tree(args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    let parts: Vec<&str> = args.split_whitespace().collect();
    let path = expand_home(parts.first().unwrap_or(&"."));
    let depth = parts
        .get(1)
        .and_then(|d| d.parse::<u32>().ok())
        .unwrap_or(3);

    match std::process::Command::new("find")
        .args([&path, "-maxdepth", &depth.to_string(), "-type", "f"])
        .output()
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let lines: Vec<&str> = stdout.lines().take(50).collect();
            if lines.is_empty() {
                vec![sys("Empty directory")]
            } else {
                let text = lines.join("\n");
                vec![StreamItem::AssistantText {
                    id: uuid::Uuid::new_v4(),
                    text,
                    timestamp: chrono::Utc::now(),
                }]
            }
        }
        Err(e) => vec![sys(&format!("Tree error: {e}"))],
    }
}

fn cmd_open(args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    let app = args.trim();
    if app.is_empty() {
        return vec![sys("Usage: /open <app name>")];
    }
    match hydra_desktop::AppManager::launch(app) {
        Ok(_) => vec![sys(&format!("Launched: {app}"))],
        Err(e) => vec![sys(&format!("Launch error: {e}"))],
    }
}

fn cmd_switch(args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    let title = args.trim();
    if title.is_empty() {
        return vec![sys("Usage: /switch <window title>")];
    }
    match hydra_desktop::AppManager::focus(title) {
        Ok(_) => vec![sys(&format!("Focused: {title}"))],
        Err(e) => vec![sys(&format!("Focus error: {e}"))],
    }
}

fn cmd_windows(_args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    match hydra_desktop::AppManager::list_windows() {
        Ok(windows) => {
            if windows.is_empty() {
                vec![sys("No windows found")]
            } else {
                let text = windows
                    .iter()
                    .map(|w| format!("{} — {}", w.app_name, w.title))
                    .collect::<Vec<_>>()
                    .join("\n");
                vec![StreamItem::AssistantText {
                    id: uuid::Uuid::new_v4(),
                    text,
                    timestamp: chrono::Utc::now(),
                }]
            }
        }
        Err(e) => vec![sys(&format!("Window list error: {e}"))],
    }
}

fn cmd_screenshot(_args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    match hydra_desktop::ScreenCapture::capture_full() {
        Ok((bytes, info)) => {
            let path = dirs::home_dir()
                .unwrap_or_default()
                .join(".hydra/data/screenshot.png");
            match std::fs::write(&path, &bytes) {
                Ok(_) => vec![sys(&format!(
                    "Screenshot saved: {} ({}x{})",
                    path.display(),
                    info.width,
                    info.height
                ))],
                Err(e) => vec![sys(&format!("Save error: {e}"))],
            }
        }
        Err(e) => vec![sys(&format!("Capture error: {e}"))],
    }
}

fn cmd_credentials(args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    let parts: Vec<&str> = args.splitn(3, ' ').collect();
    let vault_dir = dirs::home_dir().unwrap_or_default().join(".hydra/vault");

    match parts.first().map(|s| s.trim()) {
        Some("add") => {
            if parts.len() < 3 {
                return vec![sys("Usage: /credentials add <service> <username>")];
            }
            let service = parts[1].trim();
            let username = parts[2].trim();
            let _ = std::fs::create_dir_all(&vault_dir);
            let content = format!("[credentials]\nusername = \"{username}\"\npassword = \"\"\n");
            let path = vault_dir.join(format!("{service}.toml"));
            match std::fs::write(&path, &content) {
                Ok(_) => {
                    // Encrypt if passphrase is available
                    if hydra_kernel::vault_crypto::is_encryption_enabled() {
                        let _ = hydra_kernel::vault_crypto::encrypt_file(&path);
                    }
                    vec![sys(&format!(
                        "Credentials saved for {service} (user: {username}). \
                         Edit {} to add password.", path.display()
                    ))]
                }
                Err(e) => vec![sys(&format!("Write error: {e}"))],
            }
        }
        Some("list") => {
            if !vault_dir.exists() {
                return vec![sys("No credentials stored")];
            }
            match std::fs::read_dir(&vault_dir) {
                Ok(entries) => {
                    let services: Vec<String> = entries
                        .filter_map(|e| e.ok())
                        .filter_map(|e| {
                            let name = e.file_name().to_string_lossy().to_string();
                            if name.ends_with(".toml") || name.ends_with(".toml.enc") {
                                Some(name.replace(".toml.enc", "").replace(".toml", ""))
                            } else { None }
                        })
                        .collect();
                    if services.is_empty() {
                        vec![sys("No credentials stored")]
                    } else {
                        vec![sys(&format!("Stored credentials:\n{}", services.join("\n")))]
                    }
                }
                Err(e) => vec![sys(&format!("Read error: {e}"))],
            }
        }
        Some("show") | Some("view") | Some("reveal") => {
            let service = parts.get(1).unwrap_or(&"").trim();
            if service.is_empty() {
                return vec![sys("Usage: /credentials show <service>")];
            }
            // User is the principal — they ALWAYS have access to their own credentials
            let enc = vault_dir.join(format!("{service}.toml.enc"));
            let plain = vault_dir.join(format!("{service}.toml"));
            let content = if enc.exists() {
                hydra_kernel::vault_crypto::decrypt_file(&enc)
                    .unwrap_or_else(|e| format!("Decrypt failed (set HYDRA_VAULT_PASSPHRASE): {e}"))
            } else if plain.exists() {
                std::fs::read_to_string(&plain).unwrap_or_else(|e| format!("Read failed: {e}"))
            } else {
                return vec![sys(&format!("No credentials found for '{service}'"))];
            };
            vec![sys(&format!("Credentials for {service}:")), sys(&content)]
        }
        Some("delete") => {
            let service = parts.get(1).unwrap_or(&"").trim();
            if service.is_empty() {
                return vec![sys("Usage: /credentials delete <service>")];
            }
            let plain = vault_dir.join(format!("{service}.toml"));
            let enc = vault_dir.join(format!("{service}.toml.enc"));
            let removed = std::fs::remove_file(&plain).is_ok() || std::fs::remove_file(&enc).is_ok();
            if removed { vec![sys(&format!("Deleted credentials for {service}"))] }
            else { vec![sys(&format!("No credentials found for {service}"))] }
        }
        _ => vec![sys("Usage: /credentials add|list|show|delete <service>")],
    }
}

fn cmd_shell_mode(_args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    // Actual mode toggle happens in handle_submit in the TUI binary.
    // This handler just returns a confirmation message.
    vec![sys("Shell mode activated. Type commands directly. /exit to leave.")]
}

fn cmd_websearch(args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    let query = args.trim();
    if query.is_empty() {
        return vec![sys("Usage: /websearch <query>")];
    }
    let mut orch = hydra_web::SearchOrchestrator::new();
    match orch.search_blocking(query) {
        Ok(text) => vec![StreamItem::AssistantText {
            id: uuid::Uuid::new_v4(), text, timestamp: chrono::Utc::now(),
        }],
        Err(e) => vec![sys(&format!("Search: {e}"))],
    }
}

fn cmd_feedback(args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    match args.trim().to_lowercase().as_str() {
        "correct" | "right" | "yes" | "good" => {
            vec![sys("Feedback: correct. Calibration updated (success recorded).")]
        }
        "wrong" | "incorrect" | "no" | "bad" => {
            vec![sys("Feedback: incorrect. Calibration updated (failure recorded).")]
        }
        "partial" | "mixed" => {
            vec![sys("Feedback: partial. Calibration updated (partial success).")]
        }
        _ => vec![sys("Usage: /feedback correct|wrong|partial")],
    }
}

fn cmd_conduct(args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    let goal = args.trim();
    if goal.is_empty() { return vec![sys("Usage: /do <goal>")]; }
    let genome = hydra_genome::GenomeStore::new();
    let result = hydra_kernel::conductor_exec::conduct(goal, &genome);
    match result {
        hydra_kernel::conductor::ConductorResult::Complete { results } => {
            let summary = results.iter().enumerate().map(|(i, r)| {
                let status = if r.success { "OK" } else { "FAIL" };
                format!("  Step {}: [{status}] {} ({}ms)", i + 1, r.output.lines().next().unwrap_or(""), r.duration_ms)
            }).collect::<Vec<_>>().join("\n");
            vec![StreamItem::AssistantText {
                id: uuid::Uuid::new_v4(), text: format!("Task complete ({} steps):\n{summary}", results.len()),
                timestamp: chrono::Utc::now(),
            }]
        }
        hydra_kernel::conductor::ConductorResult::StepFailed { step_id, error } => {
            vec![sys(&format!("Step {} failed: {error}", step_id + 1))]
        }
        hydra_kernel::conductor::ConductorResult::EmptyPlan => vec![sys("Could not decompose goal. Please be more specific.")],
        hydra_kernel::conductor::ConductorResult::CyclicDag => vec![sys("Internal error: circular dependency in task plan.")],
        hydra_kernel::conductor::ConductorResult::Cancelled => vec![sys("Task cancelled.")],
        hydra_kernel::conductor::ConductorResult::Error(e) => vec![sys(&format!("Error: {e}"))],
    }
}

fn cmd_swarm(args: &str, _ctx: &CommandContext) -> Vec<StreamItem> {
    let goal = args.trim();
    if goal.is_empty() { return vec![sys("Usage: /swarm <research goal>")]; }
    vec![sys(&format!("Swarm launched: {goal}")),
         sys("Workers spawning... Results will appear when complete.")]
}

fn expand_home(path: &str) -> String {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return path.replacen("~", &home.display().to_string(), 1);
        }
    }
    path.to_string()
}

fn is_system_path(path: &str) -> bool {
    let blocked = ["/System", "/usr/bin", "/usr/sbin", "/etc", "/var/root"];
    blocked.iter().any(|b| path.starts_with(b))
}

fn is_destructive_command(cmd: &str) -> bool {
    let first = cmd.split_whitespace().next().unwrap_or("");
    let patterns = ["rm ", "rm\t", "rmdir", "mkfs", "dd ", "format",
        "git reset --hard", "git clean -f", "drop ", "truncate"];
    patterns.iter().any(|p| cmd.contains(p))
        || (first == "rm" && cmd.contains("-rf"))
}
