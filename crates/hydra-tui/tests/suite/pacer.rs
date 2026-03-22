//! Pacer, dot, input, welcome, app, cockpit, status tests.

use hydra_tui::constants::{ALL_DOT_COLORS, SPINNER_FRAMES};
use hydra_tui::dot::DotKind;
use hydra_tui::input::InputBox;
use hydra_tui::pacer::{ContentKind, OutputPacer, PacerSignals};
use hydra_tui::verb::VerbContext;
use hydra_tui::{CockpitView, HydraTui, StatusLine, WelcomeScreen};
use std::collections::HashSet;

#[test]
fn pacer_base_delay() {
    let pacer = OutputPacer::new();
    let delay = pacer.delay_ms(ContentKind::AssistantText);
    assert!(delay > 0, "Base delay should be positive");
}

#[test]
fn pacer_user_echo_instant() {
    let pacer = OutputPacer::new();
    assert_eq!(pacer.delay_ms(ContentKind::UserEcho), 0);
    assert_eq!(pacer.chars_per_frame(ContentKind::UserEcho), usize::MAX);
}

#[test]
fn pacer_typing_accelerates() {
    let mut pacer = OutputPacer::new();
    let normal = pacer.delay_ms(ContentKind::AssistantText);
    pacer.update_signals(PacerSignals {
        scrolling: false,
        typing: true,
        is_error: false,
        is_urgent: false,
        needs_approval: false,
    });
    let fast = pacer.delay_ms(ContentKind::AssistantText);
    assert!(fast < normal, "Typing should reduce delay");
}

#[test]
fn pacer_critical_decelerates() {
    let pacer = OutputPacer::new();
    let normal = pacer.delay_ms(ContentKind::AssistantText);
    let critical = pacer.delay_ms(ContentKind::Critical);
    assert!(critical > normal, "Critical content should slow down");
}

#[test]
fn pacer_truncation_threshold() {
    let mut pacer = OutputPacer::new();
    assert!(!pacer.should_truncate());
    pacer.record_rendered(5000);
    assert!(pacer.should_truncate());
}

#[test]
fn pacer_chars_per_frame_is_20() {
    let pacer = OutputPacer::new();
    assert_eq!(
        pacer.chars_per_frame(ContentKind::AssistantText),
        20,
        "Spec requires 20 chars/frame"
    );
}

#[test]
fn pacer_sentence_boundary_pause() {
    let pacer = OutputPacer::new();
    let delay = pacer.delay_ms(ContentKind::SentenceBoundary);
    assert_eq!(delay, 80, "Sentence pause should be 80ms");
}

#[test]
fn pacer_urgent_briefing_holds() {
    let pacer = OutputPacer::new();
    let delay = pacer.delay_ms(ContentKind::UrgentBriefing);
    assert_eq!(delay, 500, "Urgent items should hold 500ms");
}

#[test]
fn dot_colors_all_unique() {
    let colors: HashSet<(u8, u8, u8)> = ALL_DOT_COLORS.iter().copied().collect();
    assert_eq!(colors.len(), 7, "All 7 dot colors must be unique");
}

#[test]
fn dot_kinds_match_colors() {
    for kind in DotKind::all() {
        let _ = kind.color();
        let _ = kind.symbol();
    }
    assert_eq!(DotKind::all().len(), 7);
}

#[test]
fn spinner_frames_match_spec() {
    assert_eq!(SPINNER_FRAMES, &["◌", "◐", "◑", "◒", "◓", "●"]);
}

#[test]
fn spinner_interval_is_180ms() {
    assert_eq!(hydra_tui::constants::SPINNER_INTERVAL_MS, 180);
}

#[test]
fn input_basic_operations() {
    let mut input = InputBox::new();
    assert!(input.is_empty());
    input.insert('H');
    input.insert('i');
    assert_eq!(input.text(), "Hi");
    input.backspace();
    assert_eq!(input.text(), "H");
    let submitted = input.submit();
    assert_eq!(submitted, "H");
    assert!(input.is_empty());
}

#[test]
fn welcome_screen_renders() {
    let welcome = WelcomeScreen::new();
    let lines = welcome.to_lines();
    assert!(!lines.is_empty());
}

#[test]
fn app_lifecycle() {
    let mut app = HydraTui::new();
    assert!(!app.is_conversation());
    app.kernel_ready();
    assert!(app.is_conversation());
    app.start_thinking();
    app.set_verb_context(VerbContext::Forge);
    app.tick();
    app.rotate_verb();
    app.stop_thinking();
    app.quit();
    assert!(app.should_quit);
}

#[test]
fn cockpit_mode_transitions() {
    let mut cockpit = CockpitView::new();
    assert!(!cockpit.is_conversation());
    cockpit.enter_conversation();
    assert!(cockpit.is_conversation());
    cockpit.toggle_companion_panel();
    assert!(cockpit.is_conversation());
    cockpit.toggle_companion_panel();
    assert!(cockpit.is_conversation());
}

#[test]
fn status_line_format() {
    let status = StatusLine::new();
    let line = status.format();
    let text: String = line.spans.iter().map(|s| s.content.to_string()).collect();
    assert!(text.contains("V="), "Status should show Lyapunov");
    assert!(text.contains("tokens"), "Status should show tokens");
    assert!(text.contains("◈ Hydra"), "Status should show entity");
    assert!(text.contains("session:"), "Status should show session");
    assert!(text.contains("tasks:"), "Status should show tasks");
}

#[test]
fn status_line_token_formatting() {
    let mut status = StatusLine::new();
    status.tokens = 847;
    let line = status.format();
    let text: String = line.spans.iter().map(|s| s.content.to_string()).collect();
    assert!(text.contains("847"), "Under 1k should be exact");

    status.tokens = 12_000;
    let line = status.format();
    let text: String = line.spans.iter().map(|s| s.content.to_string()).collect();
    assert!(text.contains("12k"), "Over 10k should be Nk");
}

#[test]
fn status_line_persona_brackets() {
    let mut status = StatusLine::new();
    status.persona = Some("security".to_string());
    let line = status.format();
    let text: String = line.spans.iter().map(|s| s.content.to_string()).collect();
    assert!(
        text.contains("[security]"),
        "Persona should appear in brackets"
    );
}
