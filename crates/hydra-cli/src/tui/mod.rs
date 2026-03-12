pub mod app;
pub mod app_nav;
pub mod app_runtime;
pub mod cognitive_handler;
pub mod command_exec;
pub mod commands;
pub mod event;
pub mod message;
pub mod onboarding;
pub(crate) mod onboarding_draw;
pub mod project;
pub mod project_files;
pub mod slash_commands;
pub mod slash_commands_dev_files;
pub mod slash_commands_dev_project;
pub mod slash_commands_session;
pub mod slash_commands_system;
pub mod slash_commands_hydra;
pub mod slash_commands_integration;
pub mod slash_commands_model;
pub mod theme;
pub mod ui;
pub mod widgets;

use std::io;
use std::os::unix::io::AsRawFd;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use tokio::sync::mpsc;

use app::App;
use event::{Event, EventHandler};
use hydra_native::sisters::SistersHandle;

/// Redirect ALL stderr to a log file. Returns original fd for restore.
fn redirect_stderr_to_log() -> Option<i32> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let log_dir = format!("{}/.hydra", home);
    let _ = std::fs::create_dir_all(&log_dir);
    let log_path = format!("{}/hydra-tui.log", log_dir);

    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .ok()?;

    let original_fd = unsafe { libc::dup(2) };
    if original_fd < 0 {
        return None;
    }
    let fd = log_file.as_raw_fd();
    unsafe { libc::dup2(fd, 2) };
    // Keep log_file alive — dropping it would close the fd we just redirected to
    std::mem::forget(log_file);
    Some(original_fd)
}

/// Restore stderr from saved fd.
fn restore_stderr(saved_fd: Option<i32>) {
    if let Some(fd) = saved_fd {
        unsafe {
            libc::dup2(fd, 2);
            libc::close(fd);
        }
    }
}

/// Draw the splash/boot screen with a progress bar.
fn draw_splash(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    label: &str,
    pct: u16,
) {
    let version = env!("CARGO_PKG_VERSION");
    let user_name = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "user".to_string());

    let _ = terminal.draw(|frame| {
        let area = frame.area();

        // Progress bar characters
        let bar_width = 30usize;
        let filled = (bar_width as f64 * pct as f64 / 100.0) as usize;
        let empty = bar_width.saturating_sub(filled);
        let bar = format!("{}{}",
            "█".repeat(filled),
            "░".repeat(empty),
        );

        let mut lines: Vec<Line> = Vec::new();

        // Vertical centering
        let content_height = 16;
        let pad_top = area.height.saturating_sub(content_height) / 2;
        for _ in 0..pad_top {
            lines.push(Line::default());
        }

        lines.push(Line::default());
        lines.push(Line::from(vec![
            Span::styled("  Welcome back, ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                &user_name,
                Style::default().fg(Color::Rgb(0, 210, 210)).add_modifier(Modifier::BOLD),
            ),
            Span::styled("!", Style::default().fg(Color::DarkGray)),
        ]));
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            "          ◉",
            Style::default().fg(Color::Rgb(0, 210, 210)),
        )));
        lines.push(Line::from(Span::styled(
            "        ╱   ╲",
            Style::default().fg(Color::Rgb(100, 149, 237)),
        )));
        lines.push(Line::from(Span::styled(
            "       ◉─────◉",
            Style::default().fg(Color::Rgb(100, 149, 237)),
        )));
        lines.push(Line::from(Span::styled(
            "        ╲   ╱",
            Style::default().fg(Color::Rgb(100, 149, 237)),
        )));
        lines.push(Line::from(Span::styled(
            "          ◉",
            Style::default().fg(Color::Rgb(0, 210, 210)),
        )));
        lines.push(Line::default());
        lines.push(Line::from(vec![
            Span::styled(format!("  {}", app::resolve_model_name()), Style::default().fg(Color::Rgb(160, 120, 220))),
            Span::styled(" · ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("v{}", version), Style::default().fg(Color::DarkGray)),
        ]));
        lines.push(Line::default());

        // Progress bar
        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", label), Style::default()),
            Span::styled(&bar, Style::default().fg(Color::Rgb(100, 149, 237))),
            Span::styled(format!("  {}%", pct), Style::default().fg(Color::DarkGray)),
        ]));
        lines.push(Line::default());

        let para = Paragraph::new(lines);
        frame.render_widget(para, area);
    });
}

/// Launch the full TUI interface.
///
/// Startup sequence:
/// 1. Redirect stderr to log file — BEFORE anything else
/// 2. Enter raw mode + alternate screen — TUI owns terminal from first pixel
/// 3. Draw welcome screen with progress bar at 0%
/// 4. Spawn sisters in background — progress bar animates
/// 5. When done: transition to full TUI with 14/14
/// Restore terminal to sane state. Safe to call multiple times.
fn cleanup_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
    // Show cursor in case it was hidden
    let _ = execute!(io::stdout(), crossterm::cursor::Show);
}

