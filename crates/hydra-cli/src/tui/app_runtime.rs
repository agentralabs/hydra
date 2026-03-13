//! Runtime methods — extracted from app.rs for file size.
//! Contains on_sisters_ready, tick, refresh_status, submit_input.

use chrono::Local;

use hydra_native::sisters::SistersHandle;
use super::app::{App, BootState, Message, MessageRole, PrStatus, PrState};

impl App {
    /// Called once after sisters have been spawned. Updates all sidebar state.
    pub fn on_sisters_ready(&mut self, handle: SistersHandle) {
        let all = handle.all_sisters();
        let mut connected = 0usize;
        let mut total_tools = 0u64;

        for (i, (_name, opt)) in all.iter().enumerate() {
            if i < self.sisters.len() {
                let is_connected = opt.is_some();
                let tools = opt.as_ref().map(|c| c.tools.len()).unwrap_or(0);
                self.sisters[i].connected = is_connected;
                self.sisters[i].tool_count = tools;
                if is_connected {
                    connected += 1;
                    total_tools += tools as u64;
                }
            }
        }

        self.connected_count = connected;
        self.tool_count = total_tools;
        self.sisters_handle = Some(handle);
        self.boot_state = BootState::Ready;

        self.health_pct = if self.total_sisters > 0 {
            ((connected as f64 / self.total_sisters as f64) * 100.0) as u8
        } else {
            0
        };

        self.server_online = self.client.health_check();

        // P7: Detect interrupted tasks from previous sessions
        let persister = hydra_native::task_persistence::TaskPersister::new();
        if let Ok(interrupted) = persister.list_incomplete() {
            if !interrupted.is_empty() {
                let mut msg = format!("Found {} interrupted task(s):\n\n", interrupted.len());
                for cp in &interrupted {
                    msg.push_str(&format!("  {} {}\n", "◉", hydra_native::task_persistence::format_task_summary(cp)));
                    msg.push_str(&format!("    /resume-task {}  |  /cancel-task {}\n\n", cp.task_id, cp.task_id));
                }
                self.messages.push(super::app::Message {
                    role: super::app::MessageRole::System,
                    content: msg,
                    timestamp: chrono::Local::now().format("%H:%M").to_string(),
                    phase: None,
                });
            }
        }
        let _ = persister.cleanup_old(7);
    }

