//! Cognitive update handler — processes CognitiveUpdate events from the cognitive loop.

use chrono::Local;
use tokio::sync::mpsc;
use hydra_native::cognitive::CognitiveUpdate;
use super::app::{App, Message, MessageRole, PendingApproval};

impl App {
    /// Process pending CognitiveUpdate events from the channel.
    pub(crate) fn process_cognitive_updates(&mut self) {
        // Drain into a Vec — limit StreamChunks for streaming effect.
        let (updates, disconnected) = {
            match self.cognitive_rx.as_mut() {
                Some(rx) => {
                    let mut buf = Vec::new();
                    let mut disc = false;
                    let mut stream_count = 0u32;
                    loop {
                        match rx.try_recv() {
                            Ok(update) => {
                                let is_stream = matches!(&update, CognitiveUpdate::StreamChunk { .. });
                                buf.push(update);
                                if is_stream {
                                    stream_count += 1;
                                    // Limit: process max 3 stream chunks per tick (~50ms)
                                    // to create visible word-by-word streaming
                                    if stream_count >= 3 { break; }
                                }
                            }
                            Err(mpsc::error::TryRecvError::Empty) => break,
                            Err(mpsc::error::TryRecvError::Disconnected) => {
                                disc = true; break;
                            }
                        }
                    }
                    (buf, disc)
                }
                None => return,
            }
        };

        // If sender dropped (loop finished/panicked), clean up
        if disconnected && updates.is_empty() {
            self.is_thinking = false;
            self.cognitive_rx = None;
        }

        for update in updates {
            let timestamp = Local::now().format("%H:%M").to_string();

            match update {
                CognitiveUpdate::Phase(p) => {
                    // Map internal phases to user-friendly status messages
                    self.thinking_status = match p.as_str() {
                        "Perceive" | "Recall" => "Gathering context...".into(),
                        "Think" => "Thinking...".into(),
                        "Act" | "Act (direct)" => "Executing...".into(),
                        "Act (command)" => "Running command...".into(),
                        "Decide" => "Evaluating safety...".into(),
                        "Learn" => "Learning...".into(),
                        s if s.starts_with("Think (") => {
                            // "Think (Forge blueprint)" → "Generating with Forge..."
                            let inner = s.trim_start_matches("Think (").trim_end_matches(')');
                            format!("Working with {}...", inner)
                        }
                        s if s.starts_with("Omniscience") => "Scanning codebase...".into(),
                        s if s.starts_with("Deepening") => "Deepening analysis...".into(),
                        // Already user-friendly (ends with "...")
                        s if s.ends_with("...") => s.to_string(),
                        _ => format!("{}...", p),
                    };
                    self.current_phase = Some(p);
                }
                CognitiveUpdate::Typing(t) => {
                    self.is_thinking = t;
                    if t && self.thinking_status.is_empty() {
                        self.thinking_status = "Thinking...".into();
                    }
                }
                CognitiveUpdate::Message { role, content, css_class } => {
                    let msg_role = match role.as_str() {
                        "user" => MessageRole::User,
                        "hydra" | "assistant" => MessageRole::Hydra,
                        _ => MessageRole::System,
                    };
                    // Estimate tokens from all hydra/system messages (~4 chars per token)
                    if msg_role != MessageRole::User {
                        self.tokens_received += (content.len() as u64 + 3) / 4;
                    }
                    let api_role = if msg_role == MessageRole::User { "user" } else { "assistant" };
                    self.conversation_history.push((api_role.to_string(), content.clone()));

                    // "history-only" = already displayed via streaming, skip visible push
                    if css_class != "history-only" {
                        self.messages.push(Message {
                            role: msg_role,
                            content,
                            timestamp: timestamp.clone(),
                            phase: self.current_phase.clone(),
                        });
                        self.scroll_to_bottom();
                    }
                }
                CognitiveUpdate::ResetIdle => {
                    if let Some(s) = self.task_stats.build_summary(self.tick_count, self.tokens_received) {
                        self.messages.push(Message { role: MessageRole::System, content: s,
                            timestamp: timestamp.clone(), phase: Some("Summary".into()) });
                    }
                    self.current_phase = None; self.is_thinking = false;
                    self.thinking_status.clear(); self.thinking_elapsed_ms = 0;
                    self.running_sub_agents.clear();
                    self.progress = None; self.invention_engine.reset_idle();
                }
                CognitiveUpdate::AwaitApproval { approval_id, risk_level, action, description, .. } => {
                    if matches!(risk_level.as_str(), "critical" | "high" | "medium") {
                        self.pending_approval = Some(PendingApproval {
                            approval_id, risk_level: risk_level.clone(),
                            action: action.clone(), description: description.clone(),
                        });
                        self.messages.push(Message {
                            role: MessageRole::System,
                            content: format!("[{} RISK] {}\n{}\n\nApprove? (y/n)", risk_level.to_uppercase(), action, description),
                            timestamp: timestamp.clone(), phase: Some("Decide".into()),
                        });
                        self.scroll_to_bottom();
                    }
                }

                CognitiveUpdate::RepairStarted { spec, task } => {
                    self.messages.push(Message { role: MessageRole::System,
                        content: format!("● Repair({})\n  └ {}", task, spec),
                        timestamp: timestamp.clone(), phase: Some("Repair".into()) });
                    self.scroll_to_bottom();
                }
                CognitiveUpdate::RepairIteration { passed, total, .. } => {
                    self.thinking_status = format!("Repairing... ({}/{})", passed, total);
                    self.progress = Some((self.thinking_status.clone(), passed as f64 / total.max(1) as f64));
                }
                CognitiveUpdate::RepairCompleted { task, status, iterations } => {
                    self.messages.push(Message { role: MessageRole::System,
                        content: format!("● RepairComplete({})\n  └ {} ({} iterations)", task, status, iterations),
                        timestamp: timestamp.clone(), phase: Some("Repair".into()) });
                    self.progress = None; self.scroll_to_bottom();
                }

                CognitiveUpdate::OmniscienceAnalyzing { phase } => {
                    self.thinking_status = format!("Scanning: {}...", phase);
                    self.current_phase = Some(format!("Omniscience: {}", phase));
                }
                CognitiveUpdate::OmniscienceScanComplete { gaps_found, specs_generated, health_score } => {
                    self.messages.push(Message { role: MessageRole::System,
                        content: format!("● Scan complete\n  └ {} gaps, {} specs, {:.0}% health", gaps_found, specs_generated, health_score * 100.0),
                        timestamp: timestamp.clone(), phase: Some("Omniscience".into()) });
                    self.scroll_to_bottom();
                }
                CognitiveUpdate::OmniscienceGapFound { .. } => {}
                CognitiveUpdate::PhaseLoading { phase, elapsed_ms } => {
                    self.thinking_status = phase;
                    self.thinking_elapsed_ms = elapsed_ms;
                }

                // -- Phase 3, C4: Consolidation daemon cycle complete --
                CognitiveUpdate::ConsolidationCycleComplete { cycle, strengthened, decayed, gc_cleaned } => {
                    // Log as a quiet system message, not interrupting conversation
                    eprintln!(
                        "[hydra:tui] Consolidation cycle {} — +{} strengthened, -{} decayed, {} cleaned",
                        cycle, strengthened, decayed, gc_cleaned
                    );
                }

                // -- Plan events --
                // Only show plans with real deliverable steps, not generic internal phases
                CognitiveUpdate::PlanInit { goal, steps } => {
                    // Plans are internal — show as a thinking status, not as chat noise
                    if steps.len() > 1 {
                        self.thinking_status = format!("Planning: {} ({} steps)...", goal, steps.len());
                    }
                }
                CognitiveUpdate::PlanStepComplete { .. }
                | CognitiveUpdate::BeliefsLoaded { .. }
                | CognitiveUpdate::Celebrate(_)
                | CognitiveUpdate::SistersCalled { .. } => {}

                CognitiveUpdate::ToolAction { tool, args, result, success } => {
                    // Count tool I/O as tokens (~4 chars per token)
                    self.tokens_received += ((args.len() + result.len()) as u64 + 3) / 4;
                    if let Some(a) = self.running_sub_agents.iter_mut().find(|a| !a.done) {
                        a.tool_uses += 1;
                        a.activity = format!("{}{}", tool, if args.is_empty() { String::new() } else { format!("({})", args) });
                    }
                    self.task_stats.record_tool(&tool, &args);
                    let header = if args.is_empty() { format!("● {}", tool) } else { format!("● {}({})", tool, args) };
                    let marker = if success { "└" } else { "✗" };
                    self.messages.push(Message { role: MessageRole::System,
                        content: format!("{}\n  {} {}", header, marker, result),
                        timestamp: timestamp.clone(), phase: Some("Act".to_string()) });
                    self.scroll_to_bottom();
                }

                CognitiveUpdate::ProactiveAlert { title, message, priority } => {
                    self.messages.push(Message { role: MessageRole::System,
                        content: format!("[{}] {} — {}", priority, title, message),
                        timestamp: timestamp.clone(), phase: None });
                    self.scroll_to_bottom();
                }
                CognitiveUpdate::ProactiveFileSuggestion { title, message, priority, action } => {
                    let hint = action.map(|a| format!(" ({})", a)).unwrap_or_default();
                    self.messages.push(Message { role: MessageRole::System,
                        content: format!("[{}] {} — {}{}", priority, title, message, hint),
                        timestamp: timestamp.clone(), phase: None });
                    self.scroll_to_bottom();
                }

                // -- Ghost Cursor events → text-based progress in TUI --
                CognitiveUpdate::CursorMove { label, .. } => {
                    if let Some(label) = label {
                        self.progress = Some((label, 0.5));
                    }
                }
                CognitiveUpdate::CursorTyping { active } => {
                    if active {
                        self.progress = Some(("Typing...".to_string(), 0.5));
                    } else {
                        self.progress = None;
                    }
                }
                CognitiveUpdate::CursorVisibility { visible } => {
                    if !visible {
                        self.progress = None;
                    }
                }

                CognitiveUpdate::TokenUsage { input_tokens, output_tokens } => {
                    self.tokens_received += input_tokens + output_tokens;
                }
                CognitiveUpdate::MemoryModeChanged { mode } => { self.memory_capture = mode; }
                CognitiveUpdate::MemoryStatsUpdate { facts, tokens_avg, receipts } => {
                    self.memory_facts = facts; self.token_avg = tokens_avg; self.receipt_count = receipts;
                }
                CognitiveUpdate::IconState(_)
                | CognitiveUpdate::PhaseStatuses(_)
                | CognitiveUpdate::PlanClear
                | CognitiveUpdate::PlanStepStart(_)
                | CognitiveUpdate::EvidenceClear
                | CognitiveUpdate::EvidenceMemory { .. }
                | CognitiveUpdate::EvidenceCode { .. }
                | CognitiveUpdate::TimelineClear
                | CognitiveUpdate::SidebarCompleteTask(_)
                | CognitiveUpdate::SuggestMode(_)
                | CognitiveUpdate::SettingsApplied { .. }
                | CognitiveUpdate::StreamComplete
                | CognitiveUpdate::UndoStatus { .. }
                | CognitiveUpdate::SkillCrystallized { .. }
                | CognitiveUpdate::ReflectionInsight { .. }
                | CognitiveUpdate::CompressionApplied { .. }
                | CognitiveUpdate::DreamInsight { .. }
                | CognitiveUpdate::ShadowValidation { .. }
                | CognitiveUpdate::PredictionResult { .. }
                | CognitiveUpdate::PatternEvolved { .. }
                | CognitiveUpdate::TemporalStored { .. }
                | CognitiveUpdate::CursorClick
                | CognitiveUpdate::CursorModeChange { .. }
                | CognitiveUpdate::CursorPaused { .. }
                | CognitiveUpdate::McpSkillsDiscovered { .. }
                | CognitiveUpdate::FederationSync { .. }
                | CognitiveUpdate::FederationDelegated { .. }
                | CognitiveUpdate::RepairCheckResult { .. }
                | CognitiveUpdate::OmniscienceSpecGenerated { .. }
                | CognitiveUpdate::OmniscienceValidation { .. }
                | CognitiveUpdate::BeliefUpdated { .. }
                => {}
                CognitiveUpdate::GatewayStats { display } => { self.gateway_stats = display; }

                // -- Streaming: append chunks to current message --
                CognitiveUpdate::StreamChunk { content } => {
                    // Estimate tokens from stream chunks (~4 chars per token)
                    self.tokens_received += (content.len() as u64 + 3) / 4;
                    if let Some(last) = self.messages.last_mut() {
                        if last.role == MessageRole::Hydra && self.is_thinking {
                            last.content.push_str(&content);
                            self.scroll_to_bottom();
                            continue;
                        }
                    }
                    // Otherwise start a new streaming message
                    self.messages.push(Message {
                        role: MessageRole::Hydra,
                        content,
                        timestamp: timestamp.clone(),
                        phase: self.current_phase.clone(),
                    });
                    self.scroll_to_bottom();
                }

                CognitiveUpdate::SwarmSpawned { count, agent_ids } => {
                    use super::app_helpers::SubAgentState;
                    self.running_sub_agents = agent_ids.iter().map(|id| SubAgentState {
                        id: id.clone(), description: String::new(),
                        tool_uses: 0, tokens: 0, activity: String::new(), done: false,
                    }).collect();
                    self.thinking_status = format!("Running {} agents...", count);
                }
                CognitiveUpdate::SwarmTaskAssigned { agent_id, task_desc } => {
                    if let Some(a) = self.running_sub_agents.iter_mut().find(|a| a.id == agent_id) {
                        a.description = task_desc.clone();
                        a.activity = "Starting...".into();
                    }
                    self.thinking_status = format!("Running {} agents...", self.running_sub_agents.len());
                }
                CognitiveUpdate::SwarmResults { succeeded, total, failed, .. } => {
                    self.running_sub_agents.clear();
                    self.thinking_status.clear();
                    self.messages.push(Message { role: MessageRole::System,
                        content: format!("● Swarm completed\n  └ {}/{} succeeded, {} failed", succeeded, total, failed),
                        timestamp: timestamp.clone(), phase: Some("Swarm".into()) });
                    self.scroll_to_bottom();
                }
                CognitiveUpdate::AgenticTurn { turn, tool_count, exec_count } => {
                    for a in &mut self.running_sub_agents { if !a.done { a.tool_uses = tool_count; } }
                    self.thinking_status = format!("Agentic turn {} ({} tools, {} cmds)", turn, tool_count, exec_count);
                }
                CognitiveUpdate::AgenticComplete { turns, total_tokens, stop_reason } => {
                    self.tokens_received += total_tokens;
                    self.running_sub_agents.clear();
                    self.messages.push(Message { role: MessageRole::System,
                        content: format!("● AgenticLoop\n  └ {} turns, {}K tokens ({})", turns, total_tokens / 1000, stop_reason),
                        timestamp: timestamp.clone(), phase: Some("Act".into()) });
                    self.scroll_to_bottom();
                }

                CognitiveUpdate::ObstacleDetected { pattern, error_summary } => {
                    self.thinking_status = format!("Obstacle: {}...", pattern);
                    self.messages.push(Message { role: MessageRole::System,
                        content: format!("● Obstacle({})\n  ✗ {}", pattern, error_summary),
                        timestamp: timestamp.clone(), phase: self.current_phase.clone() });
                }
                CognitiveUpdate::ObstacleResolved { pattern, resolution, attempts } => {
                    self.thinking_status = "Obstacle resolved".into();
                    self.messages.push(Message { role: MessageRole::System,
                        content: format!("● Resolved({})\n  └ {} ({} attempts)", pattern, resolution, attempts),
                        timestamp: timestamp.clone(), phase: self.current_phase.clone() });
                }
                CognitiveUpdate::ProjectExecPhase { repo, phase, detail } => {
                    self.thinking_status = format!("[{}] {}", repo, phase);
                    self.messages.push(Message { role: MessageRole::System,
                        content: format!("● {}({})\n  └ {}", phase, repo, detail),
                        timestamp: timestamp.clone(), phase: self.current_phase.clone() });
                }

                // -- Phases 2-7: Superintelligence Pipeline --
                CognitiveUpdate::VerificationApplied { checked, corrected } => {
                    if corrected > 0 {
                        self.thinking_status = format!("Verified: {}/{} claims corrected", corrected, checked);
                    }
                }
                CognitiveUpdate::ModelEscalated { from, to, reason } => {
                    self.thinking_status = format!("Escalated: {} → {} ({})", from, to, reason);
                }
                CognitiveUpdate::BackgroundTaskComplete { task_name, summary } => {
                    eprintln!("[hydra:tui] Background: {} — {}", task_name, summary);
                }
                CognitiveUpdate::MetacognitiveInsight { .. } => {}

                // Build system events
                CognitiveUpdate::BuildPhaseStarted { phase, detail } => {
                    self.thinking_status = format!("Building: {}...", phase);
                    self.messages.push(Message { role: MessageRole::System,
                        content: format!("● Build({})\n  └ {}", phase, detail),
                        timestamp: timestamp.clone(), phase: Some(format!("Build: {}", phase)) });
                    self.scroll_to_bottom();
                }
                CognitiveUpdate::BuildProgress { phase, completed, total } => {
                    self.thinking_status = format!("Build {}: {}/{}", phase, completed, total);
                    self.progress = Some((format!("{} ({}/{})", phase, completed, total), completed as f64 / total.max(1) as f64));
                }
                CognitiveUpdate::BuildPhaseComplete { phase, duration_ms, summary } => {
                    self.progress = None;
                    self.messages.push(Message { role: MessageRole::System,
                        content: format!("● BuildDone({})\n  └ {} ({:.1}s)", phase, summary, duration_ms as f64 / 1000.0),
                        timestamp: timestamp.clone(), phase: Some(format!("Build: {}", phase)) });
                    self.scroll_to_bottom();
                }
                CognitiveUpdate::BuildComplete { report } => {
                    self.progress = None;
                    self.messages.push(Message { role: MessageRole::Hydra, content: report, timestamp: timestamp.clone(), phase: Some("Build".into()) });
                    self.scroll_to_bottom();
                }
                CognitiveUpdate::BuildFailed { phase, error } => {
                    self.progress = None;
                    self.messages.push(Message { role: MessageRole::System, content: format!("● BuildFailed({})\n  ✗ {}", phase, error), timestamp: timestamp.clone(), phase: Some("Build".into()) });
                    self.scroll_to_bottom();
                }
            }
        }
    }
}