pub async fn run() -> io::Result<()> {
    // STEP 1: Redirect ALL stderr BEFORE any child process exists
    let saved_stderr = redirect_stderr_to_log();

    // Install panic hook that restores terminal before printing panic
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        cleanup_terminal();
        default_hook(info);
    }));

    // STEP 2: Take over terminal IMMEDIATELY
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // STEP 3: Onboarding — if first-time user, run wizard before anything else
    if onboarding::needs_onboarding() {
        match onboarding::run_onboarding(&mut terminal) {
            Ok(profile) => {
                // Apply profile settings to environment for the cognitive loop
                if let Some(ref key) = profile.anthropic_api_key {
                    std::env::set_var("ANTHROPIC_API_KEY", key);
                }
                if let Some(ref key) = profile.openai_api_key {
                    std::env::set_var("OPENAI_API_KEY", key);
                }
                if let Some(ref model) = profile.selected_model {
                    std::env::set_var("HYDRA_MODEL", model);
                }
                if let Some(ref dir) = profile.working_directory {
                    let _ = std::env::set_current_dir(dir);
                }
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {
                // User cancelled onboarding — exit cleanly
                restore_stderr(saved_stderr);
                cleanup_terminal();
                return Ok(());
            }
            Err(e) => {
                restore_stderr(saved_stderr);
                cleanup_terminal();
                return Err(e);
            }
        }
    } else {
        // Returning user — load saved working directory from profile
        if let Some(profile) = hydra_native::profile::load_profile() {
            if let Some(ref dir) = profile.working_directory {
                let _ = std::env::set_current_dir(dir);
            }
            if let Some(ref model) = profile.selected_model {
                if std::env::var("HYDRA_MODEL").is_err() {
                    std::env::set_var("HYDRA_MODEL", model);
                }
            }
        }
    }

    // STEP 4: Acquire per-project lock — prevents two Hydras on the same project.
    // Multiple Hydras on DIFFERENT projects is fine and expected.
    let project_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut _project_lock = hydra_runtime::InstanceLock::for_project(&project_dir);
    if let Err(e) = _project_lock.acquire() {
        // Can't lock — another instance owns this project
        restore_stderr(saved_stderr);
        cleanup_terminal();
        eprintln!("{}", e);
        return Err(io::Error::new(io::ErrorKind::AlreadyExists, e.to_string()));
    }

    // STEP 5: Draw splash with 0% — user sees clean welcome from first pixel
    draw_splash(&mut terminal, "Starting Hydra...", 0);

    // STEP 6: Spawn sisters in background
    let (sisters_tx, mut sisters_rx) = mpsc::unbounded_channel::<SistersHandle>();
    tokio::spawn(async move {
        let handle = hydra_native::sisters::init_sisters().await;
        let _ = sisters_tx.send(handle);
    });

    // STEP 7: Animate progress bar while waiting for sisters
    {
        let mut tick = 0u32;
        loop {
            // Check if sisters are ready
            if let Ok(handle) = sisters_rx.try_recv() {
                // Sisters done — show 100%
                let connected = handle.connected_count();
                let total = handle.all_sisters().len();
                draw_splash(
                    &mut terminal,
                    &format!("{}/{} sisters connected!", connected, total),
                    100,
                );
                // Brief pause to show 100%
                tokio::time::sleep(std::time::Duration::from_millis(400)).await;

                // Transition to full TUI
                let mut app = App::new();
                let event_handler = EventHandler::new(250);
                app.on_sisters_ready(handle);
                terminal.draw(|frame| ui::render(frame, &mut app))?;

                let result = run_loop(&mut terminal, &mut app, &event_handler).await;

                // Cleanup
                restore_stderr(saved_stderr);
                disable_raw_mode()?;
                execute!(
                    terminal.backend_mut(),
                    LeaveAlternateScreen,
                    DisableMouseCapture
                )?;
                terminal.show_cursor()?;
                return result;
            }

            // Animate progress bar (0% → 90% over ~10 seconds, never hits 100 until ready)
            tick = tick.saturating_add(1);
            let pct = std::cmp::min(90, (tick * 3 / 2) as u16);

            let label = match (tick / 8) % 14 {
                0  => "Connecting Memory...",
                1  => "Connecting Identity...",
                2  => "Connecting Codebase...",
                3  => "Connecting Vision...",
                4  => "Connecting Comm...",
                5  => "Connecting Contract...",
                6  => "Connecting Time...",
                7  => "Connecting Planning...",
                8  => "Connecting Cognition...",
                9  => "Connecting Reality...",
                10 => "Connecting Forge...",
                11 => "Connecting Aegis...",
                12 => "Connecting Veritas...",
                13 => "Connecting Evolve...",
                _  => "Connecting...",
            };

            draw_splash(&mut terminal, label, pct);

            // ~60fps would be 16ms, but 100ms is smooth enough and light on CPU
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;

            // Check for Ctrl+C during boot
            if crossterm::event::poll(std::time::Duration::from_millis(0))? {
                if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                    if key.modifiers == crossterm::event::KeyModifiers::CONTROL
                        && key.code == crossterm::event::KeyCode::Char('c')
                    {
                        restore_stderr(saved_stderr);
                        disable_raw_mode()?;
                        execute!(
                            terminal.backend_mut(),
                            LeaveAlternateScreen,
                            DisableMouseCapture
                        )?;
                        terminal.show_cursor()?;
                        return Ok(());
                    }
                }
            }
        }
    }
}

async fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    event_handler: &EventHandler,
) -> io::Result<()> {
    loop {
        terminal.draw(|frame| ui::render(frame, app))?;

        match event_handler.next().await? {
            Event::Tick => {
                app.tick();
            }
            Event::Key(key_event) => {
                event::handle_key_event(app, key_event);
            }
            Event::Mouse(mouse) => {
                use crossterm::event::MouseEventKind;
                match mouse.kind {
                    MouseEventKind::ScrollUp => app.scroll_up(),
                    MouseEventKind::ScrollDown => app.scroll_down(),
                    _ => {}
                }
            }
            Event::Resize(_, _) => {}
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
