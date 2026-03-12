// Single block expression returning (send_message, save_current_profile).
// Used as: let (send_message, save_current_profile) = include!("app_send_handler.rs");
{
    // ── Send message handler ──
    // Wrap Arc in Signal so the closure captures only Copy types (Signal is Copy)
    let decide_sig: Signal<Arc<DecideEngine>> = use_signal(|| decide_engine.clone());
    let inv_sig: Signal<Arc<InventionEngine>> = use_signal(|| invention_engine.clone());
    let notifier_sig: Signal<Arc<parking_lot::Mutex<ProactiveNotifier>>> = use_signal(|| proactive_notifier.clone());
    let spawner_sig: Signal<Arc<AgentSpawner>> = use_signal(|| agent_spawner.clone());
    let approval_sig: Signal<Arc<ApprovalManager>> = use_signal(|| send_msg_approval_mgr.clone());
    let db_sig: Signal<Option<Arc<HydraDb>>> = use_signal(|| hydra_db.clone());
    let fed_sig: Signal<Arc<FederationManager>> = use_signal(|| federation_manager.clone());

    let mut send_message = move |text: String| {
        let validation = validate_input(&text, 10_000);
        if !validation.valid {
            input_error.set(validation.error);
            return;
        }
        input_error.set(None);
        let text = validation.trimmed;

        // Check that at least one API key or OAuth token is available
        let has_anthropic = !settings_anthropic_key.read().is_empty();
        let has_openai = !settings_openai_key.read().is_empty();
        let has_google = !settings_google_key.read().is_empty();
        let has_oauth = {
            let (status, _, _) = oauth_status.read().clone();
            status == "authenticated"
        };
        if !has_anthropic && !has_openai && !has_google && !has_oauth {
            active_error.set(Some(FriendlyError {
                message: "No API key configured".into(),
                explanation: "Go to Settings > Models and add an Anthropic, OpenAI, or Google API key to start chatting.".into(),
                options: vec![], icon_state: "error".into(), can_undo: false,
            }));
            return;
        }

        // Pulse: cancel any in-flight TTS + fire instant spoken ack
        let pulse_ref = pulse.read().clone();
        pulse_ref.cancel_tts();
        if *settings_voice.read() {
            let ack = pulse_ref.instant_ack(&text);
            let ack_key = settings_openai_key.read().clone();
            let ack_voice = settings_tts_voice.read().clone();
            pulse_ref.reset_tts_cancel();
            let cancel = pulse_ref.tts_cancel.clone();
            if !ack_key.is_empty() {
                spawn(async move {
                    let _ = crate::pulse_voice::speak_interruptible(
                        &ack, &ack_key, &ack_voice, cancel,
                    ).await;
                });
            }
        }

        let is_first_message = messages.read().is_empty();
        messages.write().push(("user".into(), text.clone(), "message".into()));
        chat_db_sig.read().save_message("user", &text);
        connected.set(true);
        input.set(String::new());
        new_session_flash.set(false);

        // Rename active session to first message (truncated)
        if is_first_message {
            let cur_id = active_session_id.read().clone();
            let label = if text.len() > 40 { format!("{}...", &text[..37]) } else { text.clone() };
            if let Some(today) = sidebar.write().sections.first_mut() {
                for item in &mut today.items {
                    if item.id == cur_id {
                        item.label = label;
                        break;
                    }
                }
            }
        }
        let task_id = active_session_id.read().clone();
        is_typing.set(true);

        let anthropic_key_val = settings_anthropic_key.read().clone();
        let openai_key_val = settings_openai_key.read().clone();
        let google_key_val = settings_google_key.read().clone();
        let model_val = settings_model.read().clone();
        let user_name = onboarding.read().user_name.clone().unwrap_or_default();
        let sisters_handle = sisters.read().clone();

        let history: Vec<(String, String)> = messages.read().iter()
            .filter(|(role, _, _)| role == "user" || role == "hydra")
            .map(|(role, content, _)| {
                let api_role = if role == "user" { "user" } else { "assistant" };
                (api_role.to_string(), content.clone())
            })
            .collect();

        let loop_config = CognitiveLoopConfig {
            text,
            anthropic_key: anthropic_key_val,
            openai_key: openai_key_val,
            google_key: google_key_val,
            model: model_val,
            user_name,
            task_id: task_id.clone(),
            history,
            session_count: messages.read().len() as u32,
            anthropic_oauth_token: {
                let (status, _, _) = oauth_status.read().clone();
                if status == "authenticated" { AnthropicOAuth::new().access_token().map(|s| s.to_string()) } else { None }
            },
            runtime: hydra_native::RuntimeSettings {
                intent_cache: *settings_intent_cache.read(), cache_ttl: settings_cache_ttl.read().clone(),
                learn_corrections: *settings_learn_corrections.read(), belief_persist: settings_belief_persist.read().clone(),
                compression: settings_compression.read().clone(), dispatch_mode: settings_dispatch_mode.read().clone(),
                sister_timeout: settings_sister_timeout.read().clone(), retry_failures: *settings_retry_failures.read(),
                dream_state: *settings_dream_state.read(), proactive: *settings_proactive.read(),
                risk_threshold: settings_risk_threshold.read().clone(), file_write: *settings_file_write.read(),
                network_access: *settings_network_access.read(), shell_exec: *settings_shell_exec.read(),
                max_file_edits: settings_max_file_edits.read().clone(), require_approval_critical: *settings_require_approval_critical.read(),
                sandbox_mode: *settings_sandbox_mode.read(), debug_mode: *settings_debug_mode.read(),
                log_level: settings_log_level.read().clone(), federation_enabled: *settings_federation.read(),
            },
        };

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<CognitiveUpdate>();

        let decide = decide_sig.read().clone();
        let inv = inv_sig.read().clone();
        let notifier = notifier_sig.read().clone();
        let spawner = spawner_sig.read().clone();
        let approval_mgr = approval_sig.read().clone();
        let db_handle = db_sig.read().clone();
        let fed_mgr = fed_sig.read().clone();
        // Wire federation enable/disable from runtime settings
        if loop_config.runtime.federation_enabled { fed_mgr.enable(); } else { fed_mgr.disable(); }
        let swarm = Some(swarm_manager.clone());
        spawn(async move { run_cognitive_loop(loop_config, sisters_handle, tx, decide, Some(undo_sig.read().clone()), Some(inv), Some(notifier), Some(spawner), Some(approval_mgr), db_handle, Some(fed_mgr), swarm).await; });

        let chat_db_rx = chat_db_sig.read().clone();
        let pulse_rx = pulse.read().clone();
        let monitor_rx = monitor.read().clone();
        let tracer_rx = tracer.read().clone();
        tracer_rx.lock().start_trace(&task_id);
        let trace_start = std::time::Instant::now();
        spawn(async move {
            while let Some(update) = rx.recv().await {
                match update {
                    CognitiveUpdate::Phase(ref p) => {
                        // Record trace span for each phase transition
                        let span = hydra_trace::SpanBuilder::new(format!("phase-{}", p), p.clone()).finish(hydra_trace::SpanStatus::Ok);
                        tracer_rx.lock().add_span(&task_id, span);
                        monitor_rx.lock().record_metric("phase_transition", trace_start.elapsed().as_millis() as f64);
                        phase.set(p.clone());
                    }
                    CognitiveUpdate::IconState(s) => icon_state.set(s),
                    CognitiveUpdate::PhaseStatuses(s) => phase_statuses.set(s),
                    CognitiveUpdate::Typing(t) => is_typing.set(t),
                    CognitiveUpdate::PlanInit { goal, steps } => {
                        let step_refs: Vec<&str> = steps.iter().map(|s| s.as_str()).collect();
                        *plan_panel.write() = PlanPanel::new(&goal, step_refs);
                    }
                    CognitiveUpdate::PlanClear => { plan_panel.write().steps.clear(); }
                    CognitiveUpdate::PlanStepStart(idx) => { plan_panel.write().start_step(idx); }
                    CognitiveUpdate::PlanStepComplete { index, duration_ms } => {
                        if index == usize::MAX {
                            let n = plan_panel.read().steps.len();
                            if n > 0 { plan_panel.write().complete_step(n - 1, None, duration_ms); }
                        } else {
                            plan_panel.write().complete_step(index, None, duration_ms);
                        }
                    }
                    CognitiveUpdate::EvidenceClear => { evidence_panel.write().clear(); }
                    CognitiveUpdate::EvidenceMemory { title, content } => {
                        evidence_panel.write().add_memory_context(&title, &content);
                    }
                    CognitiveUpdate::EvidenceCode { title, content, language, file_path } => {
                        evidence_panel.write().add_code(&title, &content, language.as_deref(), file_path.as_deref(), None);
                    }
                    CognitiveUpdate::TimelineClear => { timeline_panel.write().clear(); }
                    CognitiveUpdate::Message { role, content, css_class } => {
                        chat_db_rx.save_message(&role, &content);
                        // Pulse: speak via interruptible TTS + learn from exchange
                        if role != "user" && *settings_voice.read() {
                            let tts_text = content.clone();
                            let tts_key = settings_openai_key.read().clone();
                            let tts_voice = settings_tts_voice.read().clone();
                            let cancel = pulse_rx.tts_cancel.clone();
                            pulse_rx.reset_tts_cancel();
                            if !tts_key.is_empty() {
                                spawn(async move {
                                    let _ = crate::pulse_voice::speak_interruptible(
                                        &tts_text, &tts_key, &tts_voice, cancel,
                                    ).await;
                                });
                            }
                        }
                        // Pulse: learn from this exchange for future prediction
                        if role != "user" {
                            if let Some(last_user) = messages.read().iter().rev()
                                .find(|(r, _, _)| r == "user")
                                .map(|(_, c, _)| c.clone())
                            {
                                pulse_rx.learn(&last_user, &content);
                            }
                        }
                        messages.write().push((role, content, css_class));
                    }
                    CognitiveUpdate::SidebarCompleteTask(id) => { sidebar.write().complete_task(&id); }
                    CognitiveUpdate::Celebrate(msg) => {
                        let statuses = phase_statuses.read().clone();
                        let total_tokens: u64 = statuses.iter().filter_map(|s| s.tokens_used).sum();
                        let total_ms: u64 = statuses.iter().filter_map(|s| s.duration_ms).sum();
                        let fmt_dur = |ms: u64| if ms >= 60_000 { format!("{:.1}m", ms as f64 / 60_000.0) } else if ms >= 1_000 { format!("{:.1}s", ms as f64 / 1_000.0) } else { format!("{}ms", ms) };
                        let fmt_tok = |t: u64| if t >= 1_000 { format!("{:.1}k", t as f64 / 1_000.0) } else { format!("{}", t) };
                        let mut phase_rows = String::new();
                        for s in &statuses {
                            let dur = s.duration_ms.map(|d| fmt_dur(d)).unwrap_or("-".into());
                            let tok = s.tokens_used.map(|t| if t > 0 { fmt_tok(t) } else { "-".into() }).unwrap_or("-".into());
                            phase_rows.push_str(&format!(r#"<div class="cs-phase-row"><span class="cs-phase-check">{}</span><span class="cs-phase-name">{:?}</span><span class="cs-phase-dur">{}</span><span class="cs-phase-tok">{}</span></div>"#, "\u{2713}", s.phase, dur, tok));
                        }
                        let summary_html = format!(r#"<div class="completion-summary"><div class="cs-header"><span class="cs-badge">Completed</span><span class="cs-title">{}</span></div><div class="cs-stats"><div class="cs-stat"><span class="cs-stat-value">{}</span><span class="cs-stat-label">Duration</span></div><div class="cs-stat"><span class="cs-stat-value">{}</span><span class="cs-stat-label">Tokens</span></div><div class="cs-stat"><span class="cs-stat-value">{}</span><span class="cs-stat-label">Phases</span></div></div><div class="cs-phases">{}</div></div>"#, msg, fmt_dur(total_ms), fmt_tok(total_tokens), statuses.len(), phase_rows);
                        messages.write().push(("system".into(), summary_html, "completion".into()));
                        celebration.set(Some(Celebration::small(&msg)));
                    }
                    CognitiveUpdate::ResetIdle => {
                        monitor_rx.lock().record_metric("loop_duration_ms", trace_start.elapsed().as_millis() as f64);
                        phase.set("Idle".into());
                        icon_state.set("idle".into());
                        is_typing.set(false);
                        _active_progress.set(None);
                        phase_statuses.set(vec![]);
                        if *settings_auto_listen.read() && *settings_voice.read() { spawn(async move { tokio::time::sleep(std::time::Duration::from_millis(500)).await; document::eval("document.querySelector('.mic-btn')?.click()"); }); }
                    }
                    CognitiveUpdate::SuggestMode(mode) => { current_mode.set(mode); }
                    CognitiveUpdate::AwaitApproval { approval_id, risk_level, action, description, challenge_phrase } => {
                        pending_approval_id.set(approval_id);
                        match risk_level.as_str() {
                            "critical" => {
                                let card = ApprovalCard::critical(&action, &description, challenge_phrase.as_deref().unwrap_or(""));
                                pending_approval.set(Some(card));
                            }
                            "high" => {
                                let card = ApprovalCard::high(&action, &description, &action);
                                pending_approval.set(Some(card));
                            }
                            "medium" => {
                                let card = ApprovalCard::medium(&action, &description);
                                pending_approval.set(Some(card));
                            }
                            _ => {
                                tracing::debug!("Auto-approved {} risk action: {}", risk_level, action);
                            }
                        }
                    }
                    CognitiveUpdate::SettingsApplied { confirmation } => { let now = chrono::Local::now().format("%H:%M:%S").to_string(); timeline_panel.write().push_event(&now, TimelineEventKind::Info, "Settings applied", Some(&confirmation), Some("Settings")); }
                    CognitiveUpdate::SistersCalled { sisters } => { if !sisters.is_empty() { let now = chrono::Local::now().format("%H:%M:%S").to_string(); timeline_panel.write().push_event(&now, TimelineEventKind::Info, &format!("Sisters: {}", sisters.join(", ")), None, Some("Act")); } }
                    CognitiveUpdate::TokenUsage { input_tokens, output_tokens } => {
                        let now = chrono::Local::now().format("%H:%M:%S").to_string();
                        timeline_panel.write().push_event(&now, TimelineEventKind::Info, &format!("Tokens: {}in + {}out", input_tokens, output_tokens), None, Some("Budget"));
                        monitor_rx.lock().record_metric("tokens_input", input_tokens as f64);
                        monitor_rx.lock().record_metric("tokens_output", output_tokens as f64);
                    }
                    CognitiveUpdate::StreamChunk { content } => {
                        // Progressive TTS: speak complete sentences as they stream in
                        if *settings_voice.read() {
                            let sentences = crate::pulse_voice::split_into_sentences(&content);
                            if let Some(sentence) = sentences.last() {
                                if sentence.len() > 10 {
                                    let tts_key = settings_openai_key.read().clone();
                                    let tts_voice = settings_tts_voice.read().clone();
                                    let cancel = pulse_rx.tts_cancel.clone();
                                    let s = sentence.clone();
                                    if !tts_key.is_empty() {
                                        spawn(async move {
                                            let _ = crate::pulse_voice::speak_interruptible(
                                                &s, &tts_key, &tts_voice, cancel,
                                            ).await;
                                        });
                                    }
                                }
                            }
                        }
                    }
                    CognitiveUpdate::StreamComplete => { pulse_rx.reset_tts_cancel(); }
                    CognitiveUpdate::UndoStatus { can_undo: cu, can_redo: cr, last_action } => { can_undo.set(cu); can_redo.set(cr); last_undo_action.set(last_action); }
                    CognitiveUpdate::ProactiveAlert { title, message, priority } => {
                        let kind = match priority.as_str() {
                            "High" => TimelineEventKind::Error,
                            "Medium" => TimelineEventKind::Info,
                            _ => TimelineEventKind::Info,
                        };
                        let now = chrono::Local::now().format("%H:%M:%S").to_string();
                        timeline_panel.write().push_event(&now, kind, &format!("[{}] {}", priority, title), Some(&message), None);
                    }
                    CognitiveUpdate::SkillCrystallized { name, actions_count } => {
                        let now = chrono::Local::now().format("%H:%M:%S").to_string();
                        timeline_panel.write().push_event(&now, TimelineEventKind::Info, &format!("Skill crystallized: {}", name), Some(&format!("{} actions learned", actions_count)), None);
                    }
                    CognitiveUpdate::ReflectionInsight { insight } => {
                        let now = chrono::Local::now().format("%H:%M:%S").to_string();
                        timeline_panel.write().push_event(&now, TimelineEventKind::Info, "Metacognition insight", Some(&insight), None);
                    }
                    CognitiveUpdate::CompressionApplied { original_tokens, compressed_tokens, ratio } => {
                        let now = chrono::Local::now().format("%H:%M:%S").to_string();
                        timeline_panel.write().push_event(&now, TimelineEventKind::Info, &format!("Token compression: {} → {}", original_tokens, compressed_tokens), Some(&format!("{:.0}% reduction", (1.0 - ratio) * 100.0)), None);
                    }
                    CognitiveUpdate::DreamInsight { category, description, confidence } => {
                        let now = chrono::Local::now().format("%H:%M:%S").to_string();
                        timeline_panel.write().push_event(&now, TimelineEventKind::Info, &format!("Dream insight ({})", category), Some(&description), None);
                    }
                    CognitiveUpdate::ShadowValidation { safe, recommendation } => {
                        let now = chrono::Local::now().format("%H:%M:%S").to_string();
                        let kind = if safe { TimelineEventKind::Info } else { TimelineEventKind::Error };
                        timeline_panel.write().push_event(&now, kind, &format!("Shadow validation: {}", if safe { "SAFE" } else { "WARNING" }), Some(&recommendation), None);
                    }
                    CognitiveUpdate::PredictionResult { action, confidence, recommendation } => {
                        let now = chrono::Local::now().format("%H:%M:%S").to_string();
                        timeline_panel.write().push_event(&now, TimelineEventKind::Info, &format!("Future Echo: {:.0}% confidence", confidence * 100.0), Some(&format!("{} — {}", recommendation, action)), None);
                    }
                    CognitiveUpdate::PatternEvolved { summary } => {
                        let now = chrono::Local::now().format("%H:%M:%S").to_string();
                        timeline_panel.write().push_event(&now, TimelineEventKind::Info, "Pattern evolution", Some(&summary), None);
                    }
                    CognitiveUpdate::TemporalStored { category, content } => { let now = chrono::Utc::now().to_rfc3339(); timeline_panel.write().push_event(&now, TimelineEventKind::Info, &format!("Memory [{}]", category), Some(&content), Some("Learn")); }
                    CognitiveUpdate::CursorMove { x, y, label } => { ghost_cursor.write().move_to(x, y, label); }
                    CognitiveUpdate::CursorClick => {
                        let gc = ghost_cursor.read();
                        let (cx, cy) = (gc.x, gc.y);
                        drop(gc);
                        ghost_cursor.write().click();
                        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64;
                        ghost_click_rings.write().push((cx, cy, now));
                        let mut gc_sig = ghost_cursor.clone();
                        spawn(async move { tokio::time::sleep(std::time::Duration::from_millis(200)).await; gc_sig.write().idle(); });
                    }
                    CognitiveUpdate::CursorTyping { active } => { if active { ghost_cursor.write().start_typing(); } else { ghost_cursor.write().idle(); } }
                    CognitiveUpdate::CursorVisibility { visible } => { if visible { ghost_cursor.write().show(); } else { ghost_cursor.write().hide(); } }
                    CognitiveUpdate::CursorModeChange { mode } => {
                        let m = match mode.as_str() { "fast" => CursorMode::Fast, "invisible" => CursorMode::Invisible, "replay" => CursorMode::Replay, _ => CursorMode::Visible };
                        ghost_cursor.write().set_mode(m);
                    }
                    CognitiveUpdate::CursorPaused { paused } => { if paused { ghost_cursor.write().pause(); } else { ghost_cursor.write().resume(); } }
                    CognitiveUpdate::BeliefsLoaded { count, summary } => { evidence_panel.write().add_memory_context(&format!("Active Beliefs ({})", count), &summary); }
                    CognitiveUpdate::BeliefUpdated { subject, confidence, is_new, .. } => { let label = if is_new { "New belief" } else { "Updated" }; let now = chrono::Utc::now().to_rfc3339(); timeline_panel.write().push_event(&now, TimelineEventKind::Info, &format!("{}: {} ({:.0}%)", label, subject, confidence * 100.0), None, Some("Learn")); }
                    CognitiveUpdate::McpSkillsDiscovered { server, tools, count } => { evidence_panel.write().add_memory_context(&format!("MCP: {} ({} tools)", server, count), &tools.join(", ")); }
                    CognitiveUpdate::FederationSync { peers_online, last_sync_version } => {
                        if peers_online > 0 { let now = chrono::Utc::now().to_rfc3339(); timeline_panel.write().push_event(&now, TimelineEventKind::Info, &format!("Federation: {} peers, sync v{}", peers_online, last_sync_version), None, Some("Perceive")); }
                    }
                    CognitiveUpdate::FederationDelegated { peer_name, task_summary } => { let now = chrono::Utc::now().to_rfc3339(); timeline_panel.write().push_event(&now, TimelineEventKind::Delegation, &format!("Delegated to {}: {}", peer_name, task_summary), None, Some("Decide")); }
                    CognitiveUpdate::RepairStarted { spec, task } => { let now = chrono::Utc::now().to_rfc3339(); timeline_panel.write().push_event(&now, TimelineEventKind::Info, &format!("Self-repair started: {} ({})", task, spec), None, Some("Repair")); }
                    CognitiveUpdate::RepairCheckResult { name, passed } => { let now = chrono::Utc::now().to_rfc3339(); let status = if passed { "PASS" } else { "FAIL" }; timeline_panel.write().push_event(&now, TimelineEventKind::Info, &format!("Repair check [{}]: {}", status, name), None, Some("Repair")); }
                    CognitiveUpdate::RepairIteration { iteration, passed, total } => { let now = chrono::Utc::now().to_rfc3339(); timeline_panel.write().push_event(&now, TimelineEventKind::Info, &format!("Repair iteration {} — {}/{} checks passed", iteration, passed, total), None, Some("Repair")); }
                    CognitiveUpdate::RepairCompleted { task, status, iterations } => { let now = chrono::Utc::now().to_rfc3339(); timeline_panel.write().push_event(&now, TimelineEventKind::Info, &format!("Repair {}: {} ({}x)", status, task, iterations), None, Some("Repair")); if status == "Success" { celebration.set(Some(Celebration::small(&format!("Self-repair: {}", task)))); } }
                    CognitiveUpdate::OmniscienceAnalyzing { phase } => { let now = chrono::Utc::now().to_rfc3339(); timeline_panel.write().push_event(&now, TimelineEventKind::Info, &format!("Omniscience: {}", phase), None, Some("Omniscience")); }
                    CognitiveUpdate::OmniscienceGapFound { description, severity, category } => { let now = chrono::Utc::now().to_rfc3339(); timeline_panel.write().push_event(&now, TimelineEventKind::Info, &format!("Gap [{}|{}]: {}", severity, category, description), None, Some("Omniscience")); }
                    CognitiveUpdate::OmniscienceSpecGenerated { spec_name, task } => { let now = chrono::Utc::now().to_rfc3339(); timeline_panel.write().push_event(&now, TimelineEventKind::Info, &format!("Forge generated spec: {} — {}", spec_name, task), None, Some("Omniscience")); }
                    CognitiveUpdate::OmniscienceValidation { spec_name, safe, recommendation } => { let now = chrono::Utc::now().to_rfc3339(); let status = if safe { "SAFE" } else { "BLOCKED" }; timeline_panel.write().push_event(&now, TimelineEventKind::Info, &format!("Aegis [{}]: {} — {}", status, spec_name, recommendation), None, Some("Omniscience")); }
                    CognitiveUpdate::OmniscienceScanComplete { gaps_found, specs_generated, health_score } => { let now = chrono::Utc::now().to_rfc3339(); timeline_panel.write().push_event(&now, TimelineEventKind::Info, &format!("Omniscience: {:.0}% health, {} gaps, {} specs", health_score * 100.0, gaps_found, specs_generated), None, Some("Omniscience")); if health_score >= 0.9 { celebration.set(Some(Celebration::small("Codebase health: excellent!"))); } }
                    CognitiveUpdate::PhaseLoading { phase: p, elapsed_ms } => { let now = chrono::Local::now().format("%H:%M:%S").to_string(); let dur = if elapsed_ms >= 1000 { format!("{:.1}s", elapsed_ms as f64 / 1000.0) } else { format!("{}ms", elapsed_ms) }; timeline_panel.write().push_event(&now, TimelineEventKind::Info, &format!("{} ({})", p, dur), None, Some(&p)); }
                    CognitiveUpdate::ConsolidationCycleComplete { cycle, strengthened, decayed, gc_cleaned } => { let now = chrono::Local::now().format("%H:%M:%S").to_string(); timeline_panel.write().push_event(&now, TimelineEventKind::Info, &format!("Consolidation #{}: +{} -{} gc:{}", cycle, strengthened, decayed, gc_cleaned), None, Some("Learn")); }
                    CognitiveUpdate::ObstacleDetected { pattern, error_summary, .. } => { let now = chrono::Utc::now().to_rfc3339(); timeline_panel.write().push_event(&now, TimelineEventKind::Info, &format!("Obstacle: {} — {}", pattern, error_summary), None, Some("Obstacle")); }
                    CognitiveUpdate::ObstacleResolved { pattern, resolution, attempts, .. } => { let now = chrono::Utc::now().to_rfc3339(); timeline_panel.write().push_event(&now, TimelineEventKind::Info, &format!("Resolved {}: {} ({}x)", pattern, resolution, attempts), None, Some("Obstacle")); }
                    CognitiveUpdate::ProjectExecPhase { repo, phase, detail, .. } => { let now = chrono::Utc::now().to_rfc3339(); timeline_panel.write().push_event(&now, TimelineEventKind::Info, &format!("[{}] {} — {}", repo, phase, detail), None, Some("ProjectExec")); }
                    CognitiveUpdate::SwarmSpawned { count, .. } => { let now = chrono::Utc::now().to_rfc3339(); timeline_panel.write().push_event(&now, TimelineEventKind::Info, &format!("Swarm: {} agents spawned", count), None, Some("Swarm")); }
                    CognitiveUpdate::SwarmTaskAssigned { agent_id, task_desc } => { let now = chrono::Utc::now().to_rfc3339(); timeline_panel.write().push_event(&now, TimelineEventKind::Info, &format!("Swarm [{}]: {}", agent_id, task_desc), None, Some("Swarm")); }
                    CognitiveUpdate::SwarmResults { total, succeeded, failed, summary } => { let now = chrono::Utc::now().to_rfc3339(); timeline_panel.write().push_event(&now, TimelineEventKind::Info, &format!("Swarm done: {}/{} ok, {} fail — {}", succeeded, total, failed, summary), None, Some("Swarm")); }
                }
            }
        });
    };

    // ── Build save profile closure ──
    let save_current_profile = move || {
        let profile = PersistedProfile {
            user_name: onboarding.read().user_name.clone(),
            voice_enabled: *settings_voice.read(),
            onboarding_complete: true,
            selected_model: Some(settings_model.read().clone()),
            api_key: None,
            anthropic_api_key: { let k = settings_anthropic_key.read().clone(); if k.is_empty() { None } else { Some(k) } },
            openai_api_key: { let k = settings_openai_key.read().clone(); if k.is_empty() { None } else { Some(k) } },
            google_api_key: { let k = settings_google_key.read().clone(); if k.is_empty() { None } else { Some(k) } },
            theme: Some(settings_theme.read().clone()),
            auto_approve: *settings_auto_approve.read(),
            default_mode: Some(settings_default_mode.read().clone()),
            sounds_enabled: *settings_sounds.read(),
            sound_volume: settings_volume.read().parse::<u8>().unwrap_or(70),
            working_directory: std::env::current_dir().ok().map(|p| p.display().to_string()),
            autonomy_level: Default::default(),
        };
        save_profile(&profile);
    };

    (send_message, save_current_profile)
}
