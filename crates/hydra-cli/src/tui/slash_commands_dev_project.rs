//! Slash commands — developer project operations (/diff, /git, /test, /build, /run, /lint, /fmt, /deps, /bench, /doc, /deploy, /init).

use crate::tui::project;
use super::app::{App, Message, MessageRole};

impl App {
    pub(crate) fn slash_cmd_diff(&mut self, timestamp: &str) {
        let dir = std::path::Path::new(&self.working_dir);
        match project::git_diff(dir) {
            Some(diff) if !diff.is_empty() => {
                let lines: Vec<&str> = diff.lines().take(100).collect();
                let mut content = String::from("Uncommitted changes:\n\n");
                for line in &lines {
                    content.push_str(line);
                    content.push('\n');
                }
                let total = diff.lines().count();
                if total > 100 {
                    content.push_str(&format!("\n... {} more lines", total - 100));
                }
                self.messages.push(Message {
                    role: MessageRole::System,
                    content,
                    timestamp: timestamp.to_string(),
                    phase: None,
                });
            }
            _ => {
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: "No uncommitted changes.".to_string(),
                    timestamp: timestamp.to_string(),
                    phase: None,
                });
            }
        }
    }

    pub(crate) fn slash_cmd_git(&mut self, args: &str, timestamp: &str) {
        let dir = std::path::Path::new(&self.working_dir);
        let subcmd = args.split_whitespace().next().unwrap_or("status");
        match subcmd {
            "status" | "" => {
                match project::git_status(dir) {
                    Some(s) => {
                        self.messages.push(Message {
                            role: MessageRole::System,
                            content: format!("Git status:\n\n{}", s),
                            timestamp: timestamp.to_string(),
                            phase: None,
                        });
                    }
                    None => {
                        self.messages.push(Message {
                            role: MessageRole::System,
                            content: "Not a git repository.".to_string(),
                            timestamp: timestamp.to_string(),
                            phase: None,
                        });
                    }
                }
            }
            "log" => {
                let count: usize = args.split_whitespace().nth(1)
                    .and_then(|s| s.parse().ok()).unwrap_or(10);
                match project::git_log(dir, count) {
                    Some(s) => {
                        self.messages.push(Message {
                            role: MessageRole::System,
                            content: format!("Git log (last {}):\n\n{}", count, s),
                            timestamp: timestamp.to_string(),
                            phase: None,
                        });
                    }
                    None => {
                        self.messages.push(Message {
                            role: MessageRole::System,
                            content: "No git history.".to_string(),
                            timestamp: timestamp.to_string(),
                            phase: None,
                        });
                    }
                }
            }
            "commit" => {
                let msg = args.strip_prefix("commit").unwrap_or("").trim();
                if msg.is_empty() {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: "Usage: /git commit <message>".to_string(),
                        timestamp: timestamp.to_string(),
                        phase: None,
                    });
                } else {
                    self.spawn_command(
                        &format!("git commit -am \"{}\"", msg),
                        "git",
                        &["commit", "-am", msg],
                    );
                }
            }
            "push" => {
                self.spawn_command("git push", "git", &["push"]);
            }
            "pull" => {
                self.spawn_command("git pull", "git", &["pull"]);
            }
            "branch" => {
                self.spawn_command("git branch", "git", &["branch", "-a"]);
            }
            _ => {
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: format!("Git subcommands: status, log, commit, push, pull, branch"),
                    timestamp: timestamp.to_string(),
                    phase: None,
                });
            }
        }
    }

    pub(crate) fn slash_cmd_test(&mut self, timestamp: &str) {
        if let Some(ref info) = self.project_info {
            let (prog, cmd_args) = info.kind.test_cmd();
            let label = format!("{} {}", prog, cmd_args.join(" "));
            self.spawn_command(&label, prog, cmd_args);
        } else {
            self.messages.push(Message {
                role: MessageRole::System,
                content: "No project detected. Cannot determine test command.".to_string(),
                timestamp: timestamp.to_string(),
                phase: None,
            });
        }
    }

    pub(crate) fn slash_cmd_build(&mut self, timestamp: &str) {
        if let Some(ref info) = self.project_info {
            let (prog, cmd_args) = info.kind.build_cmd();
            let label = format!("{} {}", prog, cmd_args.join(" "));
            self.spawn_command(&label, prog, cmd_args);
        } else {
            self.messages.push(Message {
                role: MessageRole::System,
                content: "No project detected. Cannot determine build command.".to_string(),
                timestamp: timestamp.to_string(),
                phase: None,
            });
        }
    }

    pub(crate) fn slash_cmd_run(&mut self, timestamp: &str) {
        if let Some(ref info) = self.project_info {
            let (prog, cmd_args) = info.kind.run_cmd();
            let label = format!("{} {}", prog, cmd_args.join(" "));
            self.spawn_command(&label, prog, cmd_args);
        } else {
            self.messages.push(Message {
                role: MessageRole::System,
                content: "No project detected. Cannot determine run command.".to_string(),
                timestamp: timestamp.to_string(),
                phase: None,
            });
        }
    }

    pub(crate) fn slash_cmd_lint(&mut self, timestamp: &str) {
        if let Some(ref info) = self.project_info {
            let (prog, cmd_args) = info.kind.lint_cmd();
            let label = format!("{} {}", prog, cmd_args.join(" "));
            self.spawn_command(&label, prog, cmd_args);
        } else {
            self.messages.push(Message {
                role: MessageRole::System,
                content: "No project detected. Cannot determine lint command.".to_string(),
                timestamp: timestamp.to_string(),
                phase: None,
            });
        }
    }

    pub(crate) fn slash_cmd_fmt(&mut self, timestamp: &str) {
        if let Some(ref info) = self.project_info {
            let (prog, cmd_args) = info.kind.fmt_cmd();
            let label = format!("{} {}", prog, cmd_args.join(" "));
            self.spawn_command(&label, prog, cmd_args);
        } else {
            self.messages.push(Message {
                role: MessageRole::System,
                content: "No project detected. Cannot determine format command.".to_string(),
                timestamp: timestamp.to_string(),
                phase: None,
            });
        }
    }

    pub(crate) fn slash_cmd_deps(&mut self, timestamp: &str) {
        if let Some(ref info) = self.project_info {
            let (prog, cmd_args) = info.kind.deps_cmd();
            let label = format!("{} {}", prog, cmd_args.join(" "));
            self.spawn_command(&label, prog, cmd_args);
        } else {
            self.messages.push(Message {
                role: MessageRole::System,
                content: "No project detected. Cannot determine deps command.".to_string(),
                timestamp: timestamp.to_string(),
                phase: None,
            });
        }
    }

    pub(crate) fn slash_cmd_bench(&mut self, timestamp: &str) {
        if let Some(ref info) = self.project_info {
            let (prog, cmd_args) = info.kind.bench_cmd();
            let label = format!("{} {}", prog, cmd_args.join(" "));
            self.spawn_command(&label, prog, cmd_args);
        } else {
            self.messages.push(Message {
                role: MessageRole::System,
                content: "No project detected. Cannot determine bench command.".to_string(),
                timestamp: timestamp.to_string(),
                phase: None,
            });
        }
    }

    pub(crate) fn slash_cmd_doc(&mut self, timestamp: &str) {
        if let Some(ref info) = self.project_info {
            let (prog, cmd_args) = info.kind.doc_cmd();
            let label = format!("{} {}", prog, cmd_args.join(" "));
            self.spawn_command(&label, prog, cmd_args);
        } else {
            self.messages.push(Message {
                role: MessageRole::System,
                content: "No project detected. Cannot determine doc command.".to_string(),
                timestamp: timestamp.to_string(),
                phase: None,
            });
        }
    }

    pub(crate) fn slash_cmd_deploy(&mut self, timestamp: &str) {
        self.messages.push(Message {
            role: MessageRole::System,
            content: "Deploy target not configured. Set HYDRA_DEPLOY_CMD env var or configure in /config.".to_string(),
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    pub(crate) fn slash_cmd_init(&mut self, timestamp: &str) {
        self.messages.push(Message {
            role: MessageRole::System,
            content: "Hydra project initialization: coming soon. For now, Hydra auto-detects your project.".to_string(),
            timestamp: timestamp.to_string(),
            phase: None,
        });
        if let Some(ref info) = self.project_info {
            self.messages.push(Message {
                role: MessageRole::System,
                content: format!(
                    "Detected: {} {} ({})\nGit: {}{}",
                    info.kind.icon(), info.name, info.kind.label(),
                    info.git_branch.as_deref().unwrap_or("no git"),
                    match (info.git_ahead, info.git_behind) {
                        (Some(a), Some(b)) if a > 0 || b > 0 => format!(" (+{} -{} from remote)", a, b),
                        _ => String::new(),
                    }
                ),
                timestamp: timestamp.to_string(),
                phase: None,
            });
        }
    }
}
