//! Hydra Desktop — native desktop app built with Dioxus.
use dioxus::prelude::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
mod app_init_settings;
mod app_profile;
mod platform;
mod pulse_voice;
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
const HYDRA_VERSION: &str = env!("CARGO_PKG_VERSION");
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
    // ── Per-project lock — prevents two Hydras on the same project ──
    let mut lock_error_msg: Signal<Option<String>> = use_signal(|| None);
    let _project_lock: Arc<parking_lot::Mutex<hydra_runtime::InstanceLock>> = use_hook(|| {
        let project_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let mut lock = hydra_runtime::InstanceLock::for_project(&project_dir);
        if let Err(e) = lock.acquire() {
            eprintln!("[hydra] {}", e);
            lock_error_msg.set(Some(e.to_string()));
        }
        Arc::new(parking_lot::Mutex::new(lock))
    });
    // ── Load persisted profile ── + seed factory profiles + auto-load operational profile
    let persisted = load_profile();
    let onboarding_done = persisted.as_ref().map_or(false, |p| p.onboarding_complete);
    app_profile::seed_profiles_if_needed();
    let (init_op_profile, init_overlay) = app_profile::auto_load_profile()
        .map(|(p, o)| (Some(p), o)).unwrap_or((None, None));
    let chat_db = Arc::new(ChatPersistence::init_or_memory());
    let (decide_engine, invention_engine, proactive_notifier, agent_spawner, undo_stack, approval_manager, federation_manager, hydra_db, swarm_manager, file_watcher, proactive_file_engine) = include!("app_engines.rs");
    let (send_msg_approval_mgr, palette_approval_mgr, card_approval_mgr) = (approval_manager.clone(), approval_manager.clone(), approval_manager.clone());
    let sisters: Signal<Option<SistersHandle>> = use_signal(|| None);
    let sisters_status = use_signal(|| "Connecting...".to_string());
    { let mut sisters = sisters.clone(); let mut sisters_status = sisters_status.clone();
      use_hook(move || { spawn(async move {
          let handle = hydra_native::sisters::init_sisters().await;
          let status = handle.status_summary();
          sisters.set(Some(handle)); sisters_status.set(status);
      }); }); }
    let s = app_init_settings::extract_init_settings(&persisted);
    let (init_theme, init_voice, init_sounds, init_volume) = (s.theme, s.voice, s.sounds, s.volume);
    let (init_auto_approve, init_default_mode, init_model) = (s.auto_approve, s.default_mode, s.model);
    let (init_anthropic_key, init_openai_key, init_google_key) = (s.anthropic_key, s.openai_key, s.google_key);
    let init_memory_capture = s.memory_capture;
    let (init_smtp_host, init_smtp_user, init_smtp_password, init_smtp_to) = (s.smtp_host, s.smtp_user, s.smtp_password, s.smtp_to);
    // ── Profile state (operational profiles with beliefs) ──
    let mut active_op_profile: Signal<Option<hydra_native::OperationalProfile>> = use_signal(move || init_op_profile);
    let mut profile_overlay: Signal<Option<String>> = use_signal(move || init_overlay);
    // ── Core state signals ──
    let mut input = use_signal(|| String::new());
    let chat_db_init = chat_db.clone();
    let mut messages = use_signal(move || chat_db_init.load_messages());
    let chat_db_sig: Signal<Arc<ChatPersistence>> = use_signal(|| chat_db.clone());
    let mut connected = use_signal(|| false);
    let mut phase = use_signal(|| "Idle".to_string());
    let mut icon_state = use_signal(|| "idle".to_string());
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
    // ── Graceful session shutdown on window close ──
    {
        let sisters_drop = sisters.clone();
        let msgs_drop = messages.clone();
        let ob_drop = onboarding.clone();
        use_drop(move || {
            let sh = sisters_drop.read().clone();
            let msgs = msgs_drop.read().clone();
            let name = ob_drop.read().user_name.clone().unwrap_or_default();
            if let Some(handle) = sh {
                let history: Vec<(String, String)> = msgs.iter()
                    .map(|(role, content, _)| (role.clone(), content.clone()))
                    .collect();
                std::thread::spawn(move || {
                    if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                        .enable_all().build()
                    {
                        let _ = rt.block_on(tokio::time::timeout(
                            std::time::Duration::from_secs(5),
                            handle.shutdown_session(&name, &history),
                        ));
                    }
                });
            }
        });
    }
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
    let mut settings_local_model = use_signal(|| "llama3.3".to_string());
    let mut settings_anthropic_key = use_signal(move || init_anthropic_key);
    let mut settings_openai_key = use_signal(move || init_openai_key);
    let mut settings_google_key = use_signal(move || init_google_key);
    let mut settings_tab = use_signal(|| "general".to_string());
    let mut oauth_status = use_signal(|| {
        let oauth = AnthropicOAuth::new();
        if oauth.is_authenticated() {
            (("authenticated".into(), oauth.account_email().unwrap_or("").to_string(), oauth.subscription_tier().unwrap_or("").to_string()))
        } else { ("not_authenticated".to_string(), String::new(), String::new()) }
    });
    let mut oauth_loading = use_signal(|| false);
    let mut current_mode = use_signal(move || init_mode_for_current);
    let mut sidebar = use_signal(|| { let mut sb = Sidebar::new(); sb.add_task("session-1", "Session 1"); sb });
    let mut show_sidebar = use_signal(move || init_mode_for_sidebar == "workspace");
    let mut pending_approval = use_signal(|| Option::<ApprovalCard>::None);
    let mut pending_approval_id = use_signal(|| Option::<String>::None);
    let mut challenge_input = use_signal(|| String::new());
    let mut ghost_cursor = use_signal(|| GhostCursorState::new());
    let mut ghost_click_rings: Signal<Vec<(f64, f64, u64)>> = use_signal(|| Vec::new());
    let mut _active_progress = use_signal(|| Option::<ProgressJourney>::None);
    let mut celebration = use_signal(|| Option::<Celebration>::None);
    let mut celebration_dismiss_scheduled = use_signal(|| false);
    let mut active_error = use_signal(|| Option::<FriendlyError>::None);
    let mut approval_countdown = use_signal(|| 0u32);
    let mut settings_tts_voice = use_signal(|| "nova".to_string());
    let mut settings_stt_lang = use_signal(|| "en".to_string());
    let mut settings_wake_word = use_signal(|| false);
    let mut settings_audio_input = use_signal(|| "default".to_string());
    let mut settings_auto_listen = use_signal(|| false);
    let mut settings_risk_threshold = use_signal(|| "medium".to_string());
    let mut settings_file_write = use_signal(|| true);
    let mut settings_network_access = use_signal(|| true);
    let mut settings_shell_exec = use_signal(|| true);
    let mut settings_max_file_edits = use_signal(|| "25".to_string());
    let mut settings_require_approval_critical = use_signal(|| true);
    let mut settings_sandbox_mode = use_signal(|| false);
    let mut settings_intent_cache = use_signal(|| true);
    let mut settings_cache_ttl = use_signal(|| "1h".to_string());
    let mut settings_learn_corrections = use_signal(|| true);
    let mut settings_belief_persist = use_signal(|| "7 days".to_string());
    let mut settings_compression = use_signal(|| "Balanced".to_string());
    let mut settings_dispatch_mode = use_signal(|| "Parallel".to_string());
    let mut settings_sister_timeout = use_signal(|| "10s".to_string());
    let mut settings_retry_failures = use_signal(|| true);
    let mut settings_dream_state = use_signal(|| true);
    let mut settings_proactive = use_signal(|| true);
    let mut settings_federation = use_signal(|| false);
    let mut settings_memory_capture = use_signal(move || init_memory_capture);
    let mut settings_smtp_host = use_signal(move || init_smtp_host);
    let mut settings_smtp_user = use_signal(move || init_smtp_user);
    let mut settings_smtp_password = use_signal(move || init_smtp_password);
    let mut settings_smtp_to = use_signal(move || init_smtp_to);
    // ── Advanced settings ──
    let mut settings_server_port = use_signal(|| "3100".to_string());
    let mut settings_log_level = use_signal(|| "info".to_string());
    let mut settings_debug_mode = use_signal(|| false);
    let mut settings_telemetry = use_signal(|| false);
    let mut backup_status = use_signal(|| String::new());
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
    let mut voice_pending_send = use_signal(|| Option::<String>::None);
    let mut companion_auto_listen = use_signal(|| false);
    let mut tts_playing = use_signal(|| false); // echo prevention: don't listen while speaking
    let mut cognitive_done = use_signal(|| false); // auto-listen waits for this AND !tts_playing
    let mut show_search = use_signal(|| false);
    let mut search_query = use_signal(|| String::new());
    let mut show_receipts = use_signal(|| false);
    let mut session_store = use_signal(|| HashMap::<String, Vec<(String, String, String)>>::new());
    let pulse = use_signal(|| Arc::new(pulse_voice::PulseVoice::new()));
    let monitor: Signal<Arc<parking_lot::Mutex<hydra_monitor::SystemMonitor>>> = use_signal(|| Arc::new(parking_lot::Mutex::new(hydra_monitor::SystemMonitor::new())));
    let tracer: Signal<Arc<parking_lot::Mutex<hydra_trace::TraceCollector>>> = use_signal(|| Arc::new(parking_lot::Mutex::new(hydra_trace::TraceCollector::new(100))));
    // ── Undo/Redo state ──
    let mut can_undo = use_signal(|| false);
    let mut can_redo = use_signal(|| false);
    let mut last_undo_action = use_signal(|| Option::<String>::None);
    let undo_sig: Signal<Arc<parking_lot::Mutex<UndoStack>>> = use_signal(|| undo_stack.clone());
    // ── Workspace panels ──
    let mut plan_panel = use_signal(|| PlanPanel::default());
    let mut timeline_panel = use_signal(|| TimelinePanel::new());
    let mut evidence_panel = use_signal(|| EvidencePanel::new());
    // ── File watcher polling (P2 proactive suggestions) ──
    { let fw = file_watcher.clone(); let pfe = proactive_file_engine.clone(); let mut tp = timeline_panel.clone();
      use_hook(move || { if fw.is_some() { spawn(async move { loop { tokio::time::sleep(std::time::Duration::from_secs(4)).await;
        let changes = if let Some(ref w) = fw { w.lock().drain_changes() } else { vec![] };
        if !changes.is_empty() { for s in pfe.lock().process_changes(&changes) { let now = chrono::Local::now().format("%H:%M:%S").to_string();
            let kind = if matches!(s.priority, hydra_pulse::SuggestionPriority::Urgent) { TimelineEventKind::Error } else { TimelineEventKind::Info };
            let detail = s.action.map(|a| format!("{} ({:?})", s.message, a)).unwrap_or_else(|| s.message.clone());
            tp.write().push_event(&now, kind, &s.title, Some(&detail), Some("Watcher")); } } } }); } }); }
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
                                kb_approval_mgr.cancel_all();
                                is_typing.set(false); phase.set("Idle".into()); icon_state.set("idle".into());
                                pending_approval.set(None); pending_approval_id.set(None); phase_statuses.set(vec![]);
                                active_error.set(Some(FriendlyError { message: "Kill Switch Activated".into(),
                                    explanation: "All operations halted. Press Escape to dismiss, or Cmd+N for a fresh session.".into(),
                                    options: vec![], icon_state: "error".into(), can_undo: false }));
                            }
                            "k" => { let c = *show_command_palette.read(); show_command_palette.set(!c); if !c { command_palette.write().reset(); } }
                            "b" => { let c = *show_sidebar.read(); show_sidebar.set(!c); }
                            "," => { show_settings.set(true); }
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
    // Show lock error if another instance owns this project
    if lock_error_msg.read().is_some() && active_error.read().is_none() {
        let err = lock_error_msg.read().clone().unwrap_or_default();
        active_error.set(Some(FriendlyError {
            message: "Another Hydra instance is running".into(),
            explanation: format!("{} Close it or choose a different project.", err),
            options: vec![], icon_state: "error".into(), can_undo: false,
        }));
    }
    let user_name = onboarding.read().user_name.clone().unwrap_or_default();
    let greeting = if user_name.is_empty() { "Hi there!".to_string() } else { format!("Hi {}!", user_name) };
    let model_display = {
        let m = settings_model.read().clone();
        if m.contains("opus") { "Opus 4.6".to_string() }
        else if m.contains("sonnet") { "Sonnet 4.6".to_string() }
        else if m.contains("haiku") { "Haiku 4.5".to_string() }
        else { m }
    };

    // ── Send message handler + save profile (extracted for compilation memory) ──
    let (mut send_message, save_current_profile) = include!("app_send_handler.rs");

    // Voice → send bridge: peek() reads without subscribing (avoids infinite loop).
    // Re-render is triggered by input.set() in the voice trigger.
    if voice_pending_send.peek().is_some() {
        if let Some(text) = voice_pending_send.write().take() {
            eprintln!("[hydra:voice-bridge] Sending: {}", &text[..text.len().min(60)]);
            send_message(text);
        }
    }

    // ── RSX (split into fragment files for compilation memory) ──
    include!("app_rsx.rs")
}
