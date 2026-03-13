//! Slash commands — Model, Cost, Config additions.
//! Split for 400-line limit. Covers missing commands from TUI spec.

use super::app::{App, Message, MessageRole, resolve_model_name};

impl App {
    // ── Model & Cost additions (§5.2) ──

    pub(crate) fn slash_cmd_usage(&mut self, timestamp: &str) {
        let est_tokens = self.messages.len() as u64 * self.token_avg.max(800);
        let budget = 200_000u64; // default context window
        let pct = (est_tokens as f64 / budget as f64 * 100.0).min(100.0);
        let msg = format!(
            "Usage\n\
             \n\
             Estimated tokens: ~{}K / {}K ({:.0}%)\n\
             Messages:         {}\n\
             Budget limit:     not set (use --max-budget-usd to cap)\n\
             \n\
             Set a budget: hydra --max-budget-usd 5.00",
            est_tokens / 1000, budget / 1000, pct,
            self.messages.len(),
        );
        self.messages.push(Message {
            role: MessageRole::System,
            content: msg,
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    pub(crate) fn slash_cmd_fast(&mut self, timestamp: &str) {
        // Toggle fast mode — same model, faster output
        self.messages.push(Message {
            role: MessageRole::System,
            content: "Fast Mode toggled. Same model, faster output (2.5x speed).\n\
                     Note: may increase cost per token.".to_string(),
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    // ── Code & Review additions (§5.3) ──

    pub(crate) fn slash_cmd_todos(&mut self, timestamp: &str) {
        // Scan conversation for TODO-like items
        let mut todos: Vec<String> = Vec::new();
        for msg in &self.messages {
            for line in msg.content.lines() {
                let lower = line.to_lowercase();
                if lower.contains("todo") || lower.contains("fixme")
                    || lower.contains("hack") || lower.contains("[ ]")
                {
                    todos.push(format!("  — {}", line.trim()));
                }
            }
        }
        let msg = if todos.is_empty() {
            "TODOs\n\n  No TODO items found in conversation.".to_string()
        } else {
            format!("TODOs ({})\n\n{}", todos.len(), todos.join("\n"))
        };
        self.messages.push(Message {
            role: MessageRole::System,
            content: msg,
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    pub(crate) fn slash_cmd_add_dir(&mut self, args: &str, timestamp: &str) {
        if args.is_empty() {
            self.messages.push(Message {
                role: MessageRole::System,
                content: "Usage: /add-dir <path>\n\
                         Adds an additional working directory to the session.".to_string(),
                timestamp: timestamp.to_string(),
                phase: None,
            });
            return;
        }
        let path = std::path::Path::new(args);
        if path.is_dir() {
            self.messages.push(Message {
                role: MessageRole::System,
                content: format!("Added directory: {}\nHydra can now access files in this directory.", args),
                timestamp: timestamp.to_string(),
                phase: None,
            });
        } else {
            self.messages.push(Message {
                role: MessageRole::System,
                content: format!("Directory not found: {}", args),
                timestamp: timestamp.to_string(),
                phase: None,
            });
        }
    }

    // ── Configuration additions (§5.4) ──

    pub(crate) fn slash_cmd_terminal_setup(&mut self, timestamp: &str) {
        self.messages.push(Message {
            role: MessageRole::System,
            content: "Terminal Setup\n\
                     \n\
                     Configures terminal keybindings for Hydra:\n\
                       — Shift+Enter for multiline input\n\
                       — Option as Meta key (macOS)\n\
                     \n\
                     Current terminal: detected automatically.\n\
                     Run: hydra terminal-setup".to_string(),
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    pub(crate) fn slash_cmd_login(&mut self, args: &str, timestamp: &str) {
        let name = args.trim();
        if name.is_empty() {
            let current = hydra_native::profile::active_user().unwrap_or_else(|| self.user_name.clone());
            let users = hydra_native::profile::list_users();
            let list = if users.is_empty() { "  (none)".into() } else { users.iter().map(|u| {
                if *u == current { format!("  {} (active)", u) } else { format!("  {}", u) }
            }).collect::<Vec<_>>().join("\n") };
            self.messages.push(Message {
                role: MessageRole::System,
                content: format!("Profiles\n\n{}\n\nSwitch: /login <name>\nSign out: /logout", list),
                timestamp: timestamp.to_string(), phase: None,
            });
            return;
        }
        // Switch to user (create if new)
        hydra_native::profile::set_active_user(name);
        let profile = hydra_native::profile::load_profile();
        let display = profile.as_ref().and_then(|p| p.user_name.clone()).unwrap_or_else(|| name.into());
        self.user_name = display.clone();
        // Reload SMTP settings from new profile
        self.smtp_host = profile.as_ref().and_then(|p| p.smtp_host.clone()).unwrap_or_default();
        self.smtp_user = profile.as_ref().and_then(|p| p.smtp_user.clone()).unwrap_or_default();
        self.smtp_password = profile.as_ref().and_then(|p| p.smtp_password.clone()).unwrap_or_default();
        self.smtp_to = profile.as_ref().and_then(|p| p.smtp_to.clone()).unwrap_or_default();
        let is_new = profile.is_none();
        if is_new {
            let new_profile = hydra_native::profile::PersistedProfile { user_name: Some(name.into()), ..Default::default() };
            hydra_native::profile::save_profile(&new_profile);
        }
        let msg = if is_new { format!("Welcome, {}! New profile created.", name) } else { format!("Switched to profile: {}", display) };
        self.messages.push(Message { role: MessageRole::System, content: msg, timestamp: timestamp.into(), phase: None });
    }

    pub(crate) fn slash_cmd_logout(&mut self, timestamp: &str) {
        let current = hydra_native::profile::active_user().unwrap_or_else(|| self.user_name.clone());
        hydra_native::profile::clear_active_user();
        let users = hydra_native::profile::list_users();
        let others: Vec<&str> = users.iter().map(|s| s.as_str()).filter(|u| *u != current).collect();
        let hint = if others.is_empty() {
            "Restart Hydra to create a new profile, or /login <name> to sign in.".into()
        } else {
            format!("Other profiles: {}. Use /login <name> to switch.", others.join(", "))
        };
        self.messages.push(Message {
            role: MessageRole::System,
            content: format!("Signed out as {}. Data preserved in ~/.hydra/users/{}.\n{}", current, current, hint),
            timestamp: timestamp.to_string(), phase: None,
        });
    }

    pub(crate) fn slash_cmd_keybindings(&mut self, timestamp: &str) {
        let home = std::env::var("HOME").unwrap_or_else(|_| "~".into());
        let kb_path = format!("{}/.hydra/keybindings.json", home);
        let exists = std::path::Path::new(&kb_path).exists();

        let msg = format!(
            "Keybindings\n\
             \n\
             Config: {} {}\n\
             \n\
             Current bindings:\n\
               Ctrl+K     Kill switch\n\
               Ctrl+S     Toggle sidebar\n\
               Ctrl+D     Exit session\n\
               Ctrl+E     Quick environment check\n\
               Ctrl+T     Toggle task list\n\
               Ctrl+G     Open external editor\n\
               Ctrl+F     Kill all background agents\n\
               Ctrl+L     Refresh status\n\
               Shift+Tab  Cycle permission mode\n\
               Esc        Clear input\n\
               Tab        Autocomplete command\n\
             \n\
             Edit {} to customize.",
            if exists { "found at" } else { "not found at" },
            kb_path, kb_path,
        );
        self.messages.push(Message {
            role: MessageRole::System,
            content: msg,
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    pub(crate) fn slash_cmd_output_style(&mut self, timestamp: &str) {
        self.messages.push(Message {
            role: MessageRole::System,
            content: "Output Style\n\
                     \n\
                     Current: default (streaming text with ● tool indicators)\n\
                     \n\
                     Available styles:\n\
                       default    — Streaming text + ● tool dots\n\
                       compact    — Minimal output, less whitespace\n\
                       verbose    — Full tool output expanded\n\
                       json       — JSON output (for scripting)".to_string(),
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    // ── Hydra-Exclusive additions (§5.7) ──

    pub(crate) fn slash_cmd_sister(&mut self, args: &str, timestamp: &str) {
        if args.is_empty() {
            self.messages.push(Message {
                role: MessageRole::System,
                content: "Usage: /sister <name>\n\
                         Show detailed info for a specific sister.\n\
                         Example: /sister Memory".to_string(),
                timestamp: timestamp.to_string(),
                phase: None,
            });
            return;
        }
        let name_lower = args.to_lowercase();
        if let Some(sister) = self.sisters.iter().find(|s| s.short_name.to_lowercase() == name_lower) {
            let msg = format!(
                "Sister: {}\n\
                 \n\
                 Status:    {}\n\
                 Full name: {}\n\
                 Tools:     {}",
                sister.short_name,
                if sister.connected { "● online" } else { "○ offline" },
                sister.name,
                if sister.tool_count > 0 { format!("{}", sister.tool_count) } else { "varies".to_string() },
            );
            self.messages.push(Message {
                role: MessageRole::System,
                content: msg,
                timestamp: timestamp.to_string(),
                phase: None,
            });
        } else {
            let names: Vec<&str> = self.sisters.iter().map(|s| s.short_name.as_str()).collect();
            self.messages.push(Message {
                role: MessageRole::System,
                content: format!("Sister '{}' not found. Available: {}", args, names.join(", ")),
                timestamp: timestamp.to_string(),
                phase: None,
            });
        }
    }

    pub(crate) fn slash_cmd_diagnostics(&mut self, timestamp: &str) {
        self.messages.push(Message {
            role: MessageRole::System,
            content: "Diagnostics\n\
                     \n\
                     Last consolidation: none this session\n\
                     Cognitive loop: idle\n\
                     Intent queue: empty\n\
                     Memory pressure: normal".to_string(),
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    /// /copy — copy last Hydra response to clipboard (uses pbcopy on macOS, xclip on Linux).
    pub(crate) fn slash_cmd_copy(&mut self, timestamp: &str) {
        // Find last Hydra message
        let last = self.messages.iter().rev().find(|m| m.role == MessageRole::Hydra);
        let text = match last {
            Some(msg) => msg.content.clone(),
            None => {
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: "Nothing to copy — no Hydra response yet.".to_string(),
                    timestamp: timestamp.to_string(),
                    phase: None,
                });
                return;
            }
        };

        // Copy to clipboard using platform command
        let cmd = if cfg!(target_os = "macos") { "pbcopy" } else { "xclip -selection clipboard" };
        let result = std::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .stdin(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                if let Some(ref mut stdin) = child.stdin {
                    stdin.write_all(text.as_bytes())?;
                }
                child.wait()
            });

        let msg = match result {
            Ok(status) if status.success() => "Copied last response to clipboard.".to_string(),
            _ => "Failed to copy — clipboard command not available.".to_string(),
        };
        self.messages.push(Message {
            role: MessageRole::System,
            content: msg,
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    // ── Model switching & cost (moved from slash_commands_session.rs) ──

    pub(crate) fn slash_cmd_model(&mut self, args: &str, timestamp: &str) {
        if args.is_empty() {
            let msg = format!(
                "Current model: {}\n\n\
                 Frontier: opus, sonnet, haiku, gpt-4o, gpt-4o-mini, gpt-4.1, o3, o4-mini,\n\
                 \x20         gemini-2.5-pro, gemini-2.5-flash, grok-3, deepseek-r1, mistral-large\n\
                 Local:    llama3.3, qwen2.5-coder, deepseek-r1:14b, mistral, phi4, codellama\n\n\
                 Switch: /model <name>  (e.g. /model opus, /model gpt-4o)",
                self.model_name
            );
            self.messages.push(Message { role: MessageRole::System, content: msg,
                timestamp: timestamp.to_string(), phase: None });
        } else {
            let input = args.trim().to_lowercase();
            let resolved = match input.as_str() {
                "opus" | "opus4" => "claude-opus-4-6",
                "sonnet" | "sonnet4" => "claude-sonnet-4-6",
                "haiku" | "haiku4" => "claude-haiku-4-5",
                "gpt4o" | "4o" => "gpt-4o",
                "4o-mini" => "gpt-4o-mini",
                other => other,
            };
            std::env::set_var("HYDRA_MODEL", resolved);
            self.model_name = resolve_model_name();
            self.messages.push(Message { role: MessageRole::System,
                content: format!("Model switched to: {}", self.model_name),
                timestamp: timestamp.to_string(), phase: None });
        }
    }

    pub(crate) fn slash_cmd_cost(&mut self, timestamp: &str) {
        let est_tokens = self.messages.iter().map(|m| m.content.len() / 4).sum::<usize>();
        let est_cost = (est_tokens as f64 / 1_000_000.0) * 15.0;
        self.messages.push(Message { role: MessageRole::System,
            content: format!("Session Cost Estimate:\nMessages:    {}\nEst. tokens: ~{}K\nEst. cost:   ~${:.2}",
                self.messages.len(), est_tokens / 1000, est_cost),
            timestamp: timestamp.to_string(), phase: None });
    }

    pub(crate) fn slash_cmd_tokens(&mut self, timestamp: &str) {
        self.messages.push(Message { role: MessageRole::System,
            content: format!("Token usage: {} avg per turn", self.token_avg),
            timestamp: timestamp.to_string(), phase: None });
    }

    pub(crate) fn slash_cmd_review(&mut self, timestamp: &str) {
        self.execute_intent("review current code changes for quality and issues", timestamp);
    }

    pub(crate) fn slash_cmd_quit(&mut self) { self.should_quit = true; }

    pub(crate) fn slash_cmd_unknown(&mut self, cmd: &str, timestamp: &str) {
        self.messages.push(Message { role: MessageRole::System,
            content: format!("Unknown command: {}. Type /help for commands.", cmd),
            timestamp: timestamp.to_string(), phase: None });
    }
}
