//! Hydra Desktop — Fresh build. Claude Desktop quality.

use dioxus::prelude::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

mod voice_capture;

// Data structures from hydra-native (tested, working)
use hydra_native::components::onboarding::{OnboardingState, OnboardingStep};
use hydra_native::components::approval::ApprovalCard;
use hydra_native::components::progress::{Celebration, ProgressJourney};
use hydra_native::components::error_display::FriendlyError;
use hydra_native::components::sidebar::Sidebar;
use hydra_native::components::input::validate_input;
use hydra_native::components::phases::{build_phase_dots, build_connectors};
use hydra_native::components::command_palette::CommandPalette;
use hydra_native::state::hydra::PhaseStatus;
use hydra_native::components::plan_panel::{PlanPanel, StepStatus};
use hydra_native::components::timeline_panel::{TimelinePanel, TimelineEventKind};
use hydra_native::components::evidence_panel::EvidencePanel;
use hydra_native::profile::{load_profile, save_profile, PersistedProfile};
use hydra_native::sisters::SistersHandle;
use hydra_native::cognitive::{CognitiveLoopConfig, CognitiveUpdate, DecideEngine, InventionEngine, AgentSpawner, run_cognitive_loop};
use hydra_native::proactive::ProactiveNotifier;
use hydra_model::oauth::AnthropicOAuth;
use hydra_native::persistence::ChatPersistence;
use hydra_native::utils::markdown::markdown_to_html;
use hydra_native::components::globe::{globe_params, globe_svg, derive_globe_state, GlobeSize};
use hydra_native::components::ghost_cursor::{GhostCursorState, CursorMode, cursor_svg};
use hydra_db::HydraDb;
use hydra_runtime::approval::{ApprovalDecision, ApprovalManager};
use hydra_runtime::undo::UndoStack;
use hydra_native::federation::FederationManager;

const CSS: &str = include_str!("styles.css");

fn main() {
    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            dioxus::desktop::Config::new()
                .with_window(
                    dioxus::desktop::WindowBuilder::new()
                        .with_title("Hydra")
                        .with_inner_size(dioxus::desktop::LogicalSize::new(1200.0, 800.0))
                        .with_min_inner_size(dioxus::desktop::LogicalSize::new(800.0, 600.0)),
                ),
        )
        .launch(App);
}

