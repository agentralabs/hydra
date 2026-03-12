//! Slash commands — Integrations, Agents, Skills.
//! Split for 400-line limit.

use super::app::{App, Message, MessageRole};

impl App {
    // ── Integrations ──

    pub(crate) fn slash_cmd_mcp(&mut self, args: &str, timestamp: &str) {
        if args.is_empty() {
            let sister_count = self.connected_count;
            let msg = format!(
                "MCP Servers\n\
                 \n\
                 Sister MCP servers: {} connected\n\
                 External servers:   0 configured\n\
                 \n\
                 Manage from CLI:\n\
                   hydra mcp add <name> <command>\n\
                   hydra mcp add --transport http <name> <url>\n\
                   hydra mcp list\n\
                   hydra mcp remove <name>\n\
                 \n\
                 All {} sisters expose MCP tools ({}+ total).",
                sister_count, self.total_sisters, self.tool_count,
            );
            self.messages.push(Message {
                role: MessageRole::System,
                content: msg,
                timestamp: timestamp.to_string(),
                phase: None,
            });
        } else {
            self.messages.push(Message {
                role: MessageRole::System,
                content: format!("MCP: subcommand '{}' — use CLI `hydra mcp {}` for server management.", args, args),
                timestamp: timestamp.to_string(),
                phase: None,
            });
        }
    }

