//! Dot, input, spinner, and shared-type tests.

use hydra_tui::constants::{ALL_DOT_COLORS, SPINNER_FRAMES};
use hydra_tui::dot::DotKind;
use hydra_tui::input::InputBox;
use std::collections::HashSet;

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
