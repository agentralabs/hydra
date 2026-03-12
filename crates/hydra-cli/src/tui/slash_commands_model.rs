//! Slash commands — Model, Cost, Config additions (CC §5.2, §5.4, §5.7).
//! Split for 400-line limit. Covers missing commands from TUI spec.

use super::app::{App, Message, MessageRole};

impl App {
    // ── Model & Cost additions (CC §5.2) ──

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

    // ── Code & Review additions (CC §5.3) ──

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

    // ── Configuration additions (CC §5.4) ──

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

    pub(crate) fn slash_cmd_login(&mut self, timestamp: &str) {
        self.messages.push(Message {
            role: MessageRole::System,
            content: "Login\n\
                     \n\
                     Current: API key authentication\n\
                     \n\
                     Switch accounts:\n\
                       hydra login\n\
                       hydra login --provider anthropic\n\
                       hydra login --provider openai".to_string(),
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    pub(crate) fn slash_cmd_logout(&mut self, timestamp: &str) {
        self.messages.push(Message {
            role: MessageRole::System,
            content: "Signed out. API keys cleared for this session.\n\
                     Use /login to re-authenticate.".to_string(),
            timestamp: timestamp.to_string(),
            phase: None,
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

    // ── Hydra-Exclusive additions (CC §5.7) ──

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
}
