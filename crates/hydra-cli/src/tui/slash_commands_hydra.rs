//! Slash commands — Hydra-exclusive, config, control, debug, help.
//! Split from slash_commands_session.rs for 400-line limit.

use super::app::{App, Message, MessageRole};

impl App {
    // ── Config ──
    // /voice → implemented in voice.rs (mic capture + Whisper STT)

    pub(crate) fn slash_cmd_sidebar(&mut self) {
        self.sidebar_visible = !self.sidebar_visible;
    }

    pub(crate) fn slash_cmd_theme(&mut self, timestamp: &str) {
        self.messages.push(Message {
            role: MessageRole::System,
            content: "Theme: Hydra Dark (default). More themes coming soon.".to_string(),
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    pub(crate) fn slash_cmd_config(&mut self, timestamp: &str) {
        let msg = format!(
            "Configuration\n\
             \n\
             Model        {}\n\
             Sidebar      {}\n\
             Permission   {}\n\
             Trust        {}\n\
             Working dir  {}\n\
             User         {}",
            self.model_name,
            if self.sidebar_visible { "visible" } else { "hidden" },
            self.permission_mode.label(),
            self.trust_level,
            self.working_dir,
            self.user_name,
        );
        self.messages.push(Message {
            role: MessageRole::System,
            content: msg,
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    pub(crate) fn slash_cmd_doctor(&mut self, timestamp: &str) {
        self.refresh_status();
        let api_ok = !std::env::var("ANTHROPIC_API_KEY").unwrap_or_default().is_empty();
        let server_ok = self.server_online;
        let sisters_ok = self.connected_count > 0;

        let msg = format!(
            "Doctor — Health Check\n\
             \n\
             {} API key       {}\n\
             {} Server        {}\n\
             {} Sisters       {}/{}\n\
             {} Database      {}",
            if api_ok { "✓" } else { "✗" },
            if api_ok { "configured" } else { "MISSING — set ANTHROPIC_API_KEY" },
            if server_ok { "✓" } else { "✗" },
            if server_ok { "reachable" } else { "offline" },
            if sisters_ok { "✓" } else { "✗" },
            self.connected_count, self.total_sisters,
            if self.db.is_some() { "✓" } else { "✗" },
            if self.db.is_some() { "connected" } else { "unavailable" },
        );
        self.messages.push(Message {
            role: MessageRole::System,
            content: msg,
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    pub(crate) fn slash_cmd_vim(&mut self, timestamp: &str) {
        self.messages.push(Message {
            role: MessageRole::System,
            content: "Vim mode: not yet available. Keybindings are hardcoded.".to_string(),
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    // ── Hydra-Exclusive ──

    pub(crate) fn slash_cmd_version(&mut self, timestamp: &str) {
        let version = env!("CARGO_PKG_VERSION");
        let msg = format!(
            "Hydra v{}\n\
             Sisters:   {}/{}\n\
             Tools:     {}+\n\
             Model:     {}\n\
             Autonomy:  {}",
            version,
            self.connected_count, self.total_sisters,
            self.tool_count,
            self.model_name,
            self.trust_level,
        );
        self.messages.push(Message {
            role: MessageRole::System,
            content: msg,
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    pub(crate) fn slash_cmd_env(&mut self, args: &str, timestamp: &str) {
        if args == "refresh" {
            self.project_info = crate::tui::project::detect_project(
                std::path::Path::new(&self.working_dir)
            );
            self.messages.push(Message {
                role: MessageRole::System,
                content: "Environment re-probed.".to_string(),
                timestamp: timestamp.to_string(),
                phase: None,
            });
            return;
        }

        let project_desc = if let Some(ref info) = self.project_info {
            format!(
                "Project:   {} {} ({}{})\n\
                 Git:       {}",
                info.kind.icon(), info.name, info.kind.label(),
                info.crate_count.map(|c| format!(", {} crates", c)).unwrap_or_default(),
                info.git_branch.as_deref().unwrap_or("none"),
            )
        } else {
            "Project:   not detected".to_string()
        };

        let msg = format!(
            "Environment Profile\n\
             \n\
             {}\n\
             Working:   {}\n\
             Shell:     {}\n\
             Platform:  {}",
            project_desc,
            self.working_dir,
            std::env::var("SHELL").unwrap_or_else(|_| "unknown".into()),
            std::env::consts::OS,
        );
        self.messages.push(Message {
            role: MessageRole::System,
            content: msg,
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    pub(crate) fn slash_cmd_dream(&mut self, timestamp: &str) {
        let dream = self.invention_engine.maybe_dream();
        let msg = if let Some(text) = dream {
            format!("Last Dream State activity:\n  {}", text)
        } else {
            "No dream activity yet. Hydra consolidates during idle periods.".to_string()
        };
        self.messages.push(Message {
            role: MessageRole::System,
            content: msg,
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    pub(crate) fn slash_cmd_obstacles(&mut self, timestamp: &str) {
        let (successes, failures, corrections) = self.invention_engine.session_momentum();
        let total = successes + failures;
        let content = if total == 0 {
            "No obstacles encountered this session.".into()
        } else {
            format!("Session Obstacles\n\nTotal interactions: {}\n  Successes: {}\n  Failures: {}\n  Corrections: {}\n  Success rate: {:.0}%",
                total, successes, failures, corrections,
                if total > 0 { successes as f64 / total as f64 * 100.0 } else { 0.0 })
        };
        self.messages.push(Message { role: MessageRole::System,
            content, timestamp: timestamp.to_string(), phase: None });
    }

    pub(crate) fn slash_cmd_threat(&mut self, args: &str, timestamp: &str) {
        let content = match args.trim() {
            "history" => self.threat_correlator.signal_history(20),
            "patterns" => self.threat_correlator.patterns_summary(),
            _ => self.threat_correlator.summary(),
        };
        self.messages.push(Message {
            role: MessageRole::System,
            content,
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    pub(crate) fn slash_cmd_autonomy(&mut self, args: &str, timestamp: &str) {
        if args.is_empty() {
            self.messages.push(Message {
                role: MessageRole::System,
                content: format!("Autonomy level: {}", self.trust_level),
                timestamp: timestamp.to_string(),
                phase: None,
            });
        } else {
            self.trust_level = format!("Level {}", args);
            self.messages.push(Message {
                role: MessageRole::System,
                content: format!("Autonomy set to: Level {}", args),
                timestamp: timestamp.to_string(),
                phase: None,
            });
        }
    }

    pub(crate) fn slash_cmd_implement(&mut self, args: &str, timestamp: &str) {
        if args.is_empty() {
            self.messages.push(Message {
                role: MessageRole::System,
                content: "Usage: /implement <spec-path>".to_string(),
                timestamp: timestamp.to_string(),
                phase: None,
            });
            return;
        }
        self.messages.push(Message {
            role: MessageRole::System,
            content: format!(
                "⚠ [HIGH RISK] implement spec {}\n\
                 Self-modification requested. Hydra will analyze gaps and apply patches.\n\
                 \n\
                 Approve? (y/n)",
                args
            ),
            timestamp: timestamp.to_string(),
            phase: None,
        });
        self.pending_approval = Some(super::app::PendingApproval {
            approval_id: None,
            risk_level: "HIGH".to_string(),
            action: format!("implement {}", args),
            description: "Self-modification via SelfImplement pipeline".to_string(),
        });
    }

    // ── Control ──

    pub(crate) fn slash_cmd_trust(&mut self, timestamp: &str) {
        let msg = format!("Trust level: {}", self.trust_level);
        self.messages.push(Message {
            role: MessageRole::System,
            content: msg,
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    pub(crate) fn slash_cmd_approve(&mut self, timestamp: &str) {
        if self.pending_approval.is_some() {
            self.handle_approval(true);
        } else {
            self.messages.push(Message {
                role: MessageRole::System,
                content: "No pending approval.".to_string(),
                timestamp: timestamp.to_string(),
                phase: None,
            });
        }
    }

    pub(crate) fn slash_cmd_deny(&mut self, timestamp: &str) {
        if self.pending_approval.is_some() {
            self.handle_approval(false);
        } else {
            self.messages.push(Message {
                role: MessageRole::System,
                content: "No pending approval.".to_string(),
                timestamp: timestamp.to_string(),
                phase: None,
            });
        }
    }

    pub(crate) fn slash_cmd_kill(&mut self) {
        self.kill_current();
    }

    // ── Debug ──

    pub(crate) fn slash_cmd_log(&mut self, timestamp: &str) {
        let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_else(|_| "/tmp".into());
        let log_path = format!("{}/.hydra/hydra-tui.log", home);
        match std::fs::read_to_string(&log_path) {
            Ok(content) => {
                let tail: String = content.lines().rev().take(30).collect::<Vec<_>>()
                    .into_iter().rev().collect::<Vec<_>>().join("\n");
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: format!("Last 30 log lines:\n{}", tail),
                    timestamp: timestamp.to_string(),
                    phase: None,
                });
            }
            Err(_) => {
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: format!("No log file at {}", log_path),
                    timestamp: timestamp.to_string(),
                    phase: None,
                });
            }
        }
    }

    pub(crate) fn slash_cmd_debug(&mut self, timestamp: &str) {
        let is_debug = std::env::var("HYDRA_DEBUG_MODE").map(|v| v == "1").unwrap_or(false);
        if is_debug {
            std::env::set_var("HYDRA_DEBUG_MODE", "0");
            self.messages.push(Message { role: MessageRole::System,
                content: "Debug mode OFF. Verbose logging disabled.".into(),
                timestamp: timestamp.to_string(), phase: None });
        } else {
            std::env::set_var("HYDRA_DEBUG_MODE", "1");
            self.messages.push(Message { role: MessageRole::System,
                content: "Debug mode ON. Verbose phase logging enabled.".into(),
                timestamp: timestamp.to_string(), phase: None });
        }
    }

    // ── Help ──

    pub(crate) fn slash_cmd_help(&mut self, timestamp: &str) {
        use crate::tui::commands::{COMMANDS, CommandCategory};
        let mut help = String::from("Hydra Commands\n\n");
        let categories = [
            ("Session", CommandCategory::Session),
            ("Model & Cost", CommandCategory::Model),
            ("Code & Review", CommandCategory::Code),
            ("Configuration", CommandCategory::Config),
            ("Integrations", CommandCategory::Integration),
            ("Agents & Skills", CommandCategory::Agent),
            ("Developer", CommandCategory::Developer),
            ("System", CommandCategory::System),
            ("Hydra", CommandCategory::Hydra),
            ("Control", CommandCategory::Control),
            ("Debug", CommandCategory::Debug),
        ];
        for (name, cat) in &categories {
            help.push_str(&format!("  {}:\n", name));
            for cmd in COMMANDS {
                if cmd.category == *cat {
                    help.push_str(&format!("    {:<14} {}\n", cmd.name, cmd.description));
                }
            }
            help.push('\n');
        }
        help.push_str("Shortcuts:\n");
        help.push_str("  Ctrl+S       Toggle sidebar\n");
        help.push_str("  Ctrl+K       Kill execution\n");
        help.push_str("  Ctrl+L       Refresh status\n");
        help.push_str("  Ctrl+D       Exit\n");
        help.push_str("  Shift+Tab    Cycle mode (Normal → Auto-Accept → Plan)\n");
        help.push_str("  Esc          Clear input\n");
        help.push_str("  Tab          Autocomplete command\n");
        self.messages.push(Message {
            role: MessageRole::System,
            content: help,
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }
}