#[allow(non_snake_case)]
fn App() -> Element {
    // ── Load persisted profile ──
    let persisted = load_profile();
    let onboarding_done = persisted.as_ref().map_or(false, |p| p.onboarding_complete);

    let chat_db = Arc::new(ChatPersistence::init().unwrap_or_else(|e| {
        eprintln!("[hydra] Chat persistence failed: {}", e);
        ChatPersistence::init().expect("chat persistence fallback failed")
    }));

    // ── Init graduated autonomy engine ──
    let decide_engine: Arc<DecideEngine> = use_hook(|| Arc::new(DecideEngine::new()));

    let invention_engine: Arc<InventionEngine> = use_hook(|| {
        let engine = Arc::new(InventionEngine::new());
        let inv = engine.clone();
        spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                inv.tick_idle(10);
                if let Some(dream_insights) = inv.maybe_dream() {
                    tracing::info!("[hydra] Dream insights: {}", dream_insights);
                }
            }
        });
        engine
    });
    let proactive_notifier: Arc<parking_lot::Mutex<ProactiveNotifier>> =
        use_hook(|| Arc::new(parking_lot::Mutex::new(ProactiveNotifier::new())));
    let agent_spawner: Arc<AgentSpawner> = use_hook(|| Arc::new(AgentSpawner::new(100)));
    let undo_stack: Arc<parking_lot::Mutex<UndoStack>> = use_hook(|| Arc::new(parking_lot::Mutex::new(UndoStack::new(100))));
    let approval_manager: Arc<ApprovalManager> = use_hook(|| Arc::new(ApprovalManager::with_default_timeout()));
    let federation_manager: Arc<FederationManager> = use_hook(|| Arc::new(FederationManager::new()));
    // ── Initialize security database ──
    let hydra_db: Option<Arc<HydraDb>> = use_hook(|| {
        let db_path = std::env::var("HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join(".hydra")
            .join("security.db");
        match HydraDb::init(&db_path) {
            Ok(db) => {
                tracing::info!("[hydra] Security DB initialized at {:?}", db_path);
                Some(Arc::new(db))
            }
            Err(e) => {
                tracing::warn!("[hydra] Failed to init security DB: {}", e);
                None
            }
        }
    });

    // Clone for each UI consumer that captures approval_manager
    let send_msg_approval_mgr = approval_manager.clone();
    let palette_approval_mgr = approval_manager.clone();
    let card_approval_mgr = approval_manager.clone();

    // ── Init sisters (MCP connections) ──
    let sisters: Signal<Option<SistersHandle>> = use_signal(|| None);
    let sisters_status = use_signal(|| "Connecting...".to_string());
    {
        let mut sisters = sisters.clone();
        let mut sisters_status = sisters_status.clone();
        use_hook(move || {
            spawn(async move {
                let handle = hydra_native::sisters::init_sisters().await;
                let status = handle.status_summary();
                sisters.set(Some(handle));
                sisters_status.set(status);
            });
        });
    }

    // ── Extract settings from profile ──
    let init_theme = persisted.as_ref().and_then(|p| p.theme.clone()).unwrap_or_else(|| "dark".to_string());
    let init_voice = persisted.as_ref().map_or(false, |p| p.voice_enabled);
    let init_sounds = persisted.as_ref().map_or(true, |p| p.sounds_enabled);
    let init_volume = persisted.as_ref().map_or("70".to_string(), |p| p.sound_volume.to_string());
    let init_auto_approve = persisted.as_ref().map_or(false, |p| p.auto_approve);
    let init_default_mode = persisted.as_ref().and_then(|p| p.default_mode.clone()).unwrap_or_else(|| "companion".to_string());
    let init_model = persisted.as_ref().and_then(|p| p.selected_model.clone()).unwrap_or_else(|| "claude-sonnet-4-6".to_string());
    let init_anthropic_key = persisted.as_ref()
        .and_then(|p| p.anthropic_api_key.clone())
        .or_else(|| persisted.as_ref().and_then(|p| p.api_key.clone()).filter(|k| k.starts_with("sk-ant-")))
        .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok().filter(|s| !s.is_empty()))
        .unwrap_or_default();
    let init_openai_key = persisted.as_ref()
        .and_then(|p| p.openai_api_key.clone())
        .or_else(|| persisted.as_ref().and_then(|p| p.api_key.clone()).filter(|k| k.starts_with("sk-") && !k.starts_with("sk-ant-")))
        .or_else(|| std::env::var("OPENAI_API_KEY").ok().filter(|s| !s.is_empty()))
        .unwrap_or_default();
    let init_google_key = persisted.as_ref()
        .and_then(|p| p.google_api_key.clone())
        .or_else(|| std::env::var("GOOGLE_API_KEY").ok().filter(|s| !s.is_empty()))
        .unwrap_or_default();

    // ── Core state signals ──
    let mut input = use_signal(|| String::new());
    let chat_db_init = chat_db.clone();
    let mut messages = use_signal(move || chat_db_init.load_messages());
    let chat_db_sig: Signal<Arc<ChatPersistence>> = use_signal(|| chat_db.clone());
    let mut connected = use_signal(|| false);
    let mut phase = use_signal(|| "Idle".to_string());
    let mut icon_state = use_signal(|| "idle".to_string());

    // ── Onboarding ──
    let mut onboarding = use_signal(move || {
        if let Some(ref p) = persisted {
            let mut state = OnboardingState::new();
            if let Some(ref name) = p.user_name { state.set_name(name); }
            if p.voice_enabled { state.enable_voice(); }
            if p.onboarding_complete { state.advance(); state.advance(); state.advance(); }
            state
        } else {
            OnboardingState::new()
        }
    });
    let mut show_onboarding = use_signal(move || !onboarding_done);

    // ── Settings state ──
    let mut show_settings = use_signal(|| false);
    let mut settings_theme = use_signal(move || init_theme);
    let mut settings_voice = use_signal(move || init_voice);
    let mut settings_sounds = use_signal(move || init_sounds);
    let mut settings_volume = use_signal(move || init_volume);
    let mut settings_auto_approve = use_signal(move || init_auto_approve);
    let init_mode_for_current = init_default_mode.clone();
    let init_mode_for_sidebar = init_default_mode.clone();
    let mut settings_default_mode = use_signal(move || init_default_mode);
    let mut settings_model = use_signal(move || init_model);
    let mut settings_anthropic_key = use_signal(move || init_anthropic_key);
    let mut settings_openai_key = use_signal(move || init_openai_key);
    let mut settings_google_key = use_signal(move || init_google_key);
    let mut settings_tab = use_signal(|| "general".to_string());

    // ── Anthropic OAuth state ──
    let mut oauth_status = use_signal(|| {
        let oauth = AnthropicOAuth::new();
        if oauth.is_authenticated() {
            let email = oauth.account_email().unwrap_or("").to_string();
            let tier = oauth.subscription_tier().unwrap_or("").to_string();
            ("authenticated".to_string(), email, tier)
        } else {
            ("not_authenticated".to_string(), String::new(), String::new())
        }
    });
    let mut oauth_loading = use_signal(|| false);

    // ── Mode state ──
    let mut current_mode = use_signal(move || init_mode_for_current);

    // ── Sidebar ──
    let mut sidebar = use_signal(|| {
        let mut sb = Sidebar::new();
        sb.add_task("session-1", "Session 1");
        sb
    });
    let mut show_sidebar = use_signal(move || init_mode_for_sidebar == "workspace");

    // ── Approval, progress, error ──
    let mut pending_approval = use_signal(|| Option::<ApprovalCard>::None);
    let mut pending_approval_id = use_signal(|| Option::<String>::None);
    let mut challenge_input = use_signal(|| String::new());

    // ── Ghost Cursor state ──
    let mut ghost_cursor = use_signal(|| GhostCursorState::new());
    let mut ghost_click_rings: Signal<Vec<(f64, f64, u64)>> = use_signal(|| Vec::new());
    let mut _active_progress = use_signal(|| Option::<ProgressJourney>::None);
    let mut celebration = use_signal(|| Option::<Celebration>::None);
    let mut celebration_dismiss_scheduled = use_signal(|| false);
    let mut active_error = use_signal(|| Option::<FriendlyError>::None);
    let mut approval_countdown = use_signal(|| 0u32);

    // ── UI state ──
    let mut show_features = use_signal(|| false);
    let mut is_typing = use_signal(|| false);
    let mut phase_statuses = use_signal(|| Vec::<PhaseStatus>::new());
    let mut input_error = use_signal(|| Option::<String>::None);
    let mut show_command_palette = use_signal(|| false);
    let mut command_palette = use_signal(CommandPalette::new);
    let mut session_counter = use_signal(|| 1u32);
    let mut new_session_flash = use_signal(|| false);
    let mut active_session_id = use_signal(|| "session-1".to_string());
    let mut voice_listening = use_signal(|| false);
    let mut mic_stop_flag = use_signal(|| Arc::new(AtomicBool::new(false)));
    let mut show_search = use_signal(|| false);
    let mut search_query = use_signal(|| String::new());
    let mut show_receipts = use_signal(|| false);
    // Store messages per session: session_id -> Vec<(role, content, css)>
    let mut session_store = use_signal(|| HashMap::<String, Vec<(String, String, String)>>::new());

    // ── Undo/Redo state ──
    let mut can_undo = use_signal(|| false);
    let mut can_redo = use_signal(|| false);
    let mut last_undo_action = use_signal(|| Option::<String>::None);
    let undo_sig: Signal<Arc<parking_lot::Mutex<UndoStack>>> = use_signal(|| undo_stack.clone());

    // ── Workspace panels ──
    let mut plan_panel = use_signal(|| PlanPanel::default());
    let mut timeline_panel = use_signal(|| TimelinePanel::new());
    let mut evidence_panel = use_signal(|| EvidencePanel::new());

    // ── Global keyboard shortcuts via JS document listener ──
    let kb_approval_mgr = approval_manager.clone();
    use_hook(|| {
        spawn(async move {
            let mut eval = document::eval(r#"
                (async function() {
                    var queue = [];
                    document.addEventListener('keydown', function(e) {
                        if (e.metaKey || e.ctrlKey) {
                            var k = e.key.toLowerCase();
                            if (e.shiftKey && k === 'k') {
                                e.preventDefault();
                                queue.push('shift+k');
                            } else if (['k','b','n','f',',','z','1','2','3','4'].indexOf(k) !== -1) {
                                e.preventDefault();
                                queue.push(k);
                            }
                        }
                        if (e.key === 'Escape') {
                            queue.push('escape');
                        }
                    });
                    while (true) {
                        if (queue.length > 0) {
                            dioxus.send(queue.shift());
                        }
                        await new Promise(r => setTimeout(r, 16));
                    }
                })()
            "#);
            loop {
                match eval.recv::<String>().await {
                    Ok(key) => {
                        match key.as_str() {
                            "shift+k" => {
                                // Kill switch — emergency halt all activity
                                kb_approval_mgr.cancel_all();
                                is_typing.set(false);
                                phase.set("Idle".into());
                                icon_state.set("idle".into());
                                pending_approval.set(None);
                                pending_approval_id.set(None);
                                phase_statuses.set(vec![]);
                                active_error.set(Some(FriendlyError {
                                    message: "Kill Switch Activated".into(),
                                    explanation: "All operations halted. Press Escape to dismiss, or Cmd+N for a fresh session.".into(),
                                    options: vec![],
                                    icon_state: "error".into(),
                                    can_undo: false,
                                }));
                            }
                            "k" => {
                                let current = *show_command_palette.read();
                                show_command_palette.set(!current);
                                if !current { command_palette.write().reset(); }
                            }
                            "b" => {
                                let c = *show_sidebar.read();
                                show_sidebar.set(!c);
                            }
                            "," => {
                                show_settings.set(true);
                            }
                            "n" => {
                                // Save current session
                                let cur_id = active_session_id.read().clone();
                                let cur_msgs = messages.read().clone();
                                session_store.write().insert(cur_id, cur_msgs);
                                // Complete active tasks
                                let task_ids: Vec<String> = sidebar.read().today_items().iter()
                                    .filter(|item| item.active)
                                    .map(|item| item.id.clone())
                                    .collect();
                                for id in task_ids {
                                    sidebar.write().complete_task(&id);
                                }
                                // Create new DB conversation
                                let _conv_id = chat_db_sig.read().new_conversation();
                                // Create new session
                                let count = *session_counter.read() + 1;
                                session_counter.set(count);
                                let new_id = format!("session-{}", count);
                                sidebar.write().add_task(&new_id, &format!("Session {}", count));
                                active_session_id.set(new_id);
                                messages.write().clear();
                                phase.set("Idle".into());
                                icon_state.set("idle".into());
                                is_typing.set(false);
                                connected.set(false);
                                input.set(String::new());
                                plan_panel.write().steps.clear();
                                timeline_panel.write().clear();
                                evidence_panel.write().clear();
                                pending_approval.set(None);
                                pending_approval_id.set(None);
                                celebration.set(None);
                                active_error.set(None);
                                phase_statuses.set(vec![]);
                                challenge_input.set(String::new());
                                new_session_flash.set(true);
                            }
                            "f" => {
                                show_search.set(true);
                                search_query.set(String::new());
                            }
                            "1" => { current_mode.set("companion".into()); }
                            "2" => { current_mode.set("workspace".into()); show_sidebar.set(true); }
                            "3" => { current_mode.set("immersive".into()); }
                            "4" => { current_mode.set("invisible".into()); }
                            "z" => {
                                let stack = undo_sig.read();
                                let mut s = stack.lock();
                                if s.can_undo() {
                                    let _ = s.undo();
                                    can_undo.set(s.can_undo());
                                    can_redo.set(s.can_redo());
                                    last_undo_action.set(s.last_action_description().map(String::from));
                                }
                            }
                            "escape" => {
                                show_command_palette.set(false);
                                show_settings.set(false);
                                show_features.set(false);
                                show_receipts.set(false);
                                if *show_search.read() {
                                    show_search.set(false);
                                    search_query.set(String::new());
                                }
                            }
                            _ => {}
                        }
                    }
                    Err(_) => break,
                }
            }
        });
    });

    // ── Greeting ──
    let user_name = onboarding.read().user_name.clone().unwrap_or_default();
    let greeting = if user_name.is_empty() { "Hi there!".to_string() } else { format!("Hi {}!", user_name) };

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
            anthropic_oauth_token: {
                let (status, _, _) = oauth_status.read().clone();
                if status == "authenticated" {
                    AnthropicOAuth::new().access_token().map(|s| s.to_string())
                } else {
                    None
                }
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
        spawn(async move { run_cognitive_loop(loop_config, sisters_handle, tx, decide, Some(undo_sig.read().clone()), Some(inv), Some(notifier), Some(spawner), Some(approval_mgr), db_handle, Some(fed_mgr)).await; });

        let chat_db_rx = chat_db_sig.read().clone();
        spawn(async move {
            while let Some(update) = rx.recv().await {
                match update {
                    CognitiveUpdate::Phase(p) => phase.set(p),
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
                        messages.write().push((role, content, css_class));
                    }
                    CognitiveUpdate::SidebarCompleteTask(id) => { sidebar.write().complete_task(&id); }
                    CognitiveUpdate::Celebrate(msg) => { celebration.set(Some(Celebration::small(&msg))); }
                    CognitiveUpdate::ResetIdle => {
                        phase.set("Idle".into());
                        icon_state.set("idle".into());
                        is_typing.set(false);
                        _active_progress.set(None);
                        phase_statuses.set(vec![]);
                    }
                    CognitiveUpdate::SuggestMode(mode) => { current_mode.set(mode); }
                    CognitiveUpdate::AwaitApproval { approval_id, risk_level, action, description, challenge_phrase } => {
                        // Store the approval ID so buttons can submit decisions back
                        pending_approval_id.set(approval_id);
                        // Only show approval card for medium+ risk actions.
                        // None/low risk actions proceed silently — no user interruption.
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
                                // none/low risk: auto-approve silently, no UI interruption
                                tracing::debug!("Auto-approved {} risk action: {}", risk_level, action);
                            }
                        }
                    }
                    CognitiveUpdate::SettingsApplied { .. } => {}
                    CognitiveUpdate::SistersCalled { .. } => {}
                    CognitiveUpdate::TokenUsage { .. } => {}
                    CognitiveUpdate::StreamChunk { .. } => {}
                    CognitiveUpdate::StreamComplete => {}
                    CognitiveUpdate::UndoStatus { can_undo: cu, can_redo: cr, last_action } => {
                        can_undo.set(cu);
                        can_redo.set(cr);
                        last_undo_action.set(last_action);
                    }
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
        };
        save_profile(&profile);
    };

    // ══════════════════════════════════════════════
    //  RSX — every class name matches styles.css
    // ══════════════════════════════════════════════

    rsx! {
        style { {CSS} }

        // Root — theme applied via CSS class (reactive, no script needed)
        div {
            class: {
                let theme = settings_theme.read().clone();
                let theme_class = match theme.as_str() {
                    "light" => "app-root theme-light",
                    "system" => "app-root theme-system",
                    _ => "app-root",
                };
                theme_class.to_string()
            },

            // ══ Onboarding overlay ══
            if *show_onboarding.read() {
                div {
                    class: "onboarding-overlay",
                    div {
                        class: "onboarding-card",
                        div { class: "onboarding-globe" }
                        {
                            let ob = onboarding.read();
                            let view = ob.current_view();
                            rsx! {
                                h2 { class: "onboarding-title", "{view.title}" }
                                p { class: "onboarding-subtitle", "{view.subtitle}" }
                            }
                        }
                        match onboarding.read().step {
                            OnboardingStep::Intro => rsx! {
                                button {
                                    class: "btn-primary",
                                    onclick: move |_| { onboarding.write().advance(); },
                                    "Continue"
                                }
                            },
                            OnboardingStep::AskName => rsx! {
                                input {
                                    class: "onboarding-input",
                                    placeholder: "Your name...",
                                    value: "{input}",
                                    oninput: move |e| input.set(e.value()),
                                    onkeypress: move |e| {
                                        if e.key() == Key::Enter && !input.read().trim().is_empty() {
                                            onboarding.write().set_name(input.read().trim());
                                            onboarding.write().advance();
                                            input.set(String::new());
                                        }
                                    }
                                }
                                button {
                                    class: "btn-primary",
                                    onclick: move |_| {
                                        if !input.read().trim().is_empty() {
                                            onboarding.write().set_name(input.read().trim());
                                            onboarding.write().advance();
                                            input.set(String::new());
                                        }
                                    },
                                    "Continue"
                                }
                            },
                            OnboardingStep::AskApiKey => rsx! {
                                input {
                                    class: "onboarding-input",
                                    placeholder: "sk-ant-api03-... or sk-...",
                                    value: "{input}",
                                    oninput: move |e| input.set(e.value()),
                                    onkeypress: move |e| {
                                        if e.key() == Key::Enter && !input.read().trim().is_empty() {
                                            onboarding.write().set_api_key(input.read().trim());
                                            onboarding.write().advance();
                                            input.set(String::new());
                                        }
                                    }
                                }
                                div {
                                    class: "onboarding-buttons",
                                    button {
                                        class: "btn-primary",
                                        onclick: move |_| {
                                            if !input.read().trim().is_empty() {
                                                onboarding.write().set_api_key(input.read().trim());
                                                onboarding.write().advance();
                                                input.set(String::new());
                                            }
                                        },
                                        "Continue"
                                    }
                                    button {
                                        class: "btn-secondary",
                                        onclick: move |_| { onboarding.write().advance(); },
                                        "Skip for now"
                                    }
                                }
                            },
                            OnboardingStep::AskVoice => rsx! {
                                div {
                                    class: "onboarding-buttons",
                                    button {
                                        class: "btn-primary",
                                        onclick: move |_| { onboarding.write().enable_voice(); onboarding.write().advance(); },
                                        "Yes, enable"
                                    }
                                    button {
                                        class: "btn-secondary",
                                        onclick: move |_| { onboarding.write().advance(); },
                                        "Maybe later"
                                    }
                                }
                            },
                            OnboardingStep::Complete => rsx! {
                                button {
                                    class: "btn-primary",
                                    onclick: move |_| {
                                        save_current_profile();
                                        show_onboarding.set(false);
                                    },
                                    "Get started"
                                }
                            },
                        }
                        // Step dots (5 steps: Intro, AskName, AskApiKey, AskVoice, Complete)
                        div {
                            class: "step-dots",
                            {
                                let step = onboarding.read().step;
                                let steps = [
                                    OnboardingStep::Intro,
                                    OnboardingStep::AskName,
                                    OnboardingStep::AskApiKey,
                                    OnboardingStep::AskVoice,
                                    OnboardingStep::Complete,
                                ];
                                let current_idx = steps.iter().position(|s| *s == step).unwrap_or(0);
                                let c0: &str = if current_idx == 0 { "step-dot active" } else { "step-dot done" };
                                let c1: &str = if current_idx < 1 { "step-dot" } else if current_idx == 1 { "step-dot active" } else { "step-dot done" };
                                let c2: &str = if current_idx < 2 { "step-dot" } else if current_idx == 2 { "step-dot active" } else { "step-dot done" };
                                let c3: &str = if current_idx < 3 { "step-dot" } else if current_idx == 3 { "step-dot active" } else { "step-dot done" };
                                let c4: &str = if current_idx < 4 { "step-dot" } else { "step-dot active" };
                                rsx! {
                                    div { class: c0 }
                                    div { class: c1 }
                                    div { class: c2 }
                                    div { class: c3 }
                                    div { class: c4 }
                                }
                            }
                        }
                    }
                }
            }

            // ══ Command Palette ══
            if *show_command_palette.read() {
                div {
                    class: "command-palette-overlay",
                    onclick: move |_| show_command_palette.set(false),
                    div {
                        class: "command-palette",
                        onclick: move |e: MouseEvent| e.stop_propagation(),
                        input {
                            class: "command-palette-input",
                            placeholder: "Type a command...",
                            value: "{command_palette.read().query}",
                            autofocus: true,
                            oninput: move |e| command_palette.write().set_query(&e.value()),
                            onkeydown: move |e| {
                                match e.key() {
                                    Key::ArrowUp => { e.prevent_default(); command_palette.write().select_up(); }
                                    Key::ArrowDown => { e.prevent_default(); command_palette.write().select_down(); }
                                    Key::Enter => {
                                        e.prevent_default();
                                        let id = command_palette.read().selected_command_id();
                                        if let Some(cmd_id) = id {
                                            command_palette.write().record_usage(&cmd_id);
                                            show_command_palette.set(false);
                                            match cmd_id.as_str() {
                                                "toggle-sidebar" => { let c = *show_sidebar.read(); show_sidebar.set(!c); }
                                                "open-settings" => show_settings.set(true),
                                                "mode-companion" => current_mode.set("companion".into()),
                                                "mode-workspace" => { current_mode.set("workspace".into()); show_sidebar.set(true); }
                                                "mode-immersive" => current_mode.set("immersive".into()),
                                                "mode-invisible" => current_mode.set("invisible".into()),
                                                "clear-chat" => messages.write().clear(),
                                                "view-features" => show_features.set(true),
                                                "toggle-kill-switch" => {
                                                    palette_approval_mgr.cancel_all();
                                                    is_typing.set(false);
                                                    phase.set("Idle".into());
                                                    icon_state.set("idle".into());
                                                    pending_approval.set(None);
                                                    pending_approval_id.set(None);
                                                    phase_statuses.set(vec![]);
                                                    active_error.set(Some(FriendlyError {
                                                        message: "Kill Switch Activated".into(),
                                                        explanation: "All operations halted. Press Escape to dismiss.".into(),
                                                        options: vec![],
                                                        icon_state: "error".into(),
                                                        can_undo: false,
                                                    }));
                                                }
                                                "search-messages" => {
                                                    show_search.set(true);
                                                    search_query.set(String::new());
                                                }
                                                "view-receipts" => {
                                                    show_receipts.set(true);
                                                }
                                                "new-session" => {
                                                    // Save current session
                                                    let cur_id = active_session_id.read().clone();
                                                    let cur_msgs = messages.read().clone();
                                                    session_store.write().insert(cur_id, cur_msgs);
                                                    let task_ids: Vec<String> = sidebar.read().today_items().iter()
                                                        .filter(|item| item.active)
                                                        .map(|item| item.id.clone())
                                                        .collect();
                                                    for id in task_ids {
                                                        sidebar.write().complete_task(&id);
                                                    }
                                                    // Create new DB conversation
                                                    let _conv_id = chat_db_sig.read().new_conversation();
                                                    let count = *session_counter.read() + 1;
                                                    session_counter.set(count);
                                                    let new_id = format!("session-{}", count);
                                                    sidebar.write().add_task(&new_id, &format!("Session {}", count));
                                                    active_session_id.set(new_id);
                                                    messages.write().clear();
                                                    phase.set("Idle".into());
                                                    icon_state.set("idle".into());
                                                    is_typing.set(false);
                                                    connected.set(false);
                                                    input.set(String::new());
                                                    plan_panel.write().steps.clear();
                                                    timeline_panel.write().clear();
                                                    evidence_panel.write().clear();
                                                    pending_approval.set(None);
                                                    celebration.set(None);
                                                    active_error.set(None);
                                                    phase_statuses.set(vec![]);
                                                    challenge_input.set(String::new());
                                                    new_session_flash.set(true);
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                    Key::Escape => show_command_palette.set(false),
                                    _ => {}
                                }
                            },
                        }
                        div {
                            class: "command-palette-results",
                            {
                                let cp = command_palette.read();
                                let filtered = cp.filtered();
                                let selected = cp.selected_index;
                                rsx! {
                                    for (idx, cmd) in filtered.iter().enumerate() {
                                        div {
                                            class: if idx == selected { "command-item selected" } else { "command-item" },
                                            span { class: "command-label", "{cmd.label}" }
                                            if let Some(ref shortcut) = cmd.shortcut {
                                                span { class: "command-shortcut", "{shortcut}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ══ Features overlay ══
            if *show_features.read() {
                div {
                    class: "features-overlay",
                    onclick: move |_| show_features.set(false),
                    div {
                        class: "features-card",
                        onclick: move |e: MouseEvent| e.stop_propagation(),
                        h2 { class: "onboarding-title", "Features & Capabilities" }
                        p { class: "features-subtitle", "All Hydra systems and inventions" }

                        div { class: "feature-section",
                            h3 { class: "feature-section-title", "Cognitive Inventions" }
                            div { class: "feature-grid",
                                div { class: "feature-chip active", "Dream State" }
                                div { class: "feature-chip active", "Shadow Self" }
                                div { class: "feature-chip active", "Resurrection" }
                                div { class: "feature-chip active", "Token Minimizer" }
                                div { class: "feature-chip active", "Future Echo" }
                                div { class: "feature-chip active", "Mutation Engine" }
                                div { class: "feature-chip active", "Forking" }
                            }
                        }
                        div { class: "feature-section",
                            h3 { class: "feature-section-title", "Agent Swarm & Federation" }
                            div { class: "feature-grid",
                                div { class: "feature-chip active", "Peer Discovery" }
                                div { class: "feature-chip active", "Task Delegation" }
                                div { class: "feature-chip active", "Skill Sharing" }
                                div { class: "feature-chip active", "Load Balancing" }
                                div { class: "feature-chip", "Multi-Instance Sync" }
                            }
                        }
                        div { class: "feature-section",
                            h3 { class: "feature-section-title", "Safety & Control" }
                            div { class: "feature-grid",
                                div { class: "feature-chip active", "Execution Gate" }
                                div { class: "feature-chip active", "Kill Switch" }
                                div { class: "feature-chip active", "Risk Assessment" }
                                div { class: "feature-chip active", "Boundary Enforcer" }
                                div { class: "feature-chip active", "Challenge Phrases" }
                            }
                        }
                        div { class: "feature-section",
                            h3 { class: "feature-section-title", "Sisters & Skills" }
                            div { class: "feature-grid",
                                div { class: "feature-chip active", "Memory Bridge" }
                                div { class: "feature-chip active", "Vision Bridge" }
                                div { class: "feature-chip active", "Codebase Bridge" }
                                div { class: "feature-chip active", "Identity Bridge" }
                                div { class: "feature-chip active", "Skill Registry" }
                            }
                        }

                        button {
                            class: "btn-primary",
                            style: "margin-top: 16px; width: 100%;",
                            onclick: move |_| show_features.set(false),
                            "Close"
                        }
                    }
                }
            }

            // ══ Receipts overlay ══
            if *show_receipts.read() {
                div {
                    class: "overlay",
                    div {
                        class: "overlay-panel",
                        style: "max-width: 640px;",
                        h2 { class: "overlay-title", "Receipt Audit Log" }
                        div {
                            class: "receipts-list",
                            if messages.read().is_empty() {
                                p { class: "receipts-empty", "No actions recorded yet. Send a message to start." }
                            }
                            for (i, (role, content, _)) in messages.read().iter().enumerate() {
                                {
                                    let role_label = if role == "user" { "You" } else { "Hydra" };
                                    let preview = if content.len() > 100 { format!("{}...", &content[..97]) } else { content.clone() };
                                    rsx! {
                                        div {
                                            class: "receipt-row",
                                            key: "{i}",
                                            span { class: "receipt-index", "#{i}" }
                                            span { class: "receipt-role", "{role_label}" }
                                            span { class: "receipt-preview", "{preview}" }
                                        }
                                    }
                                }
                            }
                        }
                        button {
                            class: "btn-primary",
                            style: "margin-top: 16px; width: 100%;",
                            onclick: move |_| show_receipts.set(false),
                            "Close"
                        }
                    }
                }
            }

            // ══ Main layout ══
            div {
                class: if *show_sidebar.read() { "app-layout with-sidebar" } else { "app-layout" },

                // ── Sidebar ──
                if *show_sidebar.read() {
                    div {
                        class: "sidebar",
                        div {
                            class: "sidebar-header",
                            span { class: "sidebar-brand", "Hydra" }
                            button {
                                class: "sidebar-new-btn",
                                title: "New Session (Cmd+N)",
                                onclick: move |_| {
                                    // Save current session messages
                                    let cur_id = active_session_id.read().clone();
                                    let cur_msgs = messages.read().clone();
                                    session_store.write().insert(cur_id, cur_msgs);
                                    // Complete active sidebar tasks
                                    let task_ids: Vec<String> = sidebar.read().today_items().iter()
                                        .filter(|item| item.active)
                                        .map(|item| item.id.clone())
                                        .collect();
                                    for id in task_ids {
                                        sidebar.write().complete_task(&id);
                                    }
                                    // Create new DB conversation
                                    let _conv_id = chat_db_sig.read().new_conversation();
                                    // Create new session
                                    let count = *session_counter.read() + 1;
                                    session_counter.set(count);
                                    let new_id = format!("session-{}", count);
                                    sidebar.write().add_task(&new_id, &format!("Session {}", count));
                                    active_session_id.set(new_id);
                                    // Reset state for fresh session
                                    messages.write().clear();
                                    phase.set("Idle".into());
                                    icon_state.set("idle".into());
                                    is_typing.set(false);
                                    connected.set(false);
                                    input.set(String::new());
                                    plan_panel.write().steps.clear();
                                    timeline_panel.write().clear();
                                    evidence_panel.write().clear();
                                    pending_approval.set(None);
                                    celebration.set(None);
                                    active_error.set(None);
                                    phase_statuses.set(vec![]);
                                    challenge_input.set(String::new());
                                    new_session_flash.set(true);
                                },
                                "+"
                            }
                        }
                        div {
                            class: "sidebar-sessions",
                            div {
                                class: "sidebar-section-title",
                                onclick: move |_| sidebar.write().toggle_section(0),
                                span { "Today" }
                                {
                                    let ch = if sidebar.read().sections.first().map_or(false, |s| s.collapsed) { "\u{25B8}" } else { "\u{25BE}" };
                                    rsx! { span { class: "sidebar-chevron", "{ch}" } }
                                }
                            }
                            if !sidebar.read().sections.first().map_or(true, |s| s.collapsed) {
                                for item in sidebar.read().today_items().iter() {
                                    {
                                        let item_id = item.id.clone();
                                        let is_done = item.icon == "\u{2713}";
                                        let label = item.label.clone();
                                        let current_session = active_session_id.read().clone();
                                        let is_current = item_id == current_session;
                                        let item_class = if is_current { "sidebar-item active" } else { "sidebar-item" };
                                        let dot_class = if is_current { "sidebar-dot pulse" } else if is_done { "sidebar-dot done" } else { "sidebar-dot" };
                                        rsx! {
                                            div {
                                                class: item_class.to_string(),
                                                onclick: {
                                                    let switch_id = item_id.clone();
                                                    move |_| {
                                                        let cur_id = active_session_id.read().clone();
                                                        if switch_id == cur_id { return; }
                                                        // Save current session
                                                        let cur_msgs = messages.read().clone();
                                                        session_store.write().insert(cur_id.clone(), cur_msgs);
                                                        // Deactivate old, activate new in sidebar
                                                        sidebar.write().complete_task(&cur_id);
                                                        // Switch DB conversation (switch_id maps to a conversation)
                                                        let db_msgs = chat_db_sig.read().switch_conversation(&switch_id);
                                                        // Load target session messages (prefer DB, fall back to in-memory)
                                                        let stored = if !db_msgs.is_empty() {
                                                            db_msgs
                                                        } else {
                                                            session_store.read().get(&switch_id).cloned().unwrap_or_default()
                                                        };
                                                        *messages.write() = stored.clone();
                                                        connected.set(!stored.is_empty());
                                                        active_session_id.set(switch_id.clone());
                                                        // Reset transient state
                                                        phase.set("Idle".into());
                                                        icon_state.set("idle".into());
                                                        is_typing.set(false);
                                                        input.set(String::new());
                                                        pending_approval.set(None);
                                                        new_session_flash.set(false);
                                                    }
                                                },
                                                span { class: dot_class.to_string() }
                                                span { class: "sidebar-item-label", "{label}" }
                                                if is_current {
                                                    span { class: "sidebar-item-badge", "Active" }
                                                }
                                                // Archive & delete buttons (not on active session)
                                                if !is_current {
                                                    div {
                                                        class: "sidebar-item-actions",
                                                        button {
                                                            class: "sidebar-action-btn",
                                                            title: "Archive",
                                                            onclick: {
                                                                let archive_id = item_id.clone();
                                                                move |e: Event<MouseData>| {
                                                                    e.stop_propagation();
                                                                    sidebar.write().archive_task(&archive_id);
                                                                }
                                                            },
                                                            span { class: "action-icon icon-archive" }
                                                        }
                                                        button {
                                                            class: "sidebar-action-btn delete",
                                                            title: "Delete",
                                                            onclick: {
                                                                let delete_id = item_id.clone();
                                                                move |e: Event<MouseData>| {
                                                                    e.stop_propagation();
                                                                    sidebar.write().remove_task(&delete_id);
                                                                    session_store.write().remove(&delete_id);
                                                                }
                                                            },
                                                            span { class: "action-icon icon-delete" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            // History section
                            if !sidebar.read().history_items().is_empty() {
                                div {
                                    class: "sidebar-section-title",
                                    onclick: move |_| sidebar.write().toggle_section(1),
                                    span { "History" }
                                    {
                                        let ch = if sidebar.read().sections.get(1).map_or(true, |s| s.collapsed) { "\u{25B8}" } else { "\u{25BE}" };
                                        rsx! { span { class: "sidebar-chevron", "{ch}" } }
                                    }
                                }
                                if !sidebar.read().sections.get(1).map_or(true, |s| s.collapsed) {
                                    for item in sidebar.read().history_items().iter() {
                                        {
                                            let item_id = item.id.clone();
                                            let label = item.label.clone();
                                            rsx! {
                                                div {
                                                    class: "sidebar-item history",
                                                    onclick: {
                                                        let switch_id = item_id.clone();
                                                        move |_| {
                                                            // Save current, switch to archived
                                                            let cur_id = active_session_id.read().clone();
                                                            let cur_msgs = messages.read().clone();
                                                            session_store.write().insert(cur_id.clone(), cur_msgs);
                                                            sidebar.write().complete_task(&cur_id);
                                                            // Switch DB conversation
                                                            let db_msgs = chat_db_sig.read().switch_conversation(&switch_id);
                                                            let stored = if !db_msgs.is_empty() {
                                                                db_msgs
                                                            } else {
                                                                session_store.read().get(&switch_id).cloned().unwrap_or_default()
                                                            };
                                                            *messages.write() = stored.clone();
                                                            connected.set(!stored.is_empty());
                                                            active_session_id.set(switch_id.clone());
                                                            phase.set("Idle".into());
                                                            icon_state.set("idle".into());
                                                            is_typing.set(false);
                                                            input.set(String::new());
                                                            pending_approval.set(None);
                                                            new_session_flash.set(false);
                                                        }
                                                    },
                                                    span { class: "sidebar-dot done" }
                                                    span { class: "sidebar-item-label", "{label}" }
                                                    div {
                                                        class: "sidebar-item-actions",
                                                        button {
                                                            class: "sidebar-action-btn delete",
                                                            title: "Delete",
                                                            onclick: {
                                                                let del_id = item_id.clone();
                                                                move |e: Event<MouseData>| {
                                                                    e.stop_propagation();
                                                                    sidebar.write().remove_task(&del_id);
                                                                    session_store.write().remove(&del_id);
                                                                }
                                                            },
                                                            span { class: "action-icon icon-delete" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        div {
                            class: "sidebar-footer",
                            button {
                                class: "sidebar-settings-btn",
                                onclick: move |_| {
                                    // Launch TUI in a new terminal window
                                    // Try to find hydra-cli binary, fall back to cargo run
                                    let hydra_cli = std::env::var("HOME")
                                        .map(|h| format!("{}/.cargo/bin/hydra-cli", h))
                                        .unwrap_or_default();

                                    let cmd = if std::path::Path::new(&hydra_cli).exists() {
                                        hydra_cli
                                    } else {
                                        // Fall back to cargo run from the project dir
                                        // Use env!() at compile time for the project root
                                        let project_root = env!("CARGO_MANIFEST_DIR")
                                            .trim_end_matches("/crates/hydra-desktop");
                                        format!("cd '{}' && cargo run -q --bin hydra-cli", project_root)
                                    };

                                    #[cfg(target_os = "macos")]
                                    {
                                        let _ = std::process::Command::new("osascript")
                                            .args([
                                                "-e",
                                                &format!(
                                                    "tell application \"Terminal\" to do script \"{}\"",
                                                    cmd.replace('\\', "\\\\").replace('"', "\\\"")
                                                ),
                                            ])
                                            .spawn();
                                    }
                                    #[cfg(not(target_os = "macos"))]
                                    {
                                        let _ = std::process::Command::new("sh")
                                            .args(["-c", &format!("x-terminal-emulator -e '{}' &", cmd)])
                                            .spawn();
                                    }
                                },
                                title: "Open Terminal (TUI)",
                                ">"  // terminal icon
                            }
                            div {
                                class: "sidebar-status",
                                div { class: if *connected.read() { "status-dot connected" } else { "status-dot" } }
                                span { class: "sidebar-status-text", if *connected.read() { "Connected" } else { "Ready" } }
                            }
                        }
                    }
                }

                // ── Main content ──
                div {
                    class: "main-content",

                    // ── Topbar ──
                    div {
                        class: "topbar",
                        div {
                            class: "topbar-left",
                            if !*show_sidebar.read() {
                                span { class: "topbar-brand", "Hydra" }
                            }
                            span { class: "topbar-mode", "{current_mode}" }
                        }
                        div {
                            class: "topbar-center",
                            {
                                let statuses = phase_statuses.read();
                                if !statuses.is_empty() {
                                    let dots = build_phase_dots(&statuses);
                                    let connectors = build_connectors(&statuses);
                                    rsx! {
                                        div {
                                            class: "phase-dots",
                                            for (idx, dot) in dots.iter().enumerate() {
                                                if idx > 0 {
                                                    {
                                                        let conn = &connectors[idx - 1];
                                                        rsx! {
                                                            div { class: if conn.active { "phase-connector active" } else { "phase-connector" } }
                                                        }
                                                    }
                                                }
                                                div {
                                                    title: "{dot.label}",
                                                    class: format!("phase-dot {}", dot.css_class),
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    rsx! {}
                                }
                            }
                            {
                                let p = phase.read();
                                let label = match p.as_str() {
                                    "Perceive" | "Think" | "Decide" | "Act" => "Working...",
                                    "Learn" => "Finishing up...",
                                    "Done" => "Done",
                                    "Error" => "Error",
                                    "Idle" => "",
                                    other => other,
                                };
                                if !label.is_empty() {
                                    rsx! { span { class: "topbar-phase-label", "{label}" } }
                                } else {
                                    rsx! {}
                                }
                            }
                        }
                        div {
                            class: "topbar-right",
                            button {
                                class: "topbar-cmd-btn",
                                title: "Command Palette (Cmd+K)",
                                onclick: move |_| { command_palette.write().reset(); show_command_palette.set(true); },
                                "\u{2318}K"
                            }
                            button {
                                class: "topbar-icon-btn",
                                title: "Toggle Sidebar (Cmd+B)",
                                onclick: move |_| { let c = *show_sidebar.read(); show_sidebar.set(!c); },
                                "\u{2630}"
                            }
                            button {
                                class: "topbar-icon-btn",
                                title: "Settings (Cmd+,)",
                                onclick: move |_| { let c = *show_settings.read(); show_settings.set(!c); },
                                "\u{2699}"
                            }
                        }
                    }

                    // ── Content: Settings OR Chat ──
                    if *show_settings.read() {
                        // ╔══════════════════════════════════════╗
                        // ║  SETTINGS PAGE — Claude Desktop      ║
                        // ╚══════════════════════════════════════╝
                        div {
                            class: "settings-page",
                            // Left nav
                            div {
                                class: "settings-nav",
                                {
                                    let tabs: Vec<(&str, &str, &str)> = vec![
                                        ("general", "\u{2699}", "General"),
                                        ("models", "\u{2B21}", "Models"),
                                        ("sisters", "\u{2726}", "Sisters"),
                                        ("voice", "\u{266A}", "Voice"),
                                        ("policies", "\u{26E8}", "Policies"),
                                        ("behavior", "\u{2699}", "Behavior"),
                                        ("advanced", "\u{2318}", "Advanced"),
                                    ];
                                    let current_tab = settings_tab.read().clone();
                                    rsx! {
                                        for (id, icon, label) in tabs.iter() {
                                            button {
                                                class: if current_tab == *id { "settings-nav-item active" } else { "settings-nav-item" },
                                                onclick: {
                                                    let tab_id = id.to_string();
                                                    move |_| settings_tab.set(tab_id.clone())
                                                },
                                                span { class: "settings-nav-icon", "{icon}" }
                                                "{label}"
                                            }
                                        }
                                    }
                                }
                            }
                            // Right body
                            div {
                                class: "settings-body",
                                match settings_tab.read().as_str() {
                                    "general" => rsx! {
                                        h2 { class: "settings-title", "General" }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "Appearance" }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Theme" }
                                                div { class: "segmented-control",
                                                    button {
                                                        class: if *settings_theme.read() == "dark" { "segment active" } else { "segment" },
                                                        onclick: move |_| settings_theme.set("dark".into()),
                                                        "Dark"
                                                    }
                                                    button {
                                                        class: if *settings_theme.read() == "light" { "segment active" } else { "segment" },
                                                        onclick: move |_| settings_theme.set("light".into()),
                                                        "Light"
                                                    }
                                                    button {
                                                        class: if *settings_theme.read() == "system" { "segment active" } else { "segment" },
                                                        onclick: move |_| settings_theme.set("system".into()),
                                                        "System"
                                                    }
                                                }
                                            }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Default Mode" }
                                                div { class: "segmented-control",
                                                    button {
                                                        class: if *settings_default_mode.read() == "companion" { "segment active" } else { "segment" },
                                                        onclick: move |_| settings_default_mode.set("companion".into()),
                                                        "Companion"
                                                    }
                                                    button {
                                                        class: if *settings_default_mode.read() == "workspace" { "segment active" } else { "segment" },
                                                        onclick: move |_| settings_default_mode.set("workspace".into()),
                                                        "Workspace"
                                                    }
                                                }
                                            }
                                        }
                                    },
                                    "models" => rsx! {
                                        h2 { class: "settings-title", "Models" }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "Anthropic" }
                                            div { class: "model-grid",
                                                {
                                                    let models: Vec<(&str, &str)> = vec![("claude-sonnet-4-6", "Sonnet 4.6"), ("claude-opus-4-6", "Opus 4.6"), ("claude-haiku-4-5", "Haiku 4.5")];
                                                    rsx! {
                                                        for (id, label) in models.iter() {
                                                            button {
                                                                class: if *settings_model.read() == *id { "model-card active" } else { "model-card" },
                                                                onclick: { let m = id.to_string(); move |_| settings_model.set(m.clone()) },
                                                                "{label}"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            div { class: "key-input-row",
                                                input {
                                                    class: "key-input",
                                                    r#type: "password",
                                                    placeholder: "Anthropic API Key (sk-ant-...)",
                                                    value: "{settings_anthropic_key}",
                                                    oninput: move |e| settings_anthropic_key.set(e.value()),
                                                }
                                                if !settings_anthropic_key.read().is_empty() {
                                                    span { class: "key-check", "\u{2713}" }
                                                }
                                            }
                                            // ── OR: Sign in with Anthropic (OAuth / Claude Max subscription) ──
                                            div { class: "oauth-section", style: "margin-top: 12px; padding: 12px; border: 1px solid rgba(255,255,255,0.1); border-radius: 8px; background: rgba(255,255,255,0.03);",
                                                {
                                                    let (status, email, tier) = oauth_status.read().clone();
                                                    if status == "authenticated" {
                                                        rsx! {
                                                            div { style: "display: flex; align-items: center; gap: 8px;",
                                                                span { style: "color: #4ade80; font-size: 14px;", "\u{2713} Signed in via Anthropic" }
                                                                if !email.is_empty() {
                                                                    span { style: "color: rgba(255,255,255,0.5); font-size: 12px;", "({email})" }
                                                                }
                                                                if !tier.is_empty() {
                                                                    span { style: "color: #a78bfa; font-size: 12px; font-weight: 600;", "{tier}" }
                                                                }
                                                            }
                                                            button {
                                                                class: "btn-secondary",
                                                                style: "margin-top: 8px; font-size: 12px; padding: 4px 12px;",
                                                                onclick: move |_| {
                                                                    let mut oauth = AnthropicOAuth::new();
                                                                    oauth.logout();
                                                                    oauth_status.set(("not_authenticated".to_string(), String::new(), String::new()));
                                                                },
                                                                "Sign Out"
                                                            }
                                                        }
                                                    } else {
                                                        rsx! {
                                                            p { style: "color: rgba(255,255,255,0.6); font-size: 13px; margin: 0 0 8px 0;",
                                                                "Or use your Claude Pro/Max subscription ($200/mo credits):"
                                                            }
                                                            button {
                                                                class: "btn-primary",
                                                                style: "width: 100%; padding: 10px; font-size: 14px; font-weight: 600; border-radius: 6px; cursor: pointer;",
                                                                disabled: *oauth_loading.read(),
                                                                onclick: move |_| {
                                                                    oauth_loading.set(true);
                                                                    spawn(async move {
                                                                        let mut oauth = AnthropicOAuth::new();
                                                                        match oauth.login().await {
                                                                            Ok(()) => {
                                                                                let email = oauth.account_email().unwrap_or("").to_string();
                                                                                let tier = oauth.subscription_tier().unwrap_or("").to_string();
                                                                                oauth_status.set(("authenticated".to_string(), email, tier));
                                                                            }
                                                                            Err(e) => {
                                                                                eprintln!("[hydra:oauth] Login failed: {}", e);
                                                                                oauth_status.set(("failed".to_string(), e, String::new()));
                                                                            }
                                                                        }
                                                                        oauth_loading.set(false);
                                                                    });
                                                                },
                                                                if *oauth_loading.read() {
                                                                    "Waiting for browser..."
                                                                } else {
                                                                    "Sign in with Anthropic"
                                                                }
                                                            }
                                                            if status == "failed" {
                                                                p { style: "color: #ef4444; font-size: 12px; margin-top: 6px;",
                                                                    "Auth failed: {email}"
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "OpenAI" }
                                            div { class: "model-grid",
                                                {
                                                    let models: Vec<(&str, &str)> = vec![("gpt-4o", "GPT-4o"), ("gpt-4o-mini", "GPT-4o Mini")];
                                                    rsx! {
                                                        for (id, label) in models.iter() {
                                                            button {
                                                                class: if *settings_model.read() == *id { "model-card active" } else { "model-card" },
                                                                onclick: { let m = id.to_string(); move |_| settings_model.set(m.clone()) },
                                                                "{label}"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            div { class: "key-input-row",
                                                input {
                                                    class: "key-input",
                                                    r#type: "password",
                                                    placeholder: "OpenAI API Key (sk-...)",
                                                    value: "{settings_openai_key}",
                                                    oninput: move |e| settings_openai_key.set(e.value()),
                                                }
                                                if !settings_openai_key.read().is_empty() {
                                                    span { class: "key-check", "\u{2713}" }
                                                }
                                            }
                                        }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "Google" }
                                            div { class: "model-grid",
                                                button {
                                                    class: if *settings_model.read() == "gemini-2.0-flash" { "model-card active" } else { "model-card" },
                                                    onclick: move |_| settings_model.set("gemini-2.0-flash".into()),
                                                    "Gemini Flash"
                                                }
                                            }
                                            div { class: "key-input-row",
                                                input {
                                                    class: "key-input",
                                                    r#type: "password",
                                                    placeholder: "Google API Key",
                                                    value: "{settings_google_key}",
                                                    oninput: move |e| settings_google_key.set(e.value()),
                                                }
                                                if !settings_google_key.read().is_empty() {
                                                    span { class: "key-check", "\u{2713}" }
                                                }
                                            }
                                        }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "Local" }
                                            div { class: "model-grid",
                                                button {
                                                    class: if *settings_model.read() == "ollama" { "model-card active" } else { "model-card" },
                                                    onclick: move |_| settings_model.set("ollama".into()),
                                                    "Ollama"
                                                }
                                            }
                                            p { class: "settings-info", "No key needed \u{2014} runs locally" }
                                        }
                                        p { class: "settings-info", "Keys saved to ~/.hydra/profile.json. Also detected from environment variables." }
                                    },
                                    "sisters" => rsx! {
                                        h2 { class: "settings-title", "Sisters & MCP" }
                                        div { class: "settings-section",
                                            p { class: "settings-info", style: "margin-bottom: 16px;",
                                                "Hydra connects to 14 sister agents via MCP (Model Context Protocol). Each sister is a specialized AI tool server."
                                            }
                                            {
                                                let sh = sisters.read();
                                                // Build full 14-sister list: (name, category, connected, tool_count)
                                                let sister_list: Vec<(&str, &str, bool, usize)> = vec![
                                                    // Foundation Sisters (7)
                                                    ("Memory", "Foundation", sh.as_ref().map_or(false, |s| s.memory.is_some()),
                                                     sh.as_ref().and_then(|s| s.memory.as_ref()).map(|m| m.tools.len()).unwrap_or(0)),
                                                    ("Identity", "Foundation", sh.as_ref().map_or(false, |s| s.identity.is_some()),
                                                     sh.as_ref().and_then(|s| s.identity.as_ref()).map(|m| m.tools.len()).unwrap_or(0)),
                                                    ("Codebase", "Foundation", sh.as_ref().map_or(false, |s| s.codebase.is_some()),
                                                     sh.as_ref().and_then(|s| s.codebase.as_ref()).map(|m| m.tools.len()).unwrap_or(0)),
                                                    ("Vision", "Foundation", sh.as_ref().map_or(false, |s| s.vision.is_some()),
                                                     sh.as_ref().and_then(|s| s.vision.as_ref()).map(|m| m.tools.len()).unwrap_or(0)),
                                                    ("Comm", "Foundation", sh.as_ref().map_or(false, |s| s.comm.is_some()),
                                                     sh.as_ref().and_then(|s| s.comm.as_ref()).map(|m| m.tools.len()).unwrap_or(0)),
                                                    ("Contract", "Foundation", sh.as_ref().map_or(false, |s| s.contract.is_some()),
                                                     sh.as_ref().and_then(|s| s.contract.as_ref()).map(|m| m.tools.len()).unwrap_or(0)),
                                                    ("Time", "Foundation", sh.as_ref().map_or(false, |s| s.time.is_some()),
                                                     sh.as_ref().and_then(|s| s.time.as_ref()).map(|m| m.tools.len()).unwrap_or(0)),
                                                    // Cognitive Sisters (3)
                                                    ("Planning", "Cognitive", sh.as_ref().map_or(false, |s| s.planning.is_some()),
                                                     sh.as_ref().and_then(|s| s.planning.as_ref()).map(|m| m.tools.len()).unwrap_or(0)),
                                                    ("Cognition", "Cognitive", sh.as_ref().map_or(false, |s| s.cognition.is_some()),
                                                     sh.as_ref().and_then(|s| s.cognition.as_ref()).map(|m| m.tools.len()).unwrap_or(0)),
                                                    ("Reality", "Cognitive", sh.as_ref().map_or(false, |s| s.reality.is_some()),
                                                     sh.as_ref().and_then(|s| s.reality.as_ref()).map(|m| m.tools.len()).unwrap_or(0)),
                                                    // Astral Sisters (4)
                                                    ("Forge", "Astral", sh.as_ref().map_or(false, |s| s.forge.is_some()),
                                                     sh.as_ref().and_then(|s| s.forge.as_ref()).map(|m| m.tools.len()).unwrap_or(0)),
                                                    ("Aegis", "Astral", sh.as_ref().map_or(false, |s| s.aegis.is_some()),
                                                     sh.as_ref().and_then(|s| s.aegis.as_ref()).map(|m| m.tools.len()).unwrap_or(0)),
                                                    ("Veritas", "Astral", sh.as_ref().map_or(false, |s| s.veritas.is_some()),
                                                     sh.as_ref().and_then(|s| s.veritas.as_ref()).map(|m| m.tools.len()).unwrap_or(0)),
                                                    ("Evolve", "Astral", sh.as_ref().map_or(false, |s| s.evolve.is_some()),
                                                     sh.as_ref().and_then(|s| s.evolve.as_ref()).map(|m| m.tools.len()).unwrap_or(0)),
                                                ];
                                                let total: usize = sister_list.iter().map(|(_, _, _, t)| *t).sum();
                                                let connected_count = sister_list.iter().filter(|(_, _, c, _)| *c).count();
                                                rsx! {
                                                    div { class: "sisters-total", "{connected_count}/14 sisters connected \u{00B7} {total} total tools" }
                                                    // Foundation
                                                    h3 { class: "settings-section-title", style: "margin-top: 16px;", "Foundation Sisters" }
                                                    div { class: "sisters-grid",
                                                        for (name, cat, conn, tools) in sister_list.iter().filter(|(_, c, _, _)| *c == "Foundation") {
                                                            div {
                                                                class: "sister-card",
                                                                div { class: "sister-card-header",
                                                                    div { class: if *conn { "status-dot connected" } else { "status-dot" } }
                                                                    span { class: "sister-name", "{name}" }
                                                                }
                                                                {
                                                                    let _ = cat;
                                                                    let status_text = if *conn { format!("{} tools", tools) } else { "offline".to_string() };
                                                                    rsx! { span { class: "sister-tools", "{status_text}" } }
                                                                }
                                                            }
                                                        }
                                                    }
                                                    // Cognitive
                                                    h3 { class: "settings-section-title", style: "margin-top: 16px;", "Cognitive Sisters" }
                                                    div { class: "sisters-grid",
                                                        for (name, _, conn, tools) in sister_list.iter().filter(|(_, c, _, _)| *c == "Cognitive") {
                                                            div {
                                                                class: "sister-card",
                                                                div { class: "sister-card-header",
                                                                    div { class: if *conn { "status-dot connected" } else { "status-dot" } }
                                                                    span { class: "sister-name", "{name}" }
                                                                }
                                                                {
                                                                    let status_text = if *conn { format!("{} tools", tools) } else { "offline".to_string() };
                                                                    rsx! { span { class: "sister-tools", "{status_text}" } }
                                                                }
                                                            }
                                                        }
                                                    }
                                                    // Astral
                                                    h3 { class: "settings-section-title", style: "margin-top: 16px;", "Astral Sisters" }
                                                    div { class: "sisters-grid",
                                                        for (name, _, conn, tools) in sister_list.iter().filter(|(_, c, _, _)| *c == "Astral") {
                                                            div {
                                                                class: "sister-card",
                                                                div { class: "sister-card-header",
                                                                    div { class: if *conn { "status-dot connected" } else { "status-dot" } }
                                                                    span { class: "sister-name", "{name}" }
                                                                }
                                                                {
                                                                    let status_text = if *conn { format!("{} tools", tools) } else { "offline".to_string() };
                                                                    rsx! { span { class: "sister-tools", "{status_text}" } }
                                                                }
                                                            }
                                                        }
                                                    }
                                                    p { class: "settings-info", style: "margin-top: 16px;",
                                                        "External MCP servers can be added via ~/.hydra/mcp.json"
                                                    }
                                                }
                                            }
                                        }
                                    },
                                    "voice" => rsx! {
                                        h2 { class: "settings-title", "Voice & Audio" }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "Voice" }
                                            div { class: "settings-row",
                                                div { class: "settings-label-group",
                                                    span { class: "settings-label", "Voice Mode" }
                                                    span { class: "settings-desc", "Enable speech-to-text and text-to-speech" }
                                                }
                                                div { class: "toggle-group",
                                                    button {
                                                        class: if *settings_voice.read() { "toggle-track on" } else { "toggle-track" },
                                                        onclick: move |_| { let c = *settings_voice.read(); settings_voice.set(!c); },
                                                        span { class: "toggle-knob" }
                                                    }
                                                    {
                                                        let label = if *settings_voice.read() { "On" } else { "Off" };
                                                        rsx! { span { class: "toggle-label", "{label}" } }
                                                    }
                                                }
                                            }
                                            div { class: "settings-row",
                                                div { class: "settings-label-group",
                                                    span { class: "settings-label", "Sound Effects" }
                                                    span { class: "settings-desc", "Play sounds for notifications and events" }
                                                }
                                                div { class: "toggle-group",
                                                    button {
                                                        class: if *settings_sounds.read() { "toggle-track on" } else { "toggle-track" },
                                                        onclick: move |_| { let c = *settings_sounds.read(); settings_sounds.set(!c); },
                                                        span { class: "toggle-knob" }
                                                    }
                                                    {
                                                        let label = if *settings_sounds.read() { "On" } else { "Off" };
                                                        rsx! { span { class: "toggle-label", "{label}" } }
                                                    }
                                                }
                                            }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Volume" }
                                                input {
                                                    class: "settings-slider",
                                                    r#type: "range",
                                                    min: "0", max: "100",
                                                    value: "{settings_volume}",
                                                    oninput: move |e| settings_volume.set(e.value()),
                                                }
                                            }
                                        }
                                        p { class: "settings-info", "STT: Whisper (local). TTS: Piper (local). Wake word detection supported." }
                                    },
                                    "policies" => rsx! {
                                        h2 { class: "settings-title", "Safety & Policies" }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "Execution Gate" }
                                            div { class: "settings-row",
                                                div { class: "settings-label-group",
                                                    span { class: "settings-label", "Auto-approve low-risk actions" }
                                                    span { class: "settings-desc", "Skip approval for actions classified as low risk" }
                                                }
                                                div { class: "toggle-group",
                                                    button {
                                                        class: if *settings_auto_approve.read() { "toggle-track on" } else { "toggle-track" },
                                                        onclick: move |_| { let c = *settings_auto_approve.read(); settings_auto_approve.set(!c); },
                                                        span { class: "toggle-knob" }
                                                    }
                                                    {
                                                        let label = if *settings_auto_approve.read() { "On" } else { "Off" };
                                                        rsx! { span { class: "toggle-label", "{label}" } }
                                                    }
                                                }
                                            }
                                        }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "Safety Stack" }
                                            p { class: "settings-info",
                                                "Execution Gate evaluates risk before every action. Kill Switch provides emergency stop (Cmd+Shift+K). Boundary Enforcer sets hard limits on file system, network, and process access."
                                            }
                                        }
                                    },
                                    "behavior" => rsx! {
                                        h2 { class: "settings-title", "Behavior" }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "Intent Cache" }
                                            div { class: "settings-row",
                                                div { class: "settings-label-group",
                                                    span { class: "settings-label", "Enable intent caching" }
                                                    span { class: "settings-desc", "Cache classified intents to skip re-classification" }
                                                }
                                                div { class: "toggle-group",
                                                    button {
                                                        class: "toggle-track on",
                                                        span { class: "toggle-knob" }
                                                    }
                                                    span { class: "toggle-label", "On" }
                                                }
                                            }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Cache TTL" }
                                                div { class: "segmented-control",
                                                    button { class: "segment", "15m" }
                                                    button { class: "segment active", "1h" }
                                                    button { class: "segment", "4h" }
                                                    button { class: "segment", "24h" }
                                                }
                                            }
                                        }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "Belief Revision" }
                                            div { class: "settings-row",
                                                div { class: "settings-label-group",
                                                    span { class: "settings-label", "Learn from corrections" }
                                                    span { class: "settings-desc", "When you correct Hydra, it remembers for next time" }
                                                }
                                                div { class: "toggle-group",
                                                    button {
                                                        class: "toggle-track on",
                                                        span { class: "toggle-knob" }
                                                    }
                                                    span { class: "toggle-label", "On" }
                                                }
                                            }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Belief persistence" }
                                                div { class: "segmented-control",
                                                    button { class: "segment", "Session" }
                                                    button { class: "segment active", "7 days" }
                                                    button { class: "segment", "30 days" }
                                                    button { class: "segment", "Forever" }
                                                }
                                            }
                                        }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "Context Compression" }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Compression strategy" }
                                                div { class: "segmented-control",
                                                    button { class: "segment", "Minimal" }
                                                    button { class: "segment active", "Balanced" }
                                                    button { class: "segment", "Aggressive" }
                                                }
                                            }
                                        }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "Sister Routing" }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Dispatch mode" }
                                                div { class: "segmented-control",
                                                    button { class: "segment active", "Parallel" }
                                                    button { class: "segment", "Sequential" }
                                                }
                                            }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Sister timeout" }
                                                div { class: "segmented-control",
                                                    button { class: "segment", "5s" }
                                                    button { class: "segment active", "10s" }
                                                    button { class: "segment", "30s" }
                                                    button { class: "segment", "60s" }
                                                }
                                            }
                                            div { class: "settings-row",
                                                div { class: "settings-label-group",
                                                    span { class: "settings-label", "Retry on failure" }
                                                    span { class: "settings-desc", "Retry failed sister calls once before giving up" }
                                                }
                                                div { class: "toggle-group",
                                                    button {
                                                        class: "toggle-track on",
                                                        span { class: "toggle-knob" }
                                                    }
                                                    span { class: "toggle-label", "On" }
                                                }
                                            }
                                        }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "Proactive Behavior" }
                                            div { class: "settings-row",
                                                div { class: "settings-label-group",
                                                    span { class: "settings-label", "Dream state" }
                                                    span { class: "settings-desc", "Process and consolidate knowledge during idle time" }
                                                }
                                                div { class: "toggle-group",
                                                    button {
                                                        class: "toggle-track on",
                                                        span { class: "toggle-knob" }
                                                    }
                                                    span { class: "toggle-label", "On" }
                                                }
                                            }
                                            div { class: "settings-row",
                                                div { class: "settings-label-group",
                                                    span { class: "settings-label", "Proactive insights" }
                                                    span { class: "settings-desc", "Surface relevant information before you ask" }
                                                }
                                                div { class: "toggle-group",
                                                    button {
                                                        class: "toggle-track on",
                                                        span { class: "toggle-knob" }
                                                    }
                                                    span { class: "toggle-label", "On" }
                                                }
                                            }
                                        }
                                    },
                                    _ => rsx! {
                                        h2 { class: "settings-title", "Advanced" }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "Server" }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "HTTP Server" }
                                                span { class: "settings-desc", style: "color: var(--success);", "http://127.0.0.1:3100" }
                                            }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "SSE Events" }
                                                span { class: "settings-desc", "/events" }
                                            }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "JSON-RPC" }
                                                span { class: "settings-desc", "/rpc" }
                                            }
                                        }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "File Paths" }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Config" }
                                                span { class: "settings-desc", "~/.hydra/config.toml" }
                                            }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Profile" }
                                                span { class: "settings-desc", "~/.hydra/profile.json" }
                                            }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Database" }
                                                span { class: "settings-desc", "~/.hydra/hydra.db" }
                                            }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Sessions" }
                                                span { class: "settings-desc", "~/.hydra/sessions/" }
                                            }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "MCP Config" }
                                                span { class: "settings-desc", "~/.hydra/mcp.json" }
                                            }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Receipts" }
                                                span { class: "settings-desc", "~/.hydra/receipts/" }
                                            }
                                        }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "Engine" }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Cognitive Loop" }
                                                span { class: "settings-desc", "5-phase: Perceive \u{2192} Think \u{2192} Decide \u{2192} Act \u{2192} Learn" }
                                            }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Execution Gate" }
                                                span { class: "settings-desc", "6-layer security stack" }
                                            }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Receipt Ledger" }
                                                span { class: "settings-desc", "Hash-chained audit trail with tamper detection" }
                                            }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Kill Switch" }
                                                span { class: "settings-desc", "Cmd+Shift+K \u{2014} emergency halt" }
                                            }
                                        }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "Skills & Federation" }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Crystallized Skills" }
                                                span { class: "settings-desc", "Evolve sister auto-captures reusable patterns" }
                                            }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Federation" }
                                                span { class: "settings-desc", "Peer discovery, skill sharing, task delegation" }
                                            }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Undo Stack" }
                                                span { class: "settings-desc", "Cmd+Z to undo file actions \u{2014} bounded history" }
                                            }
                                        }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "Debug" }
                                            p { class: "settings-info", "Version: Hydra v1.0.0" }
                                            p { class: "settings-info", "Runtime: Dioxus 0.6 + WebView" }
                                            p { class: "settings-info", "Platform: macOS (Darwin)" }
                                        }
                                    },
                                }
                                // Save button
                                div { class: "settings-save-area",
                                    button {
                                        class: "btn-primary",
                                        onclick: move |_| {
                                            save_current_profile();
                                            let mode = settings_default_mode.read().clone();
                                            current_mode.set(mode.clone());
                                            show_sidebar.set(mode == "workspace");
                                            show_settings.set(false);
                                        },
                                        "Save & Close"
                                    }
                                    button {
                                        class: "btn-secondary",
                                        onclick: move |_| show_settings.set(false),
                                        "Cancel"
                                    }
                                }
                            }
                        }
                    } else if *current_mode.read() == "invisible" {
                        // Invisible mode
                        div {
                            class: "invisible-mode",
                            div { class: "welcome-globe" }
                            p { class: "invisible-hint", "Say \"Hey Hydra\" or press Cmd+1 to switch modes" }
                        }
                    } else {
                        // ╔══════════════════════════════════════╗
                        // ║  CHAT VIEW                           ║
                        // ╚══════════════════════════════════════╝
                        div {
                            class: "chat-container",

                            // Workspace panels
                            if (*current_mode.read() == "workspace" || *current_mode.read() == "immersive") && plan_panel.read().steps.len() > 1 {
                                div {
                                    class: "workspace-panels",

                                    // Plan
                                    div {
                                        class: "panel",
                                        h3 { class: "panel-title", "Plan" }
                                        {
                                            let pp = plan_panel.read();
                                            rsx! {
                                                div { class: "plan-goal", "{pp.goal}" }
                                                div { class: "plan-steps",
                                                    for step in pp.steps.iter() {
                                                        div {
                                                            class: format!("plan-step {}", match step.status {
                                                                StepStatus::Completed => "completed",
                                                                StepStatus::Running => "running",
                                                                StepStatus::Failed => "failed",
                                                                StepStatus::Skipped => "skipped",
                                                                StepStatus::Pending => "pending",
                                                            }),
                                                            span { class: "step-icon", "{PlanPanel::step_icon(step.status)}" }
                                                            span { class: "step-label", "{step.label}" }
                                                        }
                                                    }
                                                }
                                                div { class: "plan-progress",
                                                    div { class: "progress-bar",
                                                        div { class: "progress-fill", style: format!("width: {}%", pp.progress_percent()) }
                                                    }
                                                    span { class: "progress-text", "{pp.progress_percent() as u32}%" }
                                                }
                                                if let Some(eta) = pp.eta_display() {
                                                    span { class: "plan-eta", "ETA: {eta}" }
                                                }
                                            }
                                        }
                                    }

                                    // Timeline
                                    {
                                        let tp = timeline_panel.read();
                                        let user_events: Vec<_> = tp.events.iter()
                                            .filter(|e| e.kind != TimelineEventKind::PhaseChange)
                                            .collect();
                                        if !user_events.is_empty() {
                                            rsx! {
                                                div {
                                                    class: "panel",
                                                    h3 { class: "panel-title", "Timeline" }
                                                    div { class: "timeline-events",
                                                        for event in user_events.iter() {
                                                            div {
                                                                class: format!("timeline-event {}", TimelinePanel::event_css_class(event.kind)),
                                                                span { class: "timeline-icon", "{TimelinePanel::event_icon(event.kind)}" }
                                                                span { class: "timeline-time", "{event.timestamp}" }
                                                                span { class: "timeline-label", "{event.title}" }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            rsx! {}
                                        }
                                    }

                                    // Evidence
                                    {
                                        let ep = evidence_panel.read();
                                        let items: Vec<_> = ep.items.iter()
                                            .filter(|item| EvidencePanel::is_meaningful(item))
                                            .collect();
                                        if !items.is_empty() {
                                            rsx! {
                                                div {
                                                    class: "panel",
                                                    h3 { class: "panel-title", "Evidence" }
                                                    div { class: "evidence-items",
                                                        for item in items.iter() {
                                                            div {
                                                                class: format!("evidence-item {}", EvidencePanel::evidence_css_class(item.kind)),
                                                                div { class: "evidence-header",
                                                                    span { class: "evidence-icon", "{EvidencePanel::evidence_icon(item.kind)}" }
                                                                    span { class: "evidence-title", "{item.title}" }
                                                                    if item.pinned {
                                                                        span { class: "evidence-pin", "\u{1F4CC}" }
                                                                    }
                                                                }
                                                                {
                                                                    let summary = EvidencePanel::human_summary(item);
                                                                    rsx! { p { class: "evidence-content", "{summary}" } }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            rsx! {}
                                        }
                                    }
                                }
                            }

                            // Search bar
                            if *show_search.read() {
                                div {
                                    class: "search-bar",
                                    span { class: "search-bar-icon" }
                                    input {
                                        class: "search-bar-input",
                                        placeholder: "Search messages...",
                                        value: "{search_query}",
                                        autofocus: true,
                                        oninput: move |e| search_query.set(e.value()),
                                        onkeydown: move |e| {
                                            if e.key() == Key::Escape {
                                                show_search.set(false);
                                                search_query.set(String::new());
                                            }
                                        },
                                    }
                                    {
                                        let q = search_query.read().clone();
                                        let count = if q.is_empty() { 0 } else {
                                            messages.read().iter()
                                                .filter(|(_, content, _)| content.to_lowercase().contains(&q.to_lowercase()))
                                                .count()
                                        };
                                        rsx! {
                                            if !q.is_empty() {
                                                span { class: "search-bar-count", "{count} found" }
                                            }
                                        }
                                    }
                                    button {
                                        class: "search-bar-close",
                                        onclick: move |_| {
                                            show_search.set(false);
                                            search_query.set(String::new());
                                        },
                                        "\u{2715}"
                                    }
                                }
                            }

                            // Messages
                            div {
                                class: "messages-list",
                                id: "messages-container",

                                if messages.read().is_empty() {
                                    div {
                                        class: "welcome-state",

                                        // New session flash indicator
                                        if *new_session_flash.read() {
                                            div {
                                                class: "new-session-badge",
                                                "New Session"
                                            }
                                        }

                                        // Voice globe — SVG rendered, reactive to state
                                        {
                                            let p = phase.read();
                                            let has_approval = pending_approval.read().is_some();
                                            let voice_on = *settings_voice.read();
                                            let globe_state = derive_globe_state(&p, has_approval, voice_on, false);
                                            let params = globe_params(globe_state);
                                            let mode = current_mode.read().clone();
                                            let size = match mode.as_str() {
                                                "immersive" => GlobeSize::Full,
                                                "companion" => GlobeSize::Medium,
                                                _ => GlobeSize::Medium,
                                            };
                                            let svg_html = globe_svg(&params, size.pixels());
                                            rsx! {
                                                div {
                                                    class: "voice-globe",
                                                    dangerous_inner_html: svg_html,
                                                }
                                            }
                                        }
                                        h2 { class: "welcome-title", "{greeting}" }
                                        p { class: "welcome-subtitle", "How can I help you today?" }

                                        // Quick-start suggestions
                                        div {
                                            class: "quick-starts",
                                            button {
                                                class: "quick-start-card",
                                                onclick: move |_| {
                                                    input.set("Analyze my codebase and suggest improvements".into());
                                                },
                                                span { class: "quick-start-icon icon-search" }
                                                span { class: "quick-start-text", "Analyze codebase" }
                                            }
                                            button {
                                                class: "quick-start-card",
                                                onclick: move |_| {
                                                    input.set("Help me debug an issue".into());
                                                },
                                                span { class: "quick-start-icon icon-debug" }
                                                span { class: "quick-start-text", "Debug an issue" }
                                            }
                                            button {
                                                class: "quick-start-card",
                                                onclick: move |_| {
                                                    input.set("Write a new feature".into());
                                                },
                                                span { class: "quick-start-icon icon-build" }
                                                span { class: "quick-start-text", "Build a feature" }
                                            }
                                            button {
                                                class: "quick-start-card",
                                                onclick: move |_| {
                                                    input.set("Explain how this project works".into());
                                                },
                                                span { class: "quick-start-icon icon-docs" }
                                                span { class: "quick-start-text", "Explain project" }
                                            }
                                        }

                                    }
                                }

                                for (i, (role, content, _css)) in messages.read().iter().enumerate() {
                                    {
                                        let sq = search_query.read().clone();
                                        let is_match = if sq.is_empty() { true } else {
                                            content.to_lowercase().contains(&sq.to_lowercase())
                                        };
                                        let msg_class = if !sq.is_empty() && is_match { "message search-hit" } else if !sq.is_empty() { "message search-dim" } else { "message" };
                                        let role_label = if role == "user" { "You" } else { "Hydra" };
                                        let html = markdown_to_html(content);
                                        rsx! {
                                            div {
                                                key: "{i}",
                                                class: msg_class.to_string(),
                                                div { class: "message-role", "{role_label}" }
                                                div {
                                                    class: "message-content",
                                                    dangerous_inner_html: html,
                                                }
                                            }
                                        }
                                    }
                                }

                                if *is_typing.read() {
                                    div {
                                        class: "typing-indicator",
                                        div { class: "typing-dot" }
                                        div { class: "typing-dot" }
                                        div { class: "typing-dot" }
                                    }
                                }
                            }

                            // Auto-scroll
                            {
                                let _count = messages.read().len();
                                let _typing = *is_typing.read();
                                rsx! { script { "requestAnimationFrame(function(){{ var el = document.getElementById('messages-container'); if(el) el.scrollTop = el.scrollHeight; }})" } }
                            }

                            // Approval card
                            {
                                let approval = pending_approval.read();
                                if let Some(card) = approval.as_ref() {
                                    let icon = card.icon.clone();
                                    let title = card.title.clone();
                                    let desc = card.description.clone();
                                    let preview_text = card.preview.clone().unwrap_or_default();
                                    let has_preview = card.preview.is_some();
                                    let show_challenge = card.needs_challenge();
                                    let challenge_text = card.challenge_phrase.clone().unwrap_or_default();
                                    let has_challenge = card.challenge_phrase.is_some();
                                    let primary = card.primary_action.clone();
                                    let secondary = card.secondary_action.clone();
                                    let countdown_val = *approval_countdown.read();
                                    let approve_mgr = card_approval_mgr.clone();
                                    let deny_mgr = card_approval_mgr.clone();
                                    let key_approve_mgr = card_approval_mgr.clone();
                                    let key_deny_mgr = card_approval_mgr.clone();
                                    rsx! {
                                        div {
                                            class: "approval-card",
                                            tabindex: "0",
                                            onkeydown: move |e| {
                                                match e.key() {
                                                    Key::Character(ref c) if c == "y" || c == "Y" => {
                                                        if let Some(id) = pending_approval_id.read().clone() {
                                                            let _ = key_approve_mgr.submit_decision(&id, ApprovalDecision::Approved);
                                                        }
                                                        pending_approval.set(None); pending_approval_id.set(None); approval_countdown.set(0);
                                                    }
                                                    Key::Character(ref c) if c == "n" || c == "N" => {
                                                        if let Some(id) = pending_approval_id.read().clone() {
                                                            let _ = key_deny_mgr.submit_decision(&id, ApprovalDecision::Denied { reason: "User denied via keyboard".into() });
                                                        }
                                                        pending_approval.set(None); pending_approval_id.set(None); approval_countdown.set(0);
                                                    }
                                                    _ => {}
                                                }
                                            },
                                            div { class: "approval-header",
                                                span { class: "approval-icon", "{icon}" }
                                                span { class: "approval-title", "{title}" }
                                            }
                                            p { class: "approval-desc", "{desc}" }
                                            if has_preview {
                                                div { class: "approval-preview", "{preview_text}" }
                                            }
                                            if show_challenge && has_challenge {
                                                p { class: "approval-challenge", "Type \"{challenge_text}\" to proceed" }
                                                input {
                                                    class: "challenge-input",
                                                    value: "{challenge_input}",
                                                    oninput: move |e| challenge_input.set(e.value()),
                                                }
                                            }
                                            if countdown_val > 0 {
                                                div { class: "approval-countdown", "Auto-declining in {countdown_val}s" }
                                                div { class: "approval-progress-bar",
                                                    div {
                                                        class: "approval-progress-fill",
                                                        style: format!("width: {}%", (countdown_val as f32 / 30.0 * 100.0).min(100.0)),
                                                    }
                                                }
                                            }
                                            div { class: "approval-actions",
                                                button {
                                                    class: "btn-primary",
                                                    onclick: move |_| {
                                                        if let Some(id) = pending_approval_id.read().clone() {
                                                            let _ = approve_mgr.submit_decision(&id, ApprovalDecision::Approved);
                                                        }
                                                        pending_approval.set(None); pending_approval_id.set(None); approval_countdown.set(0);
                                                    },
                                                    "{primary} "
                                                    span { class: "kbd", "Y" }
                                                }
                                                button {
                                                    class: "btn-secondary",
                                                    onclick: move |_| {
                                                        if let Some(id) = pending_approval_id.read().clone() {
                                                            let _ = deny_mgr.submit_decision(&id, ApprovalDecision::Denied { reason: "User denied".into() });
                                                        }
                                                        pending_approval.set(None); pending_approval_id.set(None); approval_countdown.set(0);
                                                    },
                                                    "{secondary} "
                                                    span { class: "kbd", "N" }
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    rsx! {}
                                }
                            }

                            // Celebration toast (auto-dismiss after 3s)
                            {
                                let cel = celebration.read();
                                if let Some(c) = cel.as_ref() {
                                    let msg = c.message.clone();
                                    // Auto-dismiss (only schedule once)
                                    if !*celebration_dismiss_scheduled.read() {
                                        celebration_dismiss_scheduled.set(true);
                                        spawn(async move {
                                            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                                            celebration.set(None);
                                            celebration_dismiss_scheduled.set(false);
                                        });
                                    }
                                    rsx! {
                                        div {
                                            class: "celebration-toast",
                                            onclick: move |_| {
                                                celebration.set(None);
                                                celebration_dismiss_scheduled.set(false);
                                            },
                                            span { class: "celebration-check-icon" }
                                            span { class: "celebration-msg", "{msg}" }
                                        }
                                    }
                                } else {
                                    rsx! {}
                                }
                            }

                            // Error card
                            {
                                let err = active_error.read();
                                if let Some(error) = err.as_ref() {
                                    let msg = error.message.clone();
                                    let expl = error.explanation.clone();
                                    let opts: Vec<(String, bool)> = error.options.iter()
                                        .map(|o| (o.label.clone(), o.is_primary)).collect();
                                    rsx! {
                                        div {
                                            class: "error-card",
                                            div { class: "error-icon", "\u{25C9}" }
                                            p { class: "error-message", "{msg}" }
                                            p { class: "error-explanation", "{expl}" }
                                            div { class: "error-options",
                                                for (label, is_primary) in opts.iter() {
                                                    button {
                                                        class: if *is_primary { "btn-primary" } else { "btn-secondary" },
                                                        onclick: move |_| active_error.set(None),
                                                        "{label}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    rsx! {}
                                }
                            }

                            // Input bar
                            div {
                                class: "input-bar",
                                div { class: "input-wrapper",
                                    // Mic button (only when voice is enabled)
                                    if *settings_voice.read() {
                                        button {
                                            class: if *voice_listening.read() { "mic-btn listening" } else { "mic-btn" },
                                            title: if *voice_listening.read() { "Stop listening" } else { "Start voice input" },
                                            onclick: move |_| {
                                                let listening = *voice_listening.read();
                                                if listening {
                                                    // STOP recording
                                                    mic_stop_flag.read().store(true, Ordering::Relaxed);
                                                    // voice_listening will be set false when transcript arrives
                                                } else {
                                                    // START recording
                                                    let openai_key = settings_openai_key.read().clone();
                                                    if openai_key.is_empty() {
                                                        active_error.set(Some(FriendlyError {
                                                            message: "Voice input requires an OpenAI API key".into(),
                                                            explanation: "Go to Settings > Models and enter your OpenAI key to enable voice transcription.".into(),
                                                            options: vec![],
                                                            icon_state: "error".into(),
                                                            can_undo: false,
                                                        }));
                                                        return;
                                                    }
                                                    voice_listening.set(true);
                                                    // Fresh stop flag
                                                    let flag = Arc::new(AtomicBool::new(false));
                                                    mic_stop_flag.set(flag.clone());

                                                    // Record in a std::thread (cpal::Stream is not Send)
                                                    let (tx, rx) = tokio::sync::oneshot::channel::<Option<(Vec<f32>, u32)>>();
                                                    std::thread::spawn(move || {
                                                        let result = voice_capture::record_until_stopped(flag);
                                                        let _ = tx.send(result);
                                                    });

                                                    // Await result, transcribe, auto-send
                                                    spawn(async move {
                                                        if let Ok(Some((samples, sample_rate))) = rx.await {
                                                            if samples.len() > 1600 { // at least 0.1s of audio
                                                                let wav = voice_capture::encode_wav(&samples, sample_rate);
                                                                let key = settings_openai_key.read().clone();
                                                                match voice_capture::transcribe_whisper(wav, &key).await {
                                                                    Ok(text) if !text.is_empty() => {
                                                                        input.set(text);
                                                                        // Auto-click send
                                                                        document::eval("setTimeout(function(){var b=document.querySelector('.send-btn');if(b)b.click();},50);");
                                                                    }
                                                                    Err(e) => {
                                                                        eprintln!("[hydra] transcription error: {}", e);
                                                                        active_error.set(Some(FriendlyError {
                                                                            message: "Transcription failed".into(),
                                                                            explanation: e,
                                                                            options: vec![],
                                                                            icon_state: "error".into(),
                                                                            can_undo: false,
                                                                        }));
                                                                    }
                                                                    _ => {}
                                                                }
                                                            }
                                                        } else {
                                                            active_error.set(Some(FriendlyError {
                                                                message: "Microphone not available".into(),
                                                                explanation: "No input device found. Check your microphone connection and system permissions.".into(),
                                                                options: vec![],
                                                                icon_state: "error".into(),
                                                                can_undo: false,
                                                            }));
                                                        }
                                                        voice_listening.set(false);
                                                    });
                                                }
                                            },
                                            span { class: if *voice_listening.read() { "mic-icon listening" } else { "mic-icon" } }
                                        }
                                    }
                                    input {
                                        class: "chat-input",
                                        placeholder: if *voice_listening.read() { "Listening..." } else { "Message Hydra..." },
                                        value: "{input}",
                                        oninput: move |e| {
                                            input.set(e.value());
                                            if input_error.read().is_some() { input_error.set(None); }
                                        },
                                        onkeypress: move |e| {
                                            if e.key() == Key::Enter {
                                                let text = input.read().clone();
                                                send_message(text);
                                            }
                                        },
                                    }
                                    button {
                                        class: "send-btn",
                                        onclick: move |_| {
                                            let text = input.read().clone();
                                            send_message(text);
                                        },
                                        "\u{2191}"
                                    }
                                }
                                {
                                    let err = input_error.read();
                                    if let Some(ref msg) = *err {
                                        rsx! { div { class: "input-error", "{msg}" } }
                                    } else {
                                        rsx! {}
                                    }
                                }
                                p { class: "input-hint", "Enter to send \u{00B7} \u{2318}K commands \u{00B7} \u{2318}B sidebar" }
                            }
                        }
                    }
                }
            }

            // ═══════════════════════════════════════════════════════
            // GHOST CURSOR OVERLAY
            // ═══════════════════════════════════════════════════════
            {
                let gc = ghost_cursor.read();
                let cursor_class = gc.css_class();
                let cursor_style = gc.transform_style();
                let label = gc.action_label.clone().unwrap_or_default();
                let has_label = gc.action_label.is_some();
                let (pdx, pdy) = gc.pupil_offset();
                let svg_html = cursor_svg(pdx, pdy);
                let is_visible = gc.visible;
                let mode_label = gc.mode.label();
                let show_mode_badge = gc.mode != CursorMode::Visible && is_visible;
                let is_replay = gc.mode == CursorMode::Replay;
                let replay_pct = (gc.replay_progress * 100.0) as u32;
                let trail_dots: Vec<(f64, f64, f64)> = gc.trail.iter().map(|d| {
                    let opacity = 1.0 - (d.age_ms as f64 / 1000.0);
                    (d.x, d.y, opacity.max(0.0))
                }).collect();
                drop(gc);

                rsx! {
                    div {
                        class: "ghost-cursor-overlay",

                        // Trail dots
                        for (i, (tx_pos, ty_pos, opacity)) in trail_dots.iter().enumerate() {
                            div {
                                key: "trail-{i}",
                                class: "ghost-trail-dot",
                                style: format!("left: {}px; top: {}px; opacity: {};", tx_pos + 4.0, ty_pos + 4.0, opacity),
                            }
                        }

                        // Click rings
                        for (i, (rx, ry, _ts)) in ghost_click_rings.read().iter().enumerate() {
                            div {
                                key: "ring-{i}",
                                class: "ghost-click-ring",
                                style: format!("left: {}px; top: {}px;", rx, ry),
                            }
                        }

                        // Robot cursor
                        div {
                            class: "{cursor_class}",
                            style: "{cursor_style}",
                            dangerous_inner_html: "{svg_html}",
                        }

                        // Action label
                        if has_label && is_visible {
                            div {
                                class: "ghost-cursor-label visible",
                                style: "{cursor_style}",
                                "{label}"
                            }
                        }

                        // Mode badge
                        if show_mode_badge {
                            div {
                                class: "ghost-mode-badge active",
                                "Cursor: {mode_label}"
                            }
                        }

                        // Replay controls
                        if is_replay {
                            div {
                                class: "ghost-replay-bar",
                                button {
                                    onclick: move |_| {
                                        ghost_cursor.write().set_mode(CursorMode::Visible);
                                    },
                                    "Stop"
                                }
                                div { class: "replay-progress",
                                    div {
                                        class: "replay-fill",
                                        style: format!("width: {}%", replay_pct),
                                    }
                                }
                                span { class: "replay-label", "{replay_pct}%" }
                            }
                        }
                    }
                }
            }
        }
    }
}
