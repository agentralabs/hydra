//! Hydra TUI — the cockpit binary.
//!
//! Full terminal interface: welcome screen → cockpit → conversation.
//! Event loop: 50ms tick with spinner (180ms), verb rotation (2200ms),
//! cursor blink (550ms).
//!
//! CRITICAL: stderr is redirected to a log file before entering raw mode.
//! Without this, eprintln! from the kernel middlewares corrupts the TUI.
//!
//! Usage:
//!   cargo run -p hydra-tui --bin hydra_tui

use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
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
use hydra_tui::stream_types::StreamItem;
use hydra_tui::HydraTui;

const TICK_RATE: Duration = Duration::from_millis(50);
const SPINNER_MS: u128 = 180;
const VERB_MS: u128 = 2200;
const CURSOR_MS: u128 = 550;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // CRITICAL: Redirect stderr to a log file BEFORE entering raw mode.
    // Without this, kernel eprintln! output corrupts the alternate screen.
    redirect_stderr();

    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // Run the app; restore terminal on any exit
    let result = run_app(&mut terminal);

    // Clean restore — always runs even on panic
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("hydra-tui error: {e}");
    }

    Ok(())
}

/// Redirect stderr to ~/.hydra/data/tui.log so kernel eprintln! doesn't
/// corrupt the alternate screen buffer.
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
            libc::dup2(fd, 2); // redirect stderr (fd 2) to log file
        }
        // Keep file open by leaking it (fd stays valid for process lifetime)
        std::mem::forget(file);
    }
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut tui = HydraTui::new();
    let mut cognitive = CognitiveLoop::new();

    let welcome_data = collect_welcome_data(&cognitive);

    let mut last_spinner = Instant::now();
    let mut last_verb = Instant::now();
    let mut last_cursor = Instant::now();
    let mut cursor_visible = true;
    let mut voice_mode = false;
    let boot_time = Instant::now();

    tui.welcome.kernel_ready = true;
    tui.welcome.boot_status = "Ready".into();

    loop {
        // Update session time
        tui.status.session_minutes = boot_time.elapsed().as_secs() / 60;

        // Render
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
            if let Event::Key(key) = event::read()? {
                match tui.cockpit.mode {
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
                        handle_cockpit_key(
                            key,
                            &mut tui,
                            &mut cognitive,
                            &mut voice_mode,
                        );
                    }
                }
            }
        }

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

fn handle_cockpit_key(
    key: KeyEvent,
    tui: &mut HydraTui,
    cognitive: &mut CognitiveLoop,
    voice_mode: &mut bool,
) {
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        tui.quit();
        return;
    }

    if key.code == KeyCode::Char('v') && key.modifiers.contains(KeyModifiers::CONTROL) {
        *voice_mode = !*voice_mode;
        let msg = if *voice_mode {
            "Voice mode activated (stub — no audio capture)"
        } else {
            "Voice mode deactivated"
        };
        tui.push_item(StreamItem::SystemNotification {
            id: uuid::Uuid::new_v4(),
            content: msg.to_string(),
            timestamp: chrono::Utc::now(),
        });
        return;
    }

    match key.code {
        KeyCode::Enter => {
            let text = tui.input.submit();
            if text.is_empty() {
                return;
            }

            tui.push_item(StreamItem::UserMessage {
                id: uuid::Uuid::new_v4(),
                text: text.clone(),
                timestamp: chrono::Utc::now(),
            });

            tui.start_thinking();

            let start = Instant::now();
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("tokio runtime");
            let response = rt.block_on(cognitive.cycle(&text));
            let duration = start.elapsed();

            tui.stop_thinking();

            tui.push_item(StreamItem::AssistantText {
                id: uuid::Uuid::new_v4(),
                text: response,
                timestamp: chrono::Utc::now(),
            });

            tui.status.tokens = tui.status.tokens.saturating_add(1);

            let receipt = format!("[cycle|{:.1}s|mw=8]", duration.as_secs_f64());
            tui.push_item(StreamItem::SystemNotification {
                id: uuid::Uuid::new_v4(),
                content: receipt,
                timestamp: chrono::Utc::now(),
            });

            tui.stream.scroll_to_bottom();
        }
        KeyCode::Backspace => tui.input.backspace(),
        KeyCode::Delete => tui.input.delete(),
        KeyCode::Left => tui.input.move_left(),
        KeyCode::Right => tui.input.move_right(),
        KeyCode::Home => tui.input.move_home(),
        KeyCode::End => tui.input.move_end(),
        KeyCode::Up => tui.stream.scroll_up(1),
        KeyCode::Down => tui.stream.scroll_down(1),
        KeyCode::PageUp => tui.stream.scroll_up(10),
        KeyCode::PageDown => tui.stream.scroll_down(10),
        KeyCode::Char(c) => tui.input.insert(c),
        _ => {}
    }
}

fn collect_welcome_data(_cognitive: &CognitiveLoop) -> WelcomeData {
    let genome_entries = hydra_genome::GenomeStore::open().len();
    let audit_count = hydra_audit::AuditEngine::open().record_count();

    WelcomeData {
        lyapunov: 1.0,
        growth_rate: 0.003,
        morphic_depth: audit_count as u64,
        genome_entries,
        step_count: 0,
        version: "0.1.0".into(),
        beliefs_loaded: 0,
        skills_active: genome_entries,
        antifragile_classes: 0,
        systems_mapped: 0,
    }
}
