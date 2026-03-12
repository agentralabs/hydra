//! Cognitive update handler — extracted from app.rs for file size.
//! Processes CognitiveUpdate events from the cognitive loop channel.

use chrono::Local;
use tokio::sync::mpsc;
use hydra_native::cognitive::CognitiveUpdate;
use super::app::{App, Message, MessageRole, PendingApproval};

impl App {
    /// Drain all pending CognitiveUpdate events from the channel.
    /// Called every tick (250ms) to keep the TUI responsive.
    pub(crate) fn process_cognitive_updates(&mut self) {
        // Drain into a Vec to avoid borrow issues with self
        let (updates, disconnected) = {
            match self.cognitive_rx.as_mut() {
                Some(rx) => {
                    let mut buf = Vec::new();
                    let mut disc = false;
                    loop {
                        match rx.try_recv() {
                            Ok(update) => buf.push(update),
                            Err(mpsc::error::TryRecvError::Empty) => break,
                            Err(mpsc::error::TryRecvError::Disconnected) => {
                                disc = true;
                                break;
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
                CognitiveUpdate::Message { role, content, .. } => {
                    let msg_role = match role.as_str() {
                        "user" => MessageRole::User,
                        "hydra" | "assistant" => MessageRole::Hydra,
                        _ => MessageRole::System,
                    };
                    // Track conversation history for future cognitive loop calls
                    let api_role = if msg_role == MessageRole::User { "user" } else { "assistant" };
                    self.conversation_history.push((api_role.to_string(), content.clone()));

                    self.messages.push(Message {
                        role: msg_role,
                        content,
                        timestamp: timestamp.clone(),
                        phase: self.current_phase.clone(),
                    });
                    self.scroll_to_bottom();
                }
                CognitiveUpdate::ResetIdle => {
                    self.current_phase = None;
                    self.is_thinking = false;
                    self.thinking_status.clear();
                    self.thinking_elapsed_ms = 0;
                    self.progress = None;
                    self.invention_engine.reset_idle();
                }
                CognitiveUpdate::AwaitApproval { approval_id, risk_level, action, description, .. } => {
                    match risk_level.as_str() {
                        "critical" | "high" | "medium" => {
                            // Show approval prompt in TUI
                            self.pending_approval = Some(PendingApproval {
                                approval_id,
                                risk_level: risk_level.clone(),
                                action: action.clone(),
                                description: description.clone(),
                            });
                            self.messages.push(Message {
                                role: MessageRole::System,
                                content: format!(
                                    "[{} RISK] {}\n{}\n\nApprove? (y/n)",
                                    risk_level.to_uppercase(), action, description
                                ),
                                timestamp: timestamp.clone(),
                                phase: Some("Decide".to_string()),
                            });
                            self.scroll_to_bottom();
                        }
                        _ => {
                            // Low/none risk: auto-approve silently
                        }
                    }
                }

                // -- Repair events --
                CognitiveUpdate::RepairStarted { spec, task } => {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: format!("Self-repair started: {} ({})", task, spec),
                        timestamp: timestamp.clone(),
                        phase: Some("Repair".to_string()),
                    });
                    self.scroll_to_bottom();
                }
                CognitiveUpdate::RepairIteration { passed, total, .. } => {
                    // Show as progress bar, not as chat message
                    self.thinking_status = format!("Repairing... ({}/{})", passed, total);
                    self.progress = Some((
                        self.thinking_status.clone(),
                        passed as f64 / total.max(1) as f64,
                    ));
                }
                CognitiveUpdate::RepairCompleted { task, status, iterations } => {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: format!("Repair complete: {} — {} ({} iterations)", task, status, iterations),
                        timestamp: timestamp.clone(),
                        phase: Some("Repair".to_string()),
                    });
                    self.progress = None;
                    self.scroll_to_bottom();
                }

                // -- Omniscience events --
                CognitiveUpdate::OmniscienceAnalyzing { phase } => {
                    self.thinking_status = format!("Scanning: {}...", phase);
                    self.current_phase = Some(format!("Omniscience: {}", phase));
                }
                CognitiveUpdate::OmniscienceGapFound { .. } => {
                    // Individual gaps are internal detail — only show the final summary
                }
                CognitiveUpdate::OmniscienceScanComplete { gaps_found, specs_generated, health_score } => {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: format!(
                            "Omniscience scan complete: {} gaps, {} specs, {:.0}% health",
                            gaps_found, specs_generated, health_score * 100.0
                        ),
                        timestamp: timestamp.clone(),
                        phase: Some("Omniscience".to_string()),
                    });
                    self.scroll_to_bottom();
                }

                // -- Phase 3, C5.3: Phase-specific loading with elapsed time --
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
                // Hide internal step-complete messages from conversation
                CognitiveUpdate::PlanStepComplete { .. } => {}

                // Beliefs loaded — internal state, don't show in chat
                CognitiveUpdate::BeliefsLoaded { .. } => {}

                // -- Celebration --
                CognitiveUpdate::Celebrate(msg) => {
                    self.messages.push(Message {
                        role: MessageRole::Hydra,
                        content: msg,
                        timestamp: timestamp.clone(),
                        phase: None,
                    });
                    self.scroll_to_bottom();
                }

                // Sisters called — already visible in sidebar, don't pollute chat
                CognitiveUpdate::SistersCalled { .. } => {}

                // -- Proactive alerts --
                CognitiveUpdate::ProactiveAlert { title, message, priority } => {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: format!("[{}] {} — {}", priority, title, message),
                        timestamp: timestamp.clone(),
                        phase: None,
                    });
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

                // -- Silently handled / no TUI equivalent --
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
                | CognitiveUpdate::TokenUsage { .. }
                | CognitiveUpdate::StreamChunk { .. }
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

                // -- Agent Swarm events --
                CognitiveUpdate::SwarmSpawned { count, .. } => {
                    self.thinking_status = format!("Spawned {} agents", count);
                }
                CognitiveUpdate::SwarmTaskAssigned { agent_id, task_desc } => {
                    self.thinking_status = format!("Agent {} → {}", agent_id, task_desc);
                }
                CognitiveUpdate::SwarmResults { succeeded, total, failed, .. } => {
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: format!("Swarm complete: {}/{} succeeded, {} failed", succeeded, total, failed),
                        timestamp: timestamp.clone(),
                        phase: Some("Swarm".to_string()),
                    });
                    self.scroll_to_bottom();
                }

                CognitiveUpdate::ObstacleDetected { pattern, error_summary } => {
                    self.thinking_status = format!("Obstacle: {}...", pattern);
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: format!("Obstacle detected — {}: {}", pattern, error_summary),
                        timestamp: timestamp.clone(),
                        phase: self.current_phase.clone(),
                    });
                }
                CognitiveUpdate::ObstacleResolved { pattern, resolution, attempts } => {
                    self.thinking_status = "Obstacle resolved".into();
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: format!("{} — {} (attempts: {})", pattern, resolution, attempts),
                        timestamp: timestamp.clone(),
                        phase: self.current_phase.clone(),
                    });
                }
                CognitiveUpdate::ProjectExecPhase { repo, phase, detail } => {
                    self.thinking_status = format!("[{}] {}", repo, phase);
                    self.messages.push(Message {
                        role: MessageRole::System,
                        content: format!("**{}** — {}: {}", repo, phase, detail),
                        timestamp: timestamp.clone(),
                        phase: self.current_phase.clone(),
                    });
                }
            }
        }
    }
}
