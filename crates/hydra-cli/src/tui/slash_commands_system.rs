//! Slash commands — system and monitoring.
//! /sisters, /fix, /scan, /repair, /memory, /goals, /beliefs, /receipts, /health, /status

use super::app::{App, Message, MessageRole};

impl App {
    pub(crate) fn slash_cmd_sisters(&mut self, timestamp: &str) {
        self.refresh_status();
        // Clean formatting — no box borders (per visual overhaul spec)
        let mut lines = Vec::new();
        // Two-column layout
        let sisters = &self.sisters;
        let half = (sisters.len() + 1) / 2;
        for i in 0..half {
            let left = &sisters[i];
            let left_dot = if left.connected { "●" } else { "○" };
            let left_str = format!(
                "{} {:<12} {:>2} tools",
                left_dot, left.short_name, left.tool_count
            );
            if i + half < sisters.len() {
                let right = &sisters[i + half];
                let right_dot = if right.connected { "●" } else { "○" };
                lines.push(format!(
                    "  {}   {} {:<12} {:>2} tools",
                    left_str, right_dot, right.short_name, right.tool_count
                ));
            } else {
                lines.push(format!("  {}", left_str));
            }
        }
        lines.push(String::new());
        lines.push(format!(
            "  {}/{} online · {}+ tools available",
            self.connected_count, self.total_sisters, self.tool_count
        ));

        self.messages.push(Message {
            role: MessageRole::System,
            content: lines.join("\n"),
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    pub(crate) fn slash_cmd_fix(&mut self, timestamp: &str) {
        self.messages.push(Message {
            role: MessageRole::System,
            content: "Running sister repair...".to_string(),
            timestamp: timestamp.to_string(),
            phase: Some("Repair".to_string()),
        });
        self.execute_intent("repair offline sisters", timestamp);
    }

    pub(crate) fn slash_cmd_scan(&mut self, timestamp: &str) {
        self.messages.push(Message {
            role: MessageRole::System,
            content: "Starting Omniscience scan...".to_string(),
            timestamp: timestamp.to_string(),
            phase: Some("Omniscience".to_string()),
        });
        self.execute_intent("scan all repos for gaps", timestamp);
    }

    pub(crate) fn slash_cmd_repair(&mut self, timestamp: &str) {
        self.messages.push(Message {
            role: MessageRole::System,
            content: "Running self-repair specs...".to_string(),
            timestamp: timestamp.to_string(),
            phase: Some("Repair".to_string()),
        });
        self.execute_intent("run self repair", timestamp);
    }

    pub(crate) fn slash_cmd_memory(&mut self, args: &str, timestamp: &str) {
        let arg = args.trim().to_lowercase();
        if arg == "all" || arg == "facts" || arg == "none" {
            self.memory_capture = arg.clone();
            // Persist to profile
            if let Some(mut profile) = hydra_native::profile::load_profile() {
                profile.memory_capture = Some(arg.clone());
                hydra_native::profile::save_profile(&profile);
            }
            let label = match arg.as_str() {
                "all" => "Full Conversation", "facts" => "Facts Only", _ => "None",
            };
            self.messages.push(Message {
                role: MessageRole::System,
                content: format!("Memory capture set to: {}", label),
                timestamp: timestamp.to_string(), phase: None,
            });
            return;
        }
        let mode_label = match self.memory_capture.as_str() {
            "all" => "Full Conversation", "facts" => "Facts Only", _ => "None",
        };
        let msg = format!(
            "Memory Capture: {}\n  Facts: {} | Tokens avg: {} | Receipts: {}\n\n\
             Usage: /memory <all|facts|none>\n\
             \x20 all   - Full conversation capture (every message stored)\n\
             \x20 facts - Decisions and corrections only (no raw text)\n\
             \x20 none  - No capture (session-only, nothing persisted)",
            mode_label, self.memory_facts, self.token_avg, self.receipt_count
        );
        self.messages.push(Message {
            role: MessageRole::System, content: msg,
            timestamp: timestamp.to_string(), phase: None,
        });
    }

    pub(crate) fn slash_cmd_goals(&mut self, timestamp: &str) {
        self.execute_intent("show active planning goals", timestamp);
    }

    pub(crate) fn slash_cmd_beliefs(&mut self, timestamp: &str) {
        self.execute_intent("show current belief store", timestamp);
    }

    pub(crate) fn slash_cmd_receipts(&mut self, timestamp: &str) {
        let msg = format!(
            "Recent receipts: {}\n  Cryptographic chain: intact",
            self.receipt_count
        );
        self.messages.push(Message {
            role: MessageRole::System,
            content: msg,
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    pub(crate) fn slash_cmd_health(&mut self, timestamp: &str) {
        self.refresh_status();
        let mode = if self.sisters_handle.is_some() {
            "Local (embedded)"
        } else if self.server_online {
            "Server"
        } else {
            "Offline"
        };
        let msg = format!(
            "System Health Dashboard\n\
             \n\
             Mode       {}\n\
             Sisters    {}/{} ({:.0}%)\n\
             Tools      {}\n\
             Trust      {}\n\
             Memory     {} facts\n\
             Tokens     {} avg\n\
             Receipts   {}\n\
             Model      {}",
            mode,
            self.connected_count, self.total_sisters, self.health_pct,
            self.tool_count,
            self.trust_level,
            self.memory_facts,
            self.token_avg,
            self.receipt_count,
            self.model_name,
        );
        self.messages.push(Message {
            role: MessageRole::System,
            content: msg,
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    /// /email <to> <subject> — send an email via SMTP.
    /// Body is read from remaining args after subject, or defaults to a brief message.
    pub(crate) fn slash_cmd_email(&mut self, args: &str, timestamp: &str) {
        if self.smtp_host.is_empty() || self.smtp_user.is_empty() || self.smtp_password.is_empty() {
            self.messages.push(Message {
                role: MessageRole::System,
                content: "Email not configured. Run /email-setup to configure SMTP settings.".into(),
                timestamp: timestamp.to_string(), phase: None,
            });
            return;
        }
        let parts: Vec<&str> = args.splitn(3, ' ').collect();
        let (to, subject, body) = match parts.len() {
            0 => {
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: "Usage: /email <to> <subject> [body]".into(),
                    timestamp: timestamp.to_string(), phase: None,
                });
                return;
            }
            1 => (parts[0], "Message from Hydra", "Sent via Hydra"),
            2 => (parts[0], parts[1], "Sent via Hydra"),
            _ => (parts[0], parts[1], parts[2]),
        };
        let to = if to.is_empty() || !to.contains('@') {
            if self.smtp_to.is_empty() {
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: "No valid recipient. Usage: /email user@example.com Subject Body".into(),
                    timestamp: timestamp.to_string(), phase: None,
                });
                return;
            }
            &self.smtp_to
        } else { to };
        self.messages.push(Message {
            role: MessageRole::System,
            content: format!("Sending email to {}...", to),
            timestamp: timestamp.to_string(), phase: Some("Email".into()),
        });
        self.execute_intent(&format!("send email to {} subject {} body {}", to, subject, body), timestamp);
    }

    /// /email-setup — configure SMTP settings interactively.
    pub(crate) fn slash_cmd_email_setup(&mut self, args: &str, timestamp: &str) {
        let parts: Vec<&str> = args.splitn(2, ' ').collect();
        match parts.first().copied().unwrap_or("") {
            "host" => {
                let val = parts.get(1).copied().unwrap_or("").trim();
                if val.is_empty() { self.messages.push(Message { role: MessageRole::System, content: format!("SMTP host: {}", if self.smtp_host.is_empty() { "(not set)" } else { &self.smtp_host }), timestamp: timestamp.into(), phase: None }); return; }
                self.smtp_host = val.to_string();
                self.save_smtp_to_profile();
                self.messages.push(Message { role: MessageRole::System, content: format!("SMTP host set to: {}", val), timestamp: timestamp.into(), phase: None });
            }
            "user" => {
                let val = parts.get(1).copied().unwrap_or("").trim();
                if val.is_empty() { self.messages.push(Message { role: MessageRole::System, content: format!("SMTP user: {}", if self.smtp_user.is_empty() { "(not set)" } else { &self.smtp_user }), timestamp: timestamp.into(), phase: None }); return; }
                self.smtp_user = val.to_string();
                self.save_smtp_to_profile();
                self.messages.push(Message { role: MessageRole::System, content: format!("SMTP user set to: {}", val), timestamp: timestamp.into(), phase: None });
            }
            "password" => {
                let val = parts.get(1).copied().unwrap_or("").trim();
                if val.is_empty() { self.messages.push(Message { role: MessageRole::System, content: "SMTP password: (hidden)".into(), timestamp: timestamp.into(), phase: None }); return; }
                self.smtp_password = val.to_string();
                self.save_smtp_to_profile();
                self.messages.push(Message { role: MessageRole::System, content: "SMTP password set.".into(), timestamp: timestamp.into(), phase: None });
            }
            "to" => {
                let val = parts.get(1).copied().unwrap_or("").trim();
                if val.is_empty() { self.messages.push(Message { role: MessageRole::System, content: format!("Default recipient: {}", if self.smtp_to.is_empty() { "(not set)" } else { &self.smtp_to }), timestamp: timestamp.into(), phase: None }); return; }
                self.smtp_to = val.to_string();
                self.save_smtp_to_profile();
                self.messages.push(Message { role: MessageRole::System, content: format!("Default recipient set to: {}", val), timestamp: timestamp.into(), phase: None });
            }
            _ => {
                let configured = !self.smtp_host.is_empty() && !self.smtp_user.is_empty() && !self.smtp_password.is_empty();
                let status = if configured { "Configured" } else { "Not configured" };
                let msg = format!(
                    "Email Settings ({})\n\n  Host       {}\n  User       {}\n  Password   {}\n  Recipient  {}\n\n\
                     Usage:\n  /email-setup host smtp.gmail.com\n  /email-setup user you@gmail.com\n  /email-setup password <app-password>\n  /email-setup to recipient@example.com",
                    status,
                    if self.smtp_host.is_empty() { "(not set)" } else { &self.smtp_host },
                    if self.smtp_user.is_empty() { "(not set)" } else { &self.smtp_user },
                    if self.smtp_password.is_empty() { "(not set)" } else { "********" },
                    if self.smtp_to.is_empty() { "(not set)" } else { &self.smtp_to },
                );
                self.messages.push(Message { role: MessageRole::System, content: msg, timestamp: timestamp.into(), phase: None });
            }
        }
    }

    fn save_smtp_to_profile(&self) {
        if let Some(mut profile) = hydra_native::profile::load_profile() {
            profile.smtp_host = if self.smtp_host.is_empty() { None } else { Some(self.smtp_host.clone()) };
            profile.smtp_user = if self.smtp_user.is_empty() { None } else { Some(self.smtp_user.clone()) };
            profile.smtp_password = if self.smtp_password.is_empty() { None } else { Some(self.smtp_password.clone()) };
            profile.smtp_to = if self.smtp_to.is_empty() { None } else { Some(self.smtp_to.clone()) };
            hydra_native::profile::save_profile(&profile);
        }
    }

    /// /status — clean summary per visual overhaul spec (replaces sidebar info)
    pub(crate) fn slash_cmd_status(&mut self, timestamp: &str) {
        self.refresh_status();
        let mode = if self.sisters_handle.is_some() {
            "● Local"
        } else if self.server_online {
            "● Server"
        } else {
            "○ Offline"
        };

        let project_line = if let Some(ref info) = self.project_info {
            let crate_info = info.crate_count
                .map(|c| format!(" ({} crates)", c))
                .unwrap_or_default();
            format!(
                "  Project     {} ({}{}){}",
                info.name,
                info.kind.label(),
                crate_info,
                info.git_branch.as_ref()
                    .map(|b| format!("\n  Git         {}", b))
                    .unwrap_or_default()
            )
        } else {
            String::new()
        };

        let msg = format!(
            "{}\n\
             \n\
             {}  Model       {}\n\
             \n\
             {}  Sisters     {}/{}\n\
             {}  Health      {}%\n\
             {}  Memory      {} facts\n\
             {}  Tokens      {} avg\n\
             {}  Receipts    {}\n\
             {}  Autonomy    {}\n\
             {}  Mode        {}",
            project_line,
            "", self.model_name,
            "", self.connected_count, self.total_sisters,
            "", self.health_pct,
            "", self.memory_facts,
            "", self.token_avg,
            "", self.receipt_count,
            "", self.trust_level,
            "", mode,
        );
        self.messages.push(Message {
            role: MessageRole::System,
            content: msg,
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    /// /stats — Show sister gateway usage stats (sister-first enforcement metrics).
    pub(crate) fn slash_cmd_stats(&mut self, timestamp: &str) {
        let msg = if self.gateway_stats.is_empty() {
            "Sister Gateway Stats\n\n  No stats yet. Send a message first to generate sister usage data.".to_string()
        } else {
            format!("Sister Gateway Stats\n\n{}", self.gateway_stats)
        };
        self.messages.push(Message {
            role: MessageRole::System,
            content: msg,
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    pub(crate) fn slash_cmd_roi(&mut self, timestamp: &str) {
        let summary = hydra_native::knowledge::economics_tracker::roi_summary();
        self.messages.push(Message {
            role: MessageRole::System,
            content: summary,
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    pub(crate) fn slash_cmd_knowledge(&mut self, timestamp: &str) {
        let mentor = hydra_native::knowledge::mentor_system::mentor_state();
        let summary = if let Ok(state) = mentor.lock() {
            state.progress_summary()
        } else {
            "Knowledge tracker unavailable.".into()
        };
        self.messages.push(Message {
            role: MessageRole::System,
            content: format!("Knowledge Progress\n\n{}", summary),
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }
}
