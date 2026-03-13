                    CognitiveUpdate::ProactiveAlert { title, message, priority } => {
                        let kind = match priority.as_str() {
                            "High" => TimelineEventKind::Error,
                            "Medium" => TimelineEventKind::Info,
                            _ => TimelineEventKind::Info,
                        };
                        let now = chrono::Local::now().format("%H:%M:%S").to_string();
                        timeline_panel.write().push_event(
                            &now,
                            kind,
                            &format!("[{}] {}", priority, title),
                            Some(&message),
                            None,
                        );
                    }
                    CognitiveUpdate::SkillCrystallized { name, actions_count } => {
                        tracing::info!("[hydra] Skill crystallized: {} ({} actions)", name, actions_count);
                        let now = chrono::Local::now().format("%H:%M:%S").to_string();
                        timeline_panel.write().push_event(
                            &now,
                            TimelineEventKind::Info,
                            &format!("Skill crystallized: {}", name),
                            Some(&format!("{} actions learned → reusable skill", actions_count)),
                            None,
                        );
                    }
                    CognitiveUpdate::ReflectionInsight { insight } => {
                        tracing::info!("[hydra] Reflection: {}", insight);
                        let now = chrono::Local::now().format("%H:%M:%S").to_string();
                        timeline_panel.write().push_event(
                            &now,
                            TimelineEventKind::Info,
                            "Metacognition insight",
                            Some(&insight),
                            None,
                        );
                    }
                    CognitiveUpdate::CompressionApplied { original_tokens, compressed_tokens, ratio } => {
                        tracing::info!("[hydra] Context compressed: {} → {} tokens ({:.0}% reduction)", original_tokens, compressed_tokens, (1.0 - ratio) * 100.0);
                        let now = chrono::Local::now().format("%H:%M:%S").to_string();
                        timeline_panel.write().push_event(
                            &now,
                            TimelineEventKind::Info,
                            &format!("Token compression: {} → {}", original_tokens, compressed_tokens),
                            Some(&format!("{:.0}% reduction", (1.0 - ratio) * 100.0)),
                            None,
                        );
                    }
                    CognitiveUpdate::DreamInsight { category, description, confidence } => {
                        tracing::info!("[hydra] Dream insight [{}]: {} ({:.0}%)", category, description, confidence * 100.0);
                        let now = chrono::Local::now().format("%H:%M:%S").to_string();
                        timeline_panel.write().push_event(
                            &now,
                            TimelineEventKind::Info,
                            &format!("Dream insight ({})", category),
                            Some(&description),
                            None,
                        );
                    }
                    CognitiveUpdate::ShadowValidation { safe, recommendation } => {
                        tracing::info!("[hydra] Shadow validation: safe={}, {}", safe, recommendation);
                        let now = chrono::Local::now().format("%H:%M:%S").to_string();
                        let kind = if safe { TimelineEventKind::Info } else { TimelineEventKind::Error };
                        timeline_panel.write().push_event(
                            &now,
                            kind,
                            &format!("Shadow validation: {}", if safe { "SAFE" } else { "WARNING" }),
                            Some(&recommendation),
                            None,
                        );
                    }
                    CognitiveUpdate::PredictionResult { action, confidence, recommendation } => {
                        tracing::info!("[hydra] Prediction: {} ({:.0}% confidence, {})", action, confidence * 100.0, recommendation);
                        let now = chrono::Local::now().format("%H:%M:%S").to_string();
                        timeline_panel.write().push_event(
                            &now,
                            TimelineEventKind::Info,
                            &format!("Future Echo: {:.0}% confidence", confidence * 100.0),
                            Some(&format!("{} — {}", recommendation, action)),
                            None,
                        );
                    }
                    CognitiveUpdate::PatternEvolved { summary } => {
                        tracing::info!("[hydra] Pattern evolution: {}", summary);
                        let now = chrono::Local::now().format("%H:%M:%S").to_string();
                        timeline_panel.write().push_event(
                            &now,
                            TimelineEventKind::Info,
                            "Pattern evolution",
                            Some(&summary),
                            None,
                        );
                    }
                    CognitiveUpdate::TemporalStored { category, content } => {
                        tracing::info!("[hydra] Temporal memory stored [{}]: {}", category, content);
                    }
                    // ── Ghost Cursor events ──
                    CognitiveUpdate::CursorMove { x, y, label } => {
                        ghost_cursor.write().move_to(x, y, label);
                    }
                    CognitiveUpdate::CursorClick => {
                        let gc = ghost_cursor.read();
                        let cx = gc.x;
                        let cy = gc.y;
                        drop(gc);
                        ghost_cursor.write().click();
                        // Add click ring effect
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64;
                        ghost_click_rings.write().push((cx, cy, now));
                        // Reset to idle after click animation
                        let mut gc_sig = ghost_cursor.clone();
                        spawn(async move {
                            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                            gc_sig.write().idle();
                        });
                    }
                    CognitiveUpdate::CursorTyping { active } => {
                        if active {
                            ghost_cursor.write().start_typing();
                        } else {
                            ghost_cursor.write().idle();
                        }
                    }
                    CognitiveUpdate::CursorVisibility { visible } => {
                        if visible {
                            ghost_cursor.write().show();
                        } else {
                            ghost_cursor.write().hide();
                        }
                    }
                    CognitiveUpdate::CursorModeChange { mode } => {
                        let m = match mode.as_str() {
                            "fast" => CursorMode::Fast,
                            "invisible" => CursorMode::Invisible,
                            "replay" => CursorMode::Replay,
                            _ => CursorMode::Visible,
                        };
                        ghost_cursor.write().set_mode(m);
                    }
                    CognitiveUpdate::CursorPaused { paused } => {
                        if paused {
                            ghost_cursor.write().pause();
                        } else {
                            ghost_cursor.write().resume();
                        }
                    }
                    CognitiveUpdate::BeliefsLoaded { count, summary } => {
                        evidence_panel.write().add_memory_context(
                            &format!("Active Beliefs ({})", count),
                            &summary,
                        );
                    }
                    CognitiveUpdate::BeliefUpdated { subject, confidence, is_new, .. } => {
                        let action = if is_new { "New belief" } else { "Updated belief" };
                        let now = chrono::Utc::now().to_rfc3339();
                        timeline_panel.write().push_event(
                            &now,
                            TimelineEventKind::Info,
                            &format!("{}: {} ({:.0}%)", action, subject, confidence * 100.0),
                            None,
                            Some("Learn"),
                        );
                    }
                    CognitiveUpdate::McpSkillsDiscovered { server, tools, count } => {
                        evidence_panel.write().add_memory_context(
                            &format!("MCP: {} ({} tools)", server, count),
                            &tools.join(", "),
                        );
                    }
                    CognitiveUpdate::FederationSync { peers_online, last_sync_version } => {
                        if peers_online > 0 {
                            let now = chrono::Utc::now().to_rfc3339();
                            timeline_panel.write().push_event(
                                &now,
                                TimelineEventKind::Info,
                                &format!("Federation: {} peers, sync v{}", peers_online, last_sync_version),
                                None,
                                Some("Perceive"),
                            );
                        }
                    }
                    CognitiveUpdate::FederationDelegated { peer_name, task_summary } => {
                        let now = chrono::Utc::now().to_rfc3339();
                        timeline_panel.write().push_event(
                            &now,
                            TimelineEventKind::Delegation,
                            &format!("Delegated to {}: {}", peer_name, task_summary),
                            None,
                            Some("Decide"),
                        );
                    }
                    CognitiveUpdate::RepairStarted { spec, task } => {
                        let now = chrono::Utc::now().to_rfc3339();
                        timeline_panel.write().push_event(
                            &now,
                            TimelineEventKind::Info,
                            &format!("Self-repair started: {} ({})", task, spec),
                            None,
                            Some("Repair"),
                        );
                    }
                    CognitiveUpdate::RepairCheckResult { name, passed } => {
                        let now = chrono::Utc::now().to_rfc3339();
                        let status = if passed { "PASS" } else { "FAIL" };
                        timeline_panel.write().push_event(
                            &now,
                            TimelineEventKind::Info,
                            &format!("Repair check [{}]: {}", status, name),
                            None,
                            Some("Repair"),
                        );
                    }
                    CognitiveUpdate::RepairIteration { iteration, passed, total } => {
                        let now = chrono::Utc::now().to_rfc3339();
                        timeline_panel.write().push_event(
                            &now,
                            TimelineEventKind::Info,
                            &format!("Repair iteration {} — {}/{} checks passed", iteration, passed, total),
                            None,
                            Some("Repair"),
                        );
                    }
                    CognitiveUpdate::RepairCompleted { task, status, iterations } => {
                        let now = chrono::Utc::now().to_rfc3339();
                        timeline_panel.write().push_event(
                            &now,
                            TimelineEventKind::Info,
                            &format!("Repair {}: {} ({} iterations)", status, task, iterations),
                            None,
                            Some("Repair"),
                        );
                        if status == "Success" {
                            celebration.set(Some(Celebration::small(&format!("Self-repair complete: {}", task))));
                        }
                    }
                    CognitiveUpdate::OmniscienceAnalyzing { phase } => {
                        let now = chrono::Utc::now().to_rfc3339();
                        timeline_panel.write().push_event(
                            &now,
                            TimelineEventKind::Info,
                            &format!("Omniscience: {}", phase),
                            None,
                            Some("Omniscience"),
                        );
                    }
                    CognitiveUpdate::OmniscienceGapFound { description, severity, category } => {
                        let now = chrono::Utc::now().to_rfc3339();
                        timeline_panel.write().push_event(
                            &now,
                            TimelineEventKind::Info,
                            &format!("Gap [{}|{}]: {}", severity, category, description),
                            None,
                            Some("Omniscience"),
                        );
                    }
                    CognitiveUpdate::OmniscienceSpecGenerated { spec_name, task } => {
                        let now = chrono::Utc::now().to_rfc3339();
                        timeline_panel.write().push_event(
                            &now,
                            TimelineEventKind::Info,
                            &format!("Forge generated spec: {} — {}", spec_name, task),
                            None,
                            Some("Omniscience"),
                        );
                    }
                    CognitiveUpdate::OmniscienceValidation { spec_name, safe, recommendation } => {
                        let now = chrono::Utc::now().to_rfc3339();
                        let status = if safe { "SAFE" } else { "BLOCKED" };
                        timeline_panel.write().push_event(
                            &now,
                            TimelineEventKind::Info,
                            &format!("Aegis [{}]: {} — {}", status, spec_name, recommendation),
                            None,
                            Some("Omniscience"),
                        );
                    }
                    CognitiveUpdate::OmniscienceScanComplete { gaps_found, specs_generated, health_score } => {
                        let now = chrono::Utc::now().to_rfc3339();
                        let health_pct = (health_score * 100.0) as u32;
                        timeline_panel.write().push_event(
                            &now,
                            TimelineEventKind::Info,
                            &format!("Omniscience complete: {}% health, {} gaps, {} specs generated", health_pct, gaps_found, specs_generated),
                            None,
                            Some("Omniscience"),
                        );
                        if health_score >= 0.9 {
                            celebration.set(Some(Celebration::small("Codebase health: excellent!")));
                        }
                    }
                    // ── Agent Swarm events ──
                    CognitiveUpdate::SwarmSpawned { count, agent_ids } => {
                        let now = chrono::Utc::now().to_rfc3339();
                        let ids_preview: String = agent_ids.iter().take(3)
                            .map(|id| &id[..id.len().min(8)]).collect::<Vec<_>>().join(", ");
                        timeline_panel.write().push_event(
                            &now, TimelineEventKind::Info,
                            &format!("Swarm: {} agents spawned", count),
                            Some(&ids_preview), Some("Swarm"),
                        );
                    }
                    CognitiveUpdate::SwarmTaskAssigned { agent_id, task_desc } => {
                        let now = chrono::Utc::now().to_rfc3339();
                        timeline_panel.write().push_event(
                            &now, TimelineEventKind::Info,
                            &format!("Agent {} assigned", agent_id),
                            Some(&task_desc), Some("Swarm"),
                        );
                    }
                    CognitiveUpdate::SwarmResults { total, succeeded, failed, .. } => {
                        let now = chrono::Utc::now().to_rfc3339();
                        let kind = if failed > 0 { TimelineEventKind::Error } else { TimelineEventKind::Info };
                        timeline_panel.write().push_event(
                            &now, kind,
                            &format!("Swarm results: {}/{} succeeded", succeeded, total),
                            Some(&format!("{} failed", failed)), Some("Swarm"),
                        );
                    }
                    // ── Agentic Loop events ──
                    CognitiveUpdate::AgenticTurn { turn, tool_count, exec_count } => {
                        let now = chrono::Local::now().format("%H:%M:%S").to_string();
                        timeline_panel.write().push_event(
                            &now,
                            TimelineEventKind::Info,
                            &format!("Agentic turn {} — {} tools, {} commands", turn, tool_count, exec_count),
                            None,
                            Some("Act"),
                        );
                    }
                    CognitiveUpdate::AgenticComplete { turns, total_tokens, stop_reason } => {
                        let now = chrono::Local::now().format("%H:%M:%S").to_string();
                        timeline_panel.write().push_event(
                            &now,
                            TimelineEventKind::Info,
                            &format!("Agentic loop: {} turns, {} tokens", turns, total_tokens),
                            Some(&format!("Reason: {}", stop_reason)),
                            Some("Act"),
                        );
                    }
                    // -- Phases 2-7: Superintelligence Pipeline --
                    CognitiveUpdate::VerificationApplied { checked, corrected } => {
                        if corrected > 0 {
                            let now = chrono::Local::now().format("%H:%M:%S").to_string();
                            timeline_panel.write().push_event(
                                &now, TimelineEventKind::Info,
                                &format!("Verified: {}/{} claims corrected", corrected, checked),
                                None, Some("Verify"),
                            );
                        }
                    }
                    CognitiveUpdate::ModelEscalated { ref from, ref to, ref reason } => {
                        let now = chrono::Local::now().format("%H:%M:%S").to_string();
                        timeline_panel.write().push_event(
                            &now, TimelineEventKind::Info,
                            &format!("Model escalated: {} → {}", from, to),
                            Some(reason), Some("Think"),
                        );
                    }
                    CognitiveUpdate::BackgroundTaskComplete { ref task_name, ref summary } => {
                        let now = chrono::Local::now().format("%H:%M:%S").to_string();
                        timeline_panel.write().push_event(
                            &now, TimelineEventKind::Info,
                            &format!("Background: {}", task_name),
                            Some(summary), Some("Background"),
                        );
                    }
                    CognitiveUpdate::ProactiveFileSuggestion { ref title, ref message, ref priority, ref action } => {
                        let now = chrono::Local::now().format("%H:%M:%S").to_string();
                        let kind = match priority.as_str() {
                            "Urgent" => TimelineEventKind::Error,
                            _ => TimelineEventKind::Info,
                        };
                        let detail = action.as_ref().map(|a| format!("{} ({})", message, a)).unwrap_or_else(|| message.clone());
                        timeline_panel.write().push_event(
                            &now, kind, title, Some(&detail), Some("Watcher"),
                        );
                    }
                    CognitiveUpdate::MetacognitiveInsight { ref assessment } => {
                        let now = chrono::Local::now().format("%H:%M:%S").to_string();
                        timeline_panel.write().push_event(
                            &now, TimelineEventKind::Info,
                            "Metacognitive insight",
                            Some(assessment), Some("Learn"),
                        );
                    }
                    // -- Build System + Tool Action events --
                    CognitiveUpdate::BuildPhaseStarted { ref phase, ref detail } => { self.thinking_status = format!("Building: {}...", phase); let now = chrono::Local::now().format("%H:%M:%S").to_string(); timeline_panel.write().push_event(&now, TimelineEventKind::Info, &format!("Build: {}", phase), Some(detail), Some("Build")); }
                    CognitiveUpdate::BuildProgress { ref phase, completed, total } => { self.thinking_status = format!("Build {}: {}/{}", phase, completed, total); }
                    CognitiveUpdate::BuildPhaseComplete { ref phase, duration_ms, ref summary } => { let now = chrono::Local::now().format("%H:%M:%S").to_string(); timeline_panel.write().push_event(&now, TimelineEventKind::Info, &format!("{} ({:.1}s)", phase, duration_ms as f64 / 1000.0), Some(summary), Some("Build")); }
                    CognitiveUpdate::BuildComplete { ref report } => { let now = chrono::Local::now().format("%H:%M:%S").to_string(); timeline_panel.write().push_event(&now, TimelineEventKind::Info, "Build complete", Some(report), Some("Build")); }
                    CognitiveUpdate::BuildFailed { ref phase, ref error } => { let now = chrono::Local::now().format("%H:%M:%S").to_string(); timeline_panel.write().push_event(&now, TimelineEventKind::Error, &format!("Build failed: {}", phase), Some(error), Some("Build")); }
                    CognitiveUpdate::ToolAction { .. } => {}