    /// Periodic tick — refresh animations, drain cognitive updates, advance idle timer.
    pub fn tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);
        // Auto-scroll: if pinned to bottom before updates, stay pinned after
        let was_at_bottom = self.is_at_bottom();
        self.process_cognitive_updates();
        self.process_running_command();
        if was_at_bottom { self.scroll_offset = 0; }

        // Idle timer — tick_idle every ~1 second (20 ticks × 50ms).
        if self.tick_count % 20 == 0 {
            self.invention_engine.tick_idle(1);
            if self.tick_count % 200 == 0 {
                if let Some(dream_text) = self.invention_engine.maybe_dream() {
                    eprintln!("[hydra:tui:dream] {}", hydra_native::utils::safe_truncate(&dream_text, 200));
                }
            }
        }
        // File watcher: drain changes and generate suggestions every ~4 seconds
        if self.tick_count % 80 == 40 {
            if let Some(ref watcher) = self.file_watcher {
                let changes = watcher.drain_changes();
                if !changes.is_empty() {
                    let suggestions = self.proactive_file_engine.process_changes(&changes);
                    for s in suggestions {
                        let priority = format!("{:?}", s.priority);
                        let action_hint = s.action.map(|a| format!("{:?}", a));
                        let suffix = action_hint.map(|a| format!(" ({})", a)).unwrap_or_default();
                        self.messages.push(Message {
                            role: MessageRole::System,
                            content: format!("[{}] {} — {}{}", priority, s.title, s.message, suffix),
                            timestamp: chrono::Local::now().format("%H:%M").to_string(),
                            phase: None,
                        });
                    }
                }
            }
        }
        // PR status check — every ~60 seconds (1200 ticks × 50ms) (spec §11)
        if self.tick_count % 1200 == 0 && self.tick_count > 0 {
            self.check_pr_status();
        }
    }

    /// Refresh sister status from live handle.
    pub fn refresh_status(&mut self) {
        if let Some(ref handle) = self.sisters_handle {
            let all = handle.all_sisters();
            let mut connected = 0usize;
            for (i, (_name, opt)) in all.iter().enumerate() {
                if i < self.sisters.len() {
                    self.sisters[i].connected = opt.is_some();
                    if opt.is_some() {
                        connected += 1;
                    }
                }
            }
            self.connected_count = connected;
            self.health_pct = if self.total_sisters > 0 {
                ((connected as f64 / self.total_sisters as f64) * 100.0) as u8
            } else {
                0
            };
        }
        self.server_online = self.client.health_check();
    }

    /// Submit user input.
    pub fn submit_input(&mut self, input: &str) {
        // Dismiss welcome frame on first user interaction
        self.welcome_dismissed = true;

        // Allow escape commands even during approval prompt
        if input.starts_with('/') {
            match input {
                "/kill" | "/quit" | "/exit" | "/q" | "/deny" | "/n" | "/approve" | "/y" | "/clear" => {
                    // These commands work during approval
                    let timestamp = Local::now().format("%H:%M").to_string();
                    if !input.is_empty() {
                        self.history.push(input.to_string());
                    }
                    self.history_index = None;
                    self.handle_slash_command(input, &timestamp);
                    return;
                }
                _ => {}
            }
        }

        // Handle challenge phrase gate for CRITICAL actions
        if let Some(ref expected) = self.challenge_phrase.clone() {
            let timestamp = Local::now().format("%H:%M").to_string();
            if input.trim() == expected.as_str() {
                let action = self.challenge_action.take().unwrap_or_default();
                self.challenge_phrase = None;
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: format!("✓ Challenge accepted. Executing: {}", action),
                    timestamp,
                    phase: None,
                });
            } else {
                self.challenge_phrase = None;
                self.challenge_action = None;
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: "Challenge phrase incorrect. Action cancelled.".to_string(),
                    timestamp,
                    phase: None,
                });
            }
            self.scroll_to_bottom();
            return;
        }

        // Handle approval response
        if self.pending_approval.is_some() {
            let lower = input.trim().to_lowercase();
            if lower == "y" || lower == "yes" {
                self.handle_approval(true);
            } else if lower == "n" || lower == "no" {
                self.handle_approval(false);
            } else {
                let timestamp = Local::now().format("%H:%M").to_string();
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: "Type 'y' to approve, 'n' to deny, or /kill to cancel.".to_string(),
                    timestamp,
                    phase: None,
                });
                self.scroll_to_bottom();
            }
            return;
        }

        if !input.is_empty() {
            self.history.push(input.to_string());
            // Cap history to prevent unbounded growth
            if self.history.len() > 500 {
                self.history.drain(0..self.history.len() - 500);
            }
        }
        self.history_index = None;

        let timestamp = Local::now().format("%H:%M").to_string();

        if input.starts_with('/') {
            self.handle_slash_command(input, &timestamp);
            return;
        }

        // Input syntax: @file, !command, #memory (§4.2)
        if input.starts_with('!') {
            // Direct shell execution (! prefix = raw bash)
            let cmd = input[1..].trim();
            self.messages.push(Message {
                role: MessageRole::User,
                content: format!("!{}", cmd),
                timestamp: timestamp.clone(),
                phase: None,
            });
            if !cmd.is_empty() {
                self.spawn_command(cmd, "sh", &["-c", cmd]);
            }
            self.scroll_to_bottom();
            return;
        }
        if input.starts_with('#') {
            // Quick add to memory
            let note = &input[1..].trim();
            self.messages.push(Message {
                role: MessageRole::System,
                content: format!("Noted: {}", note),
                timestamp: timestamp.clone(),
                phase: None,
            });
            self.scroll_to_bottom();
            return;
        }

        // @file references (§4.2): extract file paths and include content in intent
        let (display_text, intent_text) = if input.contains('@') {
            let mut display = input.to_string();
            let mut expanded = input.to_string();
            // Find @path tokens (space-delimited or end-of-string)
            let words: Vec<&str> = input.split_whitespace().collect();
            for word in &words {
                if word.starts_with('@') && word.len() > 1 {
                    let path = &word[1..];
                    if let Ok(content) = std::fs::read_to_string(path) {
                        let lines = content.lines().count();
                        display = display.replace(word, &format!("@{} ({} lines)", path, lines));
                        expanded = expanded.replace(
                            word,
                            &format!("[file: {} ({} lines)]", path, lines),
                        );
                    }
                }
            }
            (display, expanded)
        } else {
            (input.to_string(), input.to_string())
        };

        self.messages.push(Message {
            role: MessageRole::User,
            content: display_text,
            timestamp: timestamp.clone(),
            phase: None,
        });

        // Track in conversation history
        self.conversation_history.push(("user".to_string(), intent_text.clone()));

        self.execute_intent(&intent_text, &timestamp);
        self.scroll_to_bottom();
    }

    /// Check PR status for current branch using `gh pr view` (spec §11).
    fn check_pr_status(&mut self) {
        // Run `gh pr view --json number,state,url,reviewDecision` non-blocking
        let output = std::process::Command::new("gh")
            .args(["pr", "view", "--json", "number,state,url,reviewDecision"])
            .output();
        if let Ok(out) = output {
            if out.status.success() {
                if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&out.stdout) {
                    let number = json["number"].as_u64().unwrap_or(0) as u32;
                    let url = json["url"].as_str().unwrap_or("").to_string();
                    let state_str = json["state"].as_str().unwrap_or("");
                    let review = json["reviewDecision"].as_str().unwrap_or("");
                    let state = match (state_str, review) {
                        ("MERGED", _) => PrState::Merged,
                        (_, "APPROVED") => PrState::Approved,
                        (_, "CHANGES_REQUESTED") => PrState::ChangesRequested,
                        (_, "REVIEW_REQUIRED") => PrState::ReviewRequested,
                        _ => PrState::Open,
                    };
                    if number > 0 {
                        self.pr_status = Some(PrStatus { number, state, url });
                    }
                }
            } else {
                // No PR for this branch — clear
                self.pr_status = None;
            }
        }
    }
}
