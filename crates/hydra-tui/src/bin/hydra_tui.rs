//! Hydra TUI — the cockpit binary.
//! Event loop: 50ms tick, spinner 180ms, verb 2200ms, cursor 550ms.
//! stderr redirected to log file before raw mode.
use std::io;
use std::time::{Duration, Instant};
use crossterm::event::{self, Event, KeyCode, KeyModifiers, MouseEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use hydra_kernel::engine::CognitiveLoop;
use hydra_tui::cockpit::CockpitMode;
use hydra_tui::render_cockpit;
use hydra_tui::render_welcome::{self, WelcomeData};
use hydra_tui::HydraTui;
const TICK_RATE: Duration = Duration::from_millis(50);
const SPINNER_MS: u128 = 180;
const VERB_MS: u128 = 2200;
const CURSOR_MS: u128 = 550;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = hydra_tui::config::HydraConfig::load();
    hydra_tui::theme::init(hydra_tui::theme::Theme::by_name(&config.tui.theme));
    redirect_stderr();
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    let result = run_app(&mut terminal);
    // Succession: export state on shutdown
    export_succession_state();
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        crossterm::event::DisableMouseCapture,
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;
    if let Err(e) = result {
        eprintln!("hydra-tui error: {e}");
    }
    Ok(())
}
fn redirect_stderr() {
    use std::fs::OpenOptions;
    use std::os::unix::io::AsRawFd;
    let log_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".hydra")
        .join("data");
    let _ = std::fs::create_dir_all(&log_dir);
    let log_path = log_dir.join("tui.log");
    if let Ok(file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        let fd = file.as_raw_fd();
        unsafe {
            libc::dup2(fd, 2);
        }
        std::mem::forget(file);
    }
}
fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut tui = HydraTui::new();
    let mut cognitive = CognitiveLoop::new();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    let welcome_data = collect_welcome_data(&cognitive);
    let mut last_spinner = Instant::now();
    let mut last_verb = Instant::now();
    let mut last_cursor = Instant::now();
    let mut cursor_visible = true;
    let mut voice_loop = hydra_voice::VoiceLoop::new();
    let boot_time = Instant::now();
    // Streaming state
    type ChunkRx = tokio::sync::mpsc::Receiver<hydra_kernel::loop_::llm_stream::StreamChunk>;
    let mut active_stream: Option<ChunkRx> = None;
    let mut streaming_text = String::new();
    let mut streaming_prepared: Option<hydra_kernel::engine::PreparedCycle> = None;
    // Create companion channel — companion runs independently via signal fabric
    let (companion_channel, companion_endpoint) = hydra_signals::create_companion_channel();
    let mut companion_service = hydra_companion::CompanionService::new(companion_endpoint);
    tui.companion_channel = Some(companion_channel);
    tui.welcome.kernel_ready = true;
    tui.welcome.boot_status = "Ready".into();
    push_boot_briefing(&mut tui, &welcome_data);
    loop {
        tui.status.session_minutes = boot_time.elapsed().as_secs() / 60;
        let mode = tui.cockpit.mode.clone();
        terminal.draw(|f| {
            let area = f.area();
            match mode {
                CockpitMode::Welcome => {
                    render_welcome::render(f, area, &welcome_data);
                }
                CockpitMode::Conversation | CockpitMode::CompanionPanel => {
                    render_cockpit::render(f, area, &tui, cursor_visible);
                }
            }
        })?;
        if tui.should_quit {
            return Ok(());
        }
        if event::poll(TICK_RATE)? {
            match event::read()? {
                Event::Key(key) => match tui.cockpit.mode {
                    CockpitMode::Welcome => {
                        if key.code == KeyCode::Char('c')
                            && key.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            tui.quit();
                            continue;
                        }
                        tui.cockpit.enter_conversation();
                    }
                    CockpitMode::Conversation | CockpitMode::CompanionPanel => {
                        // Ctrl+C during streaming: interrupt
                        if key.code == KeyCode::Char('c')
                            && key.modifiers.contains(KeyModifiers::CONTROL)
                            && active_stream.is_some()
                        {
                            active_stream = None;
                            streaming_text.push_str("\n[interrupted]");
                            tui.stream.update_last_text(&streaming_text);
                            tui.stop_thinking();
                            streaming_text.clear();
                            streaming_prepared = None;
                            continue;
                        }
                        let request = hydra_tui::key_handler::handle_cockpit_key(
                            key,
                            &mut tui,
                            &mut cognitive,
                            &rt,
                            &mut voice_loop,
                        );
                        // Start streaming if key handler returned a request
                        if let Some(req) = request {
                            if active_stream.is_none() {
                                match rt.block_on(cognitive.start_streaming(&req.prepared)) {
                                    Ok(rx) => {
                                        active_stream = Some(rx);
                                        streaming_text.clear();
                                        streaming_prepared = Some(req.prepared);
                                    }
                                    Err(e) => {
                                        tui.stop_thinking();
                                        tui.stream.update_last_text(
                                            &format!("[Hydra error: {e}]"),
                                        );
                                        cognitive.finalize_streaming(
                                            req.prepared, &format!("[error: {e}]"), 0,
                                        );
                                    }
                                }
                            }
                        }
                    }
                },
                Event::Mouse(mouse) => match mouse.kind {
                    MouseEventKind::ScrollUp => tui.stream.scroll_up(3),
                    MouseEventKind::ScrollDown => tui.stream.scroll_down(3),
                    _ => {}
                },
                Event::Resize(_, _) => {
                    tui.stream.scroll_to_bottom();
                }
                _ => {}
            }
        }
        if let Some(rx) = &mut active_stream {
            use hydra_kernel::loop_::llm_stream::StreamChunk;
            // Drain all available chunks this tick (non-blocking)
            loop {
                match rx.try_recv() {
                    Ok(StreamChunk::Text(chunk)) => {
                        streaming_text.push_str(&chunk);
                        tui.stream.update_last_text(&streaming_text);
                    }
                    Ok(StreamChunk::Done { tokens_used, duration_ms }) => {
                        tui.stop_thinking();
                        tui.status.tokens += tokens_used as u64;
                        // Finalize the cycle
                        if let Some(prepared) = streaming_prepared.take() {
                            cognitive.finalize_streaming(prepared, &streaming_text, tokens_used);
                        }
                        let receipt = format!(
                            "[cycle|{:.1}s|tok:{}|streamed]",
                            duration_ms as f64 / 1000.0,
                            tokens_used,
                        );
                        tui.push_item(hydra_tui::stream_types::StreamItem::SystemNotification {
                            id: uuid::Uuid::new_v4(),
                            content: receipt,
                            timestamp: chrono::Utc::now(),
                        });
                        tui.push_item(hydra_tui::stream_types::StreamItem::Blank);
                        // Speak response if voice is active
                        if voice_loop.can_speak() {
                            voice_loop.speak_response(&streaming_text);
                        }
                        active_stream = None;
                        streaming_text.clear();
                        break;
                    }
                    Ok(StreamChunk::Error(e)) => {
                        tui.stop_thinking();
                        tui.stream.update_last_text(&format!(
                            "{}\n[Streaming error: {e}]",
                            streaming_text
                        ));
                        active_stream = None;
                        streaming_text.clear();
                        if let Some(p) = streaming_prepared.take() {
                            drop(p);
                        }
                        break;
                    }
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                    Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                        active_stream = None;
                        break;
                    }
                }
            }
        }
        // Companion service tick
        companion_service.tick();
        // Drain companion outputs into temp vec (avoids borrow conflict)
        let comp_outputs: Vec<_> = tui.companion_channel.as_ref()
            .map(|ch| {
                let mut v = Vec::new();
                while let Some(o) = ch.poll_output() { v.push(o); }
                v
            })
            .unwrap_or_default();
        for output in comp_outputs {
            match output {
                hydra_signals::CompanionOutput::Message(msg) => {
                    tui.push_item(hydra_tui::stream_types::StreamItem::SystemNotification {
                        id: uuid::Uuid::new_v4(), content: msg, timestamp: chrono::Utc::now(),
                    });
                }
                hydra_signals::CompanionOutput::Signal { class, content, .. } => {
                    tui.push_item(hydra_tui::stream_types::StreamItem::SystemNotification {
                        id: uuid::Uuid::new_v4(),
                        content: format!("{} Companion: {}", class.symbol(), content),
                        timestamp: chrono::Utc::now(),
                    });
                }
                _ => {}
            }
        }
        // Voice loop polling — process mic events
        hydra_tui::key_handler::poll_voice(
            &mut tui,
            &mut voice_loop,
            &mut cognitive,
            &rt,
        );
        // Animation ticks
        let now = Instant::now();
        if now.duration_since(last_spinner).as_millis() >= SPINNER_MS {
            tui.tick();
            last_spinner = now;
        }
        if now.duration_since(last_verb).as_millis() >= VERB_MS {
            tui.rotate_verb();
            last_verb = now;
        }
        if now.duration_since(last_cursor).as_millis() >= CURSOR_MS {
            cursor_visible = !cursor_visible;
            last_cursor = now;
        }
    }
}
/// Push morning briefing items into the stream at boot.
fn push_boot_briefing(tui: &mut HydraTui, data: &WelcomeData) {
    use hydra_tui::stream_types::{BriefingPriority, StreamItem};
    // Self-repair results
    let repairs = hydra_kernel::self_repair::self_repair();
    let fixed = repairs.iter().filter(|(_, ok)| *ok).count();
    let total = repairs.len();
    if total > 0 {
        let priority = if fixed < total {
            BriefingPriority::High
        } else {
            BriefingPriority::Normal
        };
        tui.push_item(StreamItem::BriefingItem {
            id: uuid::Uuid::new_v4(),
            content: format!("Self-repair: {fixed}/{total} issues fixed"),
            priority,
            timestamp: chrono::Utc::now(),
        });
    }
    // Genome count
    if data.genome_entries > 0 {
        tui.push_item(StreamItem::BriefingItem {
            id: uuid::Uuid::new_v4(),
            content: format!("Genome: {} proven approaches loaded", data.genome_entries),
            priority: BriefingPriority::Normal,
            timestamp: chrono::Utc::now(),
        });
    }
    // Memory status
    let amem_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".hydra/data/hydra.amem");
    if amem_path.exists() {
        tui.push_item(StreamItem::BriefingItem {
            id: uuid::Uuid::new_v4(),
            content: "Memory: persistent .amem loaded".into(),
            priority: BriefingPriority::Low,
            timestamp: chrono::Utc::now(),
        });
    }
    // Dream notification — check if genome grew since last session
    let last_genome_count = load_last_genome_count();
    if data.genome_entries > last_genome_count && last_genome_count > 0 {
        let new_entries = data.genome_entries - last_genome_count;
        tui.push_item(StreamItem::DreamNotification {
            id: uuid::Uuid::new_v4(),
            content: format!(
                "[Dream] Genome grew: {} → {} entries (+{} new since last session)",
                last_genome_count, data.genome_entries, new_entries
            ),
            timestamp: chrono::Utc::now(),
        });
    }
    save_last_genome_count(data.genome_entries);
    // Boot complete
    tui.push_item(StreamItem::BriefingItem {
        id: uuid::Uuid::new_v4(),
        content: format!(
            "Boot complete — {} middlewares active, v{}",
            8, data.version
        ),
        priority: BriefingPriority::Low,
        timestamp: chrono::Utc::now(),
    });
}
fn collect_welcome_data(cognitive: &CognitiveLoop) -> WelcomeData {
    let genome_entries = cognitive.genome_len();
    let audit_count = hydra_audit::AuditEngine::open().record_count();
    let username = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "operator".into());
    WelcomeData {
        lyapunov: 1.0,
        growth_rate: 0.003,
        morphic_depth: audit_count as u64,
        genome_entries,
        step_count: audit_count as u64,
        version: env!("CARGO_PKG_VERSION").into(),
        beliefs_loaded: 0,
        skills_active: genome_entries,
        antifragile_classes: 0,
        systems_mapped: 0,
        username,
    }
}
/// Load last known genome count from ~/.hydra/data/.genome_count
fn load_last_genome_count() -> usize {
    let path = dirs::home_dir()
        .unwrap_or_default()
        .join(".hydra/data/.genome_count");
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}
/// Save current genome count for next session comparison
fn save_last_genome_count(count: usize) {
    let path = dirs::home_dir()
        .unwrap_or_default()
        .join(".hydra/data/.genome_count");
    let _ = std::fs::write(&path, count.to_string());
}
fn export_succession_state() {
    let mut s = hydra_succession::SuccessionEngine::new();
    let st = hydra_succession::InstanceState {
        instance_id: uuid::Uuid::new_v4().to_string(), lineage_id: "hydra-primary".into(),
        days_running: 1, soul_entries: 0, genome_entries: 0, calibration_profiles: 0,
    };
    let _ = s.export(&st);
}
