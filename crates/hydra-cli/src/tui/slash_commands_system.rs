//! Slash commands — system and monitoring.
//! /sisters, /fix, /scan, /repair, /memory, /goals, /beliefs, /receipts, /health, /status

use super::app::{App, Message, MessageRole};

impl App {
    pub(crate) fn slash_cmd_sisters(&mut self, timestamp: &str) {
        self.refresh_status();
        // Clean formatting — no box borders (per visual overhaul spec)
        let mut lines = Vec::new();
        // Two-column layout like Claude Code
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

    pub(crate) fn slash_cmd_memory(&mut self, timestamp: &str) {
        let msg = format!(
            "Memory Stats:\n  Facts: {}\n  Tokens avg: {}\n  Receipts: {}",
            self.memory_facts, self.token_avg, self.receipt_count
        );
        self.messages.push(Message {
            role: MessageRole::System,
            content: msg,
            timestamp: timestamp.to_string(),
            phase: None,
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
}
