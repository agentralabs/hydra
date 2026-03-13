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

        // Validate that at least one API key is available
        let has_key = !std::env::var("ANTHROPIC_API_KEY").unwrap_or_default().is_empty()
            || !std::env::var("OPENAI_API_KEY").unwrap_or_default().is_empty()
            || !std::env::var("GOOGLE_API_KEY").unwrap_or_default().is_empty();
        if !has_key {
            self.messages.push(Message {
                role: MessageRole::System,
                content: "No API key configured.\n\n\
                    Set one of these environment variables and restart:\n\
                    \u{2022} ANTHROPIC_API_KEY (for Claude)\n\
                    \u{2022} OPENAI_API_KEY (for GPT)\n\
                    \u{2022} GOOGLE_API_KEY (for Gemini)\n\n\
                    Or run the onboarding wizard: rm ~/.hydra/profile.json && hydra".to_string(),
                timestamp: timestamp.to_string(),
                phase: None,
            });
            return;
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
                    session_count: self.messages.len() as u32,
                    anthropic_oauth_token: None,
                    runtime: hydra_native::cognitive::RuntimeSettings {
                        intent_cache: std::env::var("HYDRA_INTENT_CACHE").map(|v| v != "0" && v != "false").unwrap_or(true),
                        cache_ttl: std::env::var("HYDRA_CACHE_TTL").unwrap_or_else(|_| "1h".into()),
                        learn_corrections: std::env::var("HYDRA_LEARN_CORRECTIONS").map(|v| v != "0" && v != "false").unwrap_or(true),
                        belief_persist: std::env::var("HYDRA_BELIEF_PERSIST").unwrap_or_else(|_| "7 days".into()),
                        compression: std::env::var("HYDRA_COMPRESSION").unwrap_or_else(|_| "Balanced".into()),
                        dispatch_mode: std::env::var("HYDRA_DISPATCH_MODE").unwrap_or_else(|_| "Parallel".into()),
                        sister_timeout: std::env::var("HYDRA_SISTER_TIMEOUT").unwrap_or_else(|_| "10s".into()),
                        retry_failures: std::env::var("HYDRA_RETRY_FAILURES").map(|v| v != "0" && v != "false").unwrap_or(true),
                        dream_state: std::env::var("HYDRA_DREAM_STATE").map(|v| v != "0" && v != "false").unwrap_or(true),
                        proactive: std::env::var("HYDRA_PROACTIVE").map(|v| v != "0" && v != "false").unwrap_or(true),
                        risk_threshold: std::env::var("HYDRA_RISK_THRESHOLD").unwrap_or_else(|_| "medium".into()),
                        file_write: std::env::var("HYDRA_FILE_WRITE").map(|v| v != "0" && v != "false").unwrap_or(true),
                        network_access: std::env::var("HYDRA_NETWORK_ACCESS").map(|v| v != "0" && v != "false").unwrap_or(true),
                        shell_exec: std::env::var("HYDRA_SHELL_EXEC").map(|v| v != "0" && v != "false").unwrap_or(true),
                        max_file_edits: std::env::var("HYDRA_MAX_FILE_EDITS").unwrap_or_else(|_| "25".into()),
                        require_approval_critical: std::env::var("HYDRA_REQUIRE_APPROVAL_CRITICAL").map(|v| v != "0" && v != "false").unwrap_or(true),
                        sandbox_mode: std::env::var("HYDRA_SANDBOX_MODE").map(|v| v == "1" || v == "true").unwrap_or(false),
                        debug_mode: std::env::var("HYDRA_DEBUG_MODE").map(|v| v == "1" || v == "true").unwrap_or(false),
                        log_level: std::env::var("HYDRA_LOG_LEVEL").unwrap_or_else(|_| "info".into()),
                        federation_enabled: std::env::var("HYDRA_FEDERATION_ENABLED").map(|v| v == "1" || v == "true").unwrap_or(false),
                        memory_capture: self.memory_capture.clone(),
                        agentic_loop: std::env::var("HYDRA_AGENTIC_LOOP").map(|v| v != "0" && v != "false").unwrap_or(true),
                        agentic_max_turns: std::env::var("HYDRA_AGENTIC_MAX_TURNS").ok().and_then(|v| v.parse().ok()).unwrap_or(8),
                        agentic_token_budget: std::env::var("HYDRA_AGENTIC_TOKEN_BUDGET").ok().and_then(|v| v.parse().ok()).unwrap_or(50_000),
                    },
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
                let swarm = Some(std::sync::Arc::new(self.swarm_manager.clone_handle()));

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
                        swarm,
                    )
                    .await;
                });

                return;
            }
        }

        // Diagnose what's wrong and give actionable guidance
        let mut diag = format!("Cannot process: \"{}\"\n\n", input);
        if self.sisters_handle.is_none() {
            diag.push_str("\u{2717} Sisters: Not initialized (check ~/.local/bin/ for sister binaries)\n");
        } else {
            diag.push_str(&format!("\u{2717} Sisters: 0/{} connected\n", self.total_sisters));
        }
        if !self.server_online {
            diag.push_str("\u{2717} Server: Offline (try `hydra serve` in another terminal)\n");
        }
        diag.push_str("\nTo fix: install sisters or start the server, then restart Hydra.");
        self.messages.push(Message {
            role: MessageRole::Hydra,
            content: diag,
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
