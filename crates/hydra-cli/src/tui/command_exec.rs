//! Command execution — extracted from app.rs for file size.
//! Contains spawn_command, process_running_command, execute_intent, kill_current.

use chrono::Local;
use tokio::sync::mpsc;

use hydra_native::cognitive::{CognitiveLoopConfig, CognitiveUpdate, run_cognitive_loop};
use super::app::{App, CommandEvent, Message, MessageRole, RunningCommand};

impl App {
    /// Drain output from a running shell command.
    pub(crate) fn process_running_command(&mut self) {
        let (finished, new_lines) = {
            match self.running_cmd.as_mut() {
                Some(cmd) => {
                    let mut lines = Vec::new();
                    let mut done = false;
                    loop {
                        match cmd.rx.try_recv() {
                            Ok(CommandEvent::Line(line)) => {
                                cmd.lines.push(line.clone());
                                lines.push(line);
                            }
                            Ok(CommandEvent::Done(code)) => {
                                cmd.exit_code = code;
                                done = true;
                                break;
                            }
                            Err(mpsc::error::TryRecvError::Empty) => break,
                            Err(mpsc::error::TryRecvError::Disconnected) => {
                                done = true;
                                break;
                            }
                        }
                    }
                    (done, lines)
                }
                None => return,
            }
        };

        // Update the last message with new lines
        if !new_lines.is_empty() || finished {
            if let Some(cmd) = &self.running_cmd {
                let elapsed = cmd.start.elapsed().as_secs_f64();
                let status = if finished {
                    let code = cmd.exit_code.unwrap_or(-1);
                    if code == 0 {
                        format!("completed successfully ({:.1}s)", elapsed)
                    } else {
                        format!("failed with exit code {} ({:.1}s)", code, elapsed)
                    }
                } else {
                    format!("running ({:.1}s)...", elapsed)
                };

                // Build display: show last 40 lines max
                let display_lines: Vec<&str> = cmd.lines.iter()
                    .rev().take(40).collect::<Vec<_>>()
                    .into_iter().rev()
                    .map(|s| s.as_str())
                    .collect();

                let mut content = format!("$ {}\n", cmd.label);
                for line in &display_lines {
                    content.push_str(line);
                    content.push('\n');
                }
                content.push_str(&format!("\n{}", status));

                let timestamp = Local::now().format("%H:%M").to_string();
                // Replace the last system message if it's the command output
                if let Some(last) = self.messages.last_mut() {
                    if last.role == MessageRole::System && last.content.starts_with(&format!("$ {}", cmd.label)) {
                        last.content = content;
                        last.timestamp = timestamp;
                    } else {
                        self.messages.push(Message {
                            role: MessageRole::System,
                            content,
                            timestamp,
                            phase: Some("Act".to_string()),
                        });
                    }
                }
                self.scroll_to_bottom();
            }
        }

        if finished {
            self.running_cmd = None;
        }
    }

    /// Spawn a shell command asynchronously and stream its output.
    pub(crate) fn spawn_command(&mut self, label: &str, program: &str, args: &[&str]) {
        let timestamp = Local::now().format("%H:%M").to_string();

        // Show initial message
        self.messages.push(Message {
            role: MessageRole::System,
            content: format!("$ {}\nStarting...", label),
            timestamp,
            phase: Some("Act".to_string()),
        });
        self.scroll_to_bottom();

        let (tx, rx) = mpsc::unbounded_channel::<CommandEvent>();
        let program = program.to_string();
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let work_dir = self.working_dir.clone();

        tokio::spawn(async move {
            use tokio::io::{AsyncBufReadExt, BufReader};
            use tokio::process::Command;

            let child = Command::new(&program)
                .args(&args)
                .current_dir(&work_dir)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn();

            match child {
                Ok(mut child) => {
                    // Read stdout
                    let stdout = child.stdout.take();
                    let stderr = child.stderr.take();
                    let tx2 = tx.clone();

                    let stdout_task = tokio::spawn(async move {
                        if let Some(stdout) = stdout {
                            let reader = BufReader::new(stdout);
                            let mut lines = reader.lines();
                            while let Ok(Some(line)) = lines.next_line().await {
                                if tx2.send(CommandEvent::Line(line)).is_err() {
                                    break;
                                }
                            }
                        }
                    });

                    let tx3 = tx.clone();
                    let stderr_task = tokio::spawn(async move {
                        if let Some(stderr) = stderr {
                            let reader = BufReader::new(stderr);
                            let mut lines = reader.lines();
                            while let Ok(Some(line)) = lines.next_line().await {
                                if tx3.send(CommandEvent::Line(line)).is_err() {
                                    break;
                                }
                            }
                        }
                    });

                    let _ = stdout_task.await;
                    let _ = stderr_task.await;

                    let status = child.wait().await;
                    let code = status.ok().and_then(|s| s.code());
                    let _ = tx.send(CommandEvent::Done(code));
                }
                Err(e) => {
                    let _ = tx.send(CommandEvent::Line(format!("Failed to start: {}", e)));
                    let _ = tx.send(CommandEvent::Done(Some(1)));
                }
            }
        });

        self.running_cmd = Some(RunningCommand {
            label: label.to_string(),
            lines: Vec::new(),
            rx,
            start: std::time::Instant::now(),
            exit_code: None,
        });
    }

