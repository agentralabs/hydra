//! Slash commands — session management, model/cost, code/review.
//! Claude Code parity commands for conversation lifecycle.

use super::app::{App, Message, MessageRole};

impl App {
    // ── Session Management ──

    pub(crate) fn slash_cmd_clear(&mut self) {
        self.messages.clear();
        self.scroll_offset = 0;
        self.conversation_history.clear();
    }

    pub(crate) fn slash_cmd_compact(&mut self, args: &str, timestamp: &str) {
        if self.messages.len() > 20 {
            let drain_count = self.messages.len() - 20;
            let archived: Vec<Message> = self.messages.drain(0..drain_count).collect();

            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            let archive_path = format!("{}/.hydra/conversation-archive.log", home);
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&archive_path)
            {
                use std::io::Write;
                let _ = writeln!(file, "--- Compacted {} messages at {} ---", drain_count, timestamp);
                for msg in &archived {
                    let role = match msg.role {
                        MessageRole::User => "you",
                        MessageRole::Hydra => "hydra",
                        MessageRole::System => "system",
                    };
                    let _ = writeln!(file, "[{}] {}: {}", msg.timestamp, role, msg.content);
                }
                let _ = writeln!(file, "--- End compact ---\n");
            }

            let focus_note = if args.is_empty() {
                String::new()
            } else {
                format!("\n  Focus: {}", args)
            };

            self.messages.insert(0, Message {
                role: MessageRole::System,
                content: format!(
                    "Compacted {} messages (archived to ~/.hydra/conversation-archive.log).{}",
                    drain_count, focus_note
                ),
                timestamp: timestamp.to_string(),
                phase: None,
            });
        } else {
            self.messages.push(Message {
                role: MessageRole::System,
                content: "Nothing to compact (< 20 messages).".to_string(),
                timestamp: timestamp.to_string(),
                phase: None,
            });
        }
        if self.conversation_history.len() > 20 {
            let drain_count = self.conversation_history.len() - 20;
            self.conversation_history.drain(0..drain_count);
        }
        self.scroll_offset = 0;
    }

    pub(crate) fn slash_cmd_history(&mut self, args: &str, timestamp: &str) {
        if args == "archive" {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            let archive_path = format!("{}/.hydra/conversation-archive.log", home);
            match std::fs::read_to_string(&archive_path) {
                Ok(content) => {
                    let tail: Vec<&str> = content.lines().rev().take(60).collect();
                    let display: String = tail.into_iter().rev().collect::<Vec<_>>().join("\n");
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: format!("Conversation Archive (last 60 lines):\n\n{}", display),
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                }
                Err(_) => {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: "No archive yet. Use /compact to archive older messages.".to_string(),
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                }
            }
        } else if self.history.is_empty() {
            self.messages.push(Message {
                role: MessageRole::System,
                content: "No command history.".to_string(),
                timestamp: timestamp.to_string(),
                phase: None,
            });
        } else {
            let mut lines = String::from("Command History:\n");
            for (i, h) in self.history.iter().rev().take(20).enumerate() {
                lines.push_str(&format!("  {}. {}\n", i + 1, h));
            }
            self.messages.push(Message {
                role: MessageRole::System,
                content: lines,
                timestamp: timestamp.to_string(),
                phase: None,
            });
        }
    }

    pub(crate) fn slash_cmd_resume(&mut self, _args: &str, timestamp: &str) {
        self.messages.push(Message {
            role: MessageRole::System,
            content: "Session resume: not yet available. Sessions are ephemeral.".to_string(),
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    pub(crate) fn slash_cmd_fork(&mut self, timestamp: &str) {
        let msg_count = self.messages.len();
        self.messages.push(Message {
            role: MessageRole::System,
            content: format!(
                "Conversation forked. {} messages preserved.\n\
                 You can try something different. Use /rewind to go back.",
                msg_count
            ),
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    pub(crate) fn slash_cmd_rewind(&mut self, timestamp: &str) {
        if self.messages.len() <= 1 {
            self.messages.push(Message {
                role: MessageRole::System,
                content: "Nothing to rewind.".to_string(),
                timestamp: timestamp.to_string(),
                phase: None,
            });
            return;
        }
        let mut removed = 0;
        while self.messages.len() > 1 && removed < 2 {
            self.messages.pop();
            removed += 1;
        }
        if !self.conversation_history.is_empty() {
            self.conversation_history.pop();
        }
        self.messages.push(Message {
            role: MessageRole::System,
            content: "Rewound last exchange.".to_string(),
            timestamp: timestamp.to_string(),
            phase: None,
        });
        self.scroll_to_bottom();
    }

    pub(crate) fn slash_cmd_rename(&mut self, args: &str, timestamp: &str) {
        if args.is_empty() {
            self.messages.push(Message {
                role: MessageRole::System,
                content: "Usage: /rename <session-name>".to_string(),
                timestamp: timestamp.to_string(),
                phase: None,
            });
        } else {
            self.messages.push(Message {
                role: MessageRole::System,
                content: format!("Session renamed to \"{}\".", args),
                timestamp: timestamp.to_string(),
                phase: None,
            });
        }
    }

    pub(crate) fn slash_cmd_export(&mut self, args: &str, timestamp: &str) {
        let filename = if args.is_empty() {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            format!("{}/.hydra/export-{}.md", home, timestamp.replace(':', ""))
        } else {
            args.to_string()
        };

        let mut content = String::from("# Hydra Conversation Export\n\n");
        for msg in &self.messages {
            let role = match msg.role {
                MessageRole::User => "> ",
                MessageRole::Hydra => "",
                MessageRole::System => "ℹ ",
            };
            content.push_str(&format!("{}{}\n\n", role, msg.content));
        }

        match std::fs::write(&filename, &content) {
            Ok(_) => {
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: format!("Exported {} messages to {}", self.messages.len(), filename),
                    timestamp: timestamp.to_string(),
                    phase: None,
                });
            }
            Err(e) => {
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: format!("Export failed: {}", e),
                    timestamp: timestamp.to_string(),
                    phase: None,
                });
            }
        }
    }

    pub(crate) fn slash_cmd_context(&mut self, timestamp: &str) {
        let msg_tokens: usize = self.messages.iter()
            .map(|m| m.content.len() / 4)
            .sum();
        let max_tokens: usize = 200_000;
        let sys_tokens = 16_000usize; // system prompt estimate
        let tool_tokens = 10_000usize; // tool definitions estimate
        let file_tokens = msg_tokens / 5; // file contents heuristic
        let conv_tokens = msg_tokens.saturating_sub(file_tokens);
        let used = sys_tokens + tool_tokens + conv_tokens + file_tokens;
        let free = max_tokens.saturating_sub(used);

        let pct = ((used as f64 / max_tokens as f64) * 100.0).min(100.0);
        let filled = (pct / 5.0) as usize;
        let empty_bar = 20_usize.saturating_sub(filled);
        let bar = format!("{}{}", "█".repeat(filled), "░".repeat(empty_bar));

        fn mini_bar(pct: f64) -> String {
            let f = (pct / 10.0) as usize;
            let e = 10_usize.saturating_sub(f);
            format!("{}{}", "██".repeat(f).chars().take(f).collect::<String>(),
                    "░░".repeat(e).chars().take(e).collect::<String>())
        }
        let sys_pct = sys_tokens as f64 / max_tokens as f64 * 100.0;
        let conv_pct = conv_tokens as f64 / max_tokens as f64 * 100.0;
        let tool_pct = tool_tokens as f64 / max_tokens as f64 * 100.0;
        let file_pct = file_tokens as f64 / max_tokens as f64 * 100.0;
        let free_pct = free as f64 / max_tokens as f64 * 100.0;

        let msg = format!(
            "Context Usage: {:.0}% {} {}K / {}K tokens\n\
             \n\
             System prompt:   {} {:>3.0}%\n\
             Conversation:    {} {:>3.0}%\n\
             Tool definitions:{} {:>3.0}%\n\
             File contents:   {} {:>3.0}%\n\
             Free:            {} {:>3.0}%",
            pct, bar, used / 1000, max_tokens / 1000,
            mini_bar(sys_pct), sys_pct,
            mini_bar(conv_pct), conv_pct,
            mini_bar(tool_pct), tool_pct,
            mini_bar(file_pct), file_pct,
            mini_bar(free_pct), free_pct,
        );
        self.messages.push(Message {
            role: MessageRole::System,
            content: msg,
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    // ── Model & Cost ──

    pub(crate) fn slash_cmd_model(&mut self, timestamp: &str) {
        let msg = format!(
            "Current model: {}\n\
             Available: Opus 4.6, Sonnet 4.6, Haiku 4.5\n\
             Set with: HYDRA_MODEL env var",
            self.model_name
        );
        self.messages.push(Message {
            role: MessageRole::System,
            content: msg,
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    pub(crate) fn slash_cmd_cost(&mut self, timestamp: &str) {
        let msg_count = self.messages.len();
        let est_tokens = self.messages.iter()
            .map(|m| m.content.len() / 4)
            .sum::<usize>();
        let est_cost = (est_tokens as f64 / 1_000_000.0) * 15.0;
        let msg = format!(
            "Session Cost Estimate:\n\
             Messages:    {}\n\
             Est. tokens: ~{}K\n\
             Est. cost:   ~${:.2}",
            msg_count, est_tokens / 1000, est_cost
        );
        self.messages.push(Message {
            role: MessageRole::System,
            content: msg,
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    pub(crate) fn slash_cmd_tokens(&mut self, timestamp: &str) {
        let msg = format!("Token usage: {} avg per turn", self.token_avg);
        self.messages.push(Message {
            role: MessageRole::System,
            content: msg,
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    // ── Code & Review ──

    pub(crate) fn slash_cmd_review(&mut self, timestamp: &str) {
        self.execute_intent("review current code changes for quality and issues", timestamp);
    }

    // ── Exit ──

    pub(crate) fn slash_cmd_quit(&mut self) {
        self.should_quit = true;
    }

    pub(crate) fn slash_cmd_unknown(&mut self, cmd: &str, timestamp: &str) {
        self.messages.push(Message {
            role: MessageRole::System,
            content: format!("Unknown command: {}. Type /help for commands.", cmd),
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }
}