    pub(crate) fn slash_cmd_ide(&mut self, timestamp: &str) {
        self.messages.push(Message {
            role: MessageRole::System,
            content: "IDE Integrations\n\
                     \n\
                     VSCode extension: installed (hydra-vscode)\n\
                     \n\
                     Manage from CLI:\n\
                       hydra ide install vscode\n\
                       hydra ide install jetbrains\n\
                       hydra ide status".to_string(),
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    pub(crate) fn slash_cmd_install_github_app(&mut self, timestamp: &str) {
        self.messages.push(Message {
            role: MessageRole::System,
            content: "GitHub App Setup\n\
                     \n\
                     Install the Hydra GitHub App for:\n\
                       — Automated PR reviews\n\
                       — Issue triage\n\
                       — CI integration\n\
                     \n\
                     Run: hydra install-github-app\n\
                     Or visit: github.com/apps/hydra-ai".to_string(),
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    pub(crate) fn slash_cmd_hooks(&mut self, timestamp: &str) {
        let home = std::env::var("HOME").unwrap_or_else(|_| "~".into());
        let hooks_path = format!("{}/.hydra/settings.json", home);
        let project_hooks = ".hydra/settings.json";

        let has_global = std::path::Path::new(&hooks_path).exists();
        let has_project = std::path::Path::new(project_hooks).exists();

        let msg = format!(
            "Hook Configuration\n\
             \n\
             Global hooks:  {} ({})\n\
             Project hooks: {} ({})\n\
             \n\
             Hook events: PreToolUse, PostToolUse, Stop, SubagentStop,\n\
             TaskCompleted, TeammateIdle\n\
             \n\
             Example (in settings.json):\n\
               \"hooks\": {{\n\
                 \"PostToolUse\": [{{\n\
                   \"matcher\": \"Write(*.rs)\",\n\
                   \"hooks\": [{{ \"type\": \"command\", \"command\": \"cargo fmt -- $file\" }}]\n\
                 }}]\n\
               }}",
            if has_global { "configured" } else { "none" }, hooks_path,
            if has_project { "configured" } else { "none" }, project_hooks,
        );
        self.messages.push(Message {
            role: MessageRole::System,
            content: msg,
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    pub(crate) fn slash_cmd_plugin(&mut self, args: &str, timestamp: &str) {
        let msg = match args {
            "list" | "" => "Plugins\n\
                           \n\
                           No plugins installed.\n\
                           \n\
                           Usage:\n\
                             /plugin install <name>\n\
                             /plugin list\n\
                             /plugin uninstall <name>".to_string(),
            _ => format!("Plugin '{}': not found. Use /plugin list to see available.", args),
        };
        self.messages.push(Message {
            role: MessageRole::System,
            content: msg,
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    pub(crate) fn slash_cmd_remote_control(&mut self, timestamp: &str) {
        self.messages.push(Message {
            role: MessageRole::System,
            content: "Remote Control\n\
                     \n\
                     Status: disabled\n\
                     Enable with: /remote-control enable\n\
                     \n\
                     When enabled, Hydra can be controlled from the web interface\n\
                     at http://localhost:3000".to_string(),
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    pub(crate) fn slash_cmd_remote(&mut self, timestamp: &str) {
        self.messages.push(Message {
            role: MessageRole::System,
            content: "Remote Sessions\n\
                     \n\
                     No remote sessions available.\n\
                     \n\
                     Connect to a remote Hydra instance:\n\
                       /remote connect <host:port>\n\
                       /remote disconnect".to_string(),
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    // ── Agents & Skills ──

    pub(crate) fn slash_cmd_agents(&mut self, timestamp: &str) {
        let project_dir = ".hydra/agents/";
        let home = std::env::var("HOME").unwrap_or_else(|_| "~".into());
        let personal_dir = format!("{}/.hydra/agents/", home);

        let project_count = std::fs::read_dir(project_dir)
            .map(|d| d.filter(|e| e.as_ref().map(|e| e.path().extension().map(|x| x == "md").unwrap_or(false)).unwrap_or(false)).count())
            .unwrap_or(0);
        let personal_count = std::fs::read_dir(&personal_dir)
            .map(|d| d.filter(|e| e.as_ref().map(|e| e.path().extension().map(|x| x == "md").unwrap_or(false)).unwrap_or(false)).count())
            .unwrap_or(0);

        let mut msg = format!(
            "Subagents\n\
             \n\
             Built-in:\n\
               Plan agent     — Structured planning for complex tasks\n\
               Explore agent  — Read-only codebase exploration\n\
               Task agent     — Delegated subtask execution\n\
             \n\
             Custom agents: {} project, {} personal",
            project_count, personal_count,
        );
        if project_count > 0 || personal_count > 0 {
            msg.push_str("\n\nDefine agents in .hydra/agents/ or ~/.hydra/agents/");
        }
        self.messages.push(Message {
            role: MessageRole::System,
            content: msg,
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    pub(crate) fn slash_cmd_skills(&mut self, timestamp: &str) {
        let project_dir = ".hydra/skills/";
        let home = std::env::var("HOME").unwrap_or_else(|_| "~".into());
        let personal_dir = format!("{}/.hydra/skills/", home);

        let project_count = std::fs::read_dir(project_dir)
            .map(|d| d.filter(|e| e.is_ok()).count())
            .unwrap_or(0);
        let personal_count = std::fs::read_dir(&personal_dir)
            .map(|d| d.filter(|e| e.is_ok()).count())
            .unwrap_or(0);

        let msg = format!(
            "Skills\n\
             \n\
             Project skills: {} ({})\n\
             Personal skills: {} ({})\n\
             \n\
             Define skills as markdown files in .hydra/skills/ or ~/.hydra/skills/\n\
             Skills with disable-model-invocation: false can auto-trigger.",
            project_count, project_dir,
            personal_count, personal_dir,
        );
        self.messages.push(Message {
            role: MessageRole::System,
            content: msg,
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    pub(crate) fn slash_cmd_commands(&mut self, timestamp: &str) {
        use crate::tui::commands::COMMANDS;
        let mut msg = format!("All Commands ({} total)\n\n", COMMANDS.len());
        for cmd in COMMANDS {
            msg.push_str(&format!("  {:<20} {}\n", cmd.name, cmd.description));
        }
        self.messages.push(Message {
            role: MessageRole::System,
            content: msg,
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    pub(crate) fn slash_cmd_plan(&mut self, timestamp: &str) {
        self.permission_mode = super::app::PermissionMode::Plan;
        self.messages.push(Message {
            role: MessageRole::System,
            content: "Entered Plan mode. Hydra will plan but not execute.\n\
                     Use Shift+Tab to cycle back to Normal mode.".to_string(),
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    pub(crate) fn slash_cmd_bashes(&mut self, timestamp: &str) {
        if self.running_cmd.is_some() {
            let cmd = self.running_cmd.as_ref().unwrap();
            let elapsed = cmd.start.elapsed().as_secs();
            let msg = format!(
                "Background Processes\n\
                 \n\
                 1. {} ({}s elapsed, {} lines)\n\
                 \n\
                 Use /kill to stop.",
                cmd.label, elapsed, cmd.lines.len(),
            );
            self.messages.push(Message {
                role: MessageRole::System,
                content: msg,
                timestamp: timestamp.to_string(),
                phase: None,
            });
        } else {
            self.messages.push(Message {
                role: MessageRole::System,
                content: "Background Processes\n\n  No background tasks running.".to_string(),
                timestamp: timestamp.to_string(),
                phase: None,
            });
        }
    }

    pub(crate) fn slash_cmd_tasks(&mut self, timestamp: &str) {
        let mut msg = String::from("Tasks\n\n");
        // Persistent checkpoints from disk
        let persister = hydra_native::task_persistence::TaskPersister::new();
        if let Ok(checkpoints) = persister.list_incomplete() {
            if !checkpoints.is_empty() {
                msg.push_str("  Interrupted (resumable):\n");
                for cp in &checkpoints {
                    msg.push_str(&format!("    ◉ {}\n", hydra_native::task_persistence::format_task_summary(cp)));
                    msg.push_str(&format!("      /resume-task {}  |  /cancel-task {}\n", cp.task_id, cp.task_id));
                }
                msg.push('\n');
            }
        }
        // Session tasks
        if !self.recent_tasks.is_empty() {
            msg.push_str("  Session:\n");
            for (i, task) in self.recent_tasks.iter().enumerate() {
                let icon = match task.status {
                    super::app::TaskStatus::Complete => "✓",
                    super::app::TaskStatus::Running => "◉",
                    super::app::TaskStatus::Failed => "✗",
                };
                msg.push_str(&format!("    {} {}. {}\n", icon, i + 1, task.summary));
            }
        }
        if self.recent_tasks.is_empty() && msg.len() < 15 {
            msg.push_str("  No active or interrupted tasks.\n");
        }
        self.messages.push(Message {
            role: MessageRole::System,
            content: msg,
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    pub(crate) fn slash_cmd_resume_task(&mut self, args: &str, timestamp: &str) {
        let task_id = args.trim();
        if task_id.is_empty() {
            self.messages.push(Message {
                role: MessageRole::System,
                content: "Usage: /resume-task <task-id>".to_string(),
                timestamp: timestamp.to_string(),
                phase: None,
            });
            return;
        }
        self.execute_intent(&format!("/test-repo --resume {}", task_id), timestamp);
    }

    pub(crate) fn slash_cmd_cancel_task(&mut self, args: &str, timestamp: &str) {
        let task_id = args.trim();
        if task_id.is_empty() {
            self.messages.push(Message {
                role: MessageRole::System,
                content: "Usage: /cancel-task <task-id>".to_string(),
                timestamp: timestamp.to_string(),
                phase: None,
            });
            return;
        }
        let persister = hydra_native::task_persistence::TaskPersister::new();
        let msg = match hydra_native::task_persistence::recovery::cancel_task(&persister, task_id) {
            Ok(m) => m,
            Err(e) => format!("Failed to cancel: {}", e),
        };
        self.messages.push(Message {
            role: MessageRole::System,
            content: msg,
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }
}