    pub(crate) fn execute_intent(&mut self, input: &str, timestamp: &str) {
        // If we have a server, route through it
        if self.server_online {
            let body = serde_json::json!({ "intent": input });
            if let Ok(resp) = self.client.post("/api/conversations", &body) {
                let content = resp
                    .get("response")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Intent received. Processing...");
                self.messages.push(Message {
                    role: MessageRole::Hydra,
                    content: content.to_string(),
                    timestamp: timestamp.to_string(),
                    phase: Some("Act".to_string()),
                });
                return;
            }
        }

        // Sisters connected locally → spawn cognitive loop (same as Desktop)
        if let Some(ref sisters) = self.sisters_handle {
            if self.connected_count > 0 {
                // Drop previous channel to signal old loop to stop
                self.cognitive_rx = None;

                let (tx, rx) = mpsc::unbounded_channel::<CognitiveUpdate>();
                self.cognitive_rx = Some(rx);
                self.is_thinking = true;
                self.thinking_status = "Processing...".into();

                let task_id = uuid::Uuid::new_v4().to_string();
                let loop_config = CognitiveLoopConfig {
                    text: input.to_string(),
                    anthropic_key: std::env::var("ANTHROPIC_API_KEY").unwrap_or_default(),
                    openai_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
                    google_key: std::env::var("GOOGLE_API_KEY").unwrap_or_default(),
                    model: std::env::var("HYDRA_MODEL").unwrap_or_else(|_| "claude-sonnet-4-20250514".to_string()),
                    user_name: self.user_name.clone(),
                    task_id,
                    history: self.conversation_history.clone(),
                    anthropic_oauth_token: None,
                };

                let sisters_handle = Some(sisters.clone());
                let decide = self.decide_engine.clone();
                let undo = Some(self.undo_stack.clone());
                let inv = Some(self.invention_engine.clone());
                let notifier = Some(self.proactive_notifier.clone());
                let spawner = Some(self.agent_spawner.clone());
                let approval = Some(self.approval_manager.clone());
                let db = self.db.clone();
                let fed = Some(self.federation_manager.clone());

                tokio::spawn(async move {
                    run_cognitive_loop(
                        loop_config,
                        sisters_handle,
                        tx,
                        decide,
                        undo,
                        inv,
                        notifier,
                        spawner,
                        approval,
                        db,
                        fed,
                    )
                    .await;
                });

                return;
            }
        }

        // No sisters, no server
        self.messages.push(Message {
            role: MessageRole::Hydra,
            content: format!(
                "Received: \"{}\"\n\n\
                 [No sisters connected]\n\
                 Sister binaries not found in ~/.local/bin/\n\
                 Install sisters or run `hydra serve` for server mode.",
                input
            ),
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    /// Kill current running execution.
    pub fn kill_current(&mut self) {
        let timestamp = Local::now().format("%H:%M").to_string();
        if self.server_online {
            let _ = self.client.post("/api/system/kill", &serde_json::json!({}));
        }
        self.progress = None;
        self.current_phase = None;
        self.is_thinking = false;
        self.thinking_status.clear();
        self.cognitive_rx = None; // Drop the channel — loop will stop when tx fails
        self.pending_approval = None; // Clear any pending approval prompt
        // Kill running shell command by dropping the receiver
        if self.running_cmd.is_some() {
            self.running_cmd = None;
        }
        self.messages.push(Message {
            role: MessageRole::System,
            content: "Execution killed.".to_string(),
            timestamp,
            phase: None,
        });
        self.scroll_to_bottom();
    }

    /// Handle approval decision (y/n).
    pub(crate) fn handle_approval(&mut self, approved: bool) {
        let timestamp = Local::now().format("%H:%M").to_string();
        if let Some(approval) = self.pending_approval.take() {
            if approved {
                if let Some(id) = &approval.approval_id {
                    let _ = self.approval_manager.submit_decision(
                        id,
                        hydra_runtime::approval::ApprovalDecision::Approved,
                    );
                }
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: format!("Approved: {}", approval.action),
                    timestamp,
                    phase: Some("Decide".to_string()),
                });
            } else {
                if let Some(id) = &approval.approval_id {
                    let _ = self.approval_manager.submit_decision(
                        id,
                        hydra_runtime::approval::ApprovalDecision::Denied {
                            reason: "User denied".into(),
                        },
                    );
                }
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: format!("Denied: {}", approval.action),
                    timestamp,
                    phase: Some("Decide".to_string()),
                });
            }
        }
        self.scroll_to_bottom();
    }
}
