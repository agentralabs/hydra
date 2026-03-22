//! Verb context and thinking verb state tests.

use hydra_tui::constants::{ALL_VERB_COLORS, SPINNER_FRAMES};
use hydra_tui::verb::{ThinkingVerbState, VerbContext};
use std::collections::HashSet;

#[test]
fn verb_colors_all_unique() {
    let colors: HashSet<(u8, u8, u8)> = ALL_VERB_COLORS.iter().copied().collect();
    assert_eq!(colors.len(), 12, "All 12 verb colors must be unique");
}

#[test]
fn verb_contexts_have_alternatives() {
    for ctx in VerbContext::all() {
        let alts = ctx.alternatives();
        assert_eq!(alts.len(), 4, "{ctx:?} must have 4 alternatives");
    }
}

#[test]
fn verb_colors_match_spec_rgb() {
    assert_eq!(VerbContext::General.rgb(), (200, 169, 110));
    assert_eq!(VerbContext::Forge.rgb(), (200, 112, 74));
    assert_eq!(VerbContext::Codebase.rgb(), (106, 184, 212));
    assert_eq!(VerbContext::Memory.rgb(), (74, 170, 106));
    assert_eq!(VerbContext::Synthesis.rgb(), (138, 106, 191));
    assert_eq!(VerbContext::Workflow.rgb(), (106, 138, 191));
    assert_eq!(VerbContext::Veritas.rgb(), (74, 200, 160));
    assert_eq!(VerbContext::Aegis.rgb(), (200, 74, 74));
    assert_eq!(VerbContext::Dream.rgb(), (122, 106, 200));
    assert_eq!(VerbContext::Persona.rgb(), (200, 106, 154));
    assert_eq!(VerbContext::Data.rgb(), (138, 200, 122));
    assert_eq!(VerbContext::HydraBranded.rgb(), (232, 200, 122));
}

#[test]
fn verb_sister_mapping() {
    assert_eq!(VerbContext::from_sister("forge"), VerbContext::Forge);
    assert_eq!(VerbContext::from_sister("codebase"), VerbContext::Codebase);
    assert_eq!(VerbContext::from_sister("memory"), VerbContext::Memory);
    assert_eq!(VerbContext::from_sister("veritas"), VerbContext::Veritas);
    assert_eq!(VerbContext::from_sister("aegis"), VerbContext::Aegis);
    assert_eq!(
        VerbContext::from_sister("agentic-forge"),
        VerbContext::Forge
    );
    assert_eq!(
        VerbContext::from_sister("cognition"),
        VerbContext::Synthesis
    );
    assert_eq!(VerbContext::from_sister("data"), VerbContext::Data);
    assert_eq!(VerbContext::from_sister("unknown"), VerbContext::General);
}

#[test]
fn verb_primary_verbs_match_spec() {
    assert_eq!(VerbContext::General.primary_verb(), "Cogitating");
    assert_eq!(VerbContext::Forge.primary_verb(), "Forging");
    assert_eq!(VerbContext::Codebase.primary_verb(), "Scanning");
    assert_eq!(VerbContext::Memory.primary_verb(), "Remembering");
    assert_eq!(VerbContext::Synthesis.primary_verb(), "Synthesizing");
    assert_eq!(VerbContext::Workflow.primary_verb(), "Orchestrating");
    assert_eq!(VerbContext::Veritas.primary_verb(), "Verifying");
    assert_eq!(VerbContext::Aegis.primary_verb(), "Shielding");
    assert_eq!(VerbContext::Dream.primary_verb(), "Dreaming");
    assert_eq!(VerbContext::Persona.primary_verb(), "Channeling");
    assert_eq!(VerbContext::Data.primary_verb(), "Crunching");
    assert_eq!(VerbContext::HydraBranded.primary_verb(), "Hydrating");
}

#[test]
fn verb_completion_format() {
    let state = ThinkingVerbState::new(VerbContext::Memory);
    let line = state.completion_display(0.1);
    assert_eq!(line, "● Remembered for 0.1s");

    let state = ThinkingVerbState::new(VerbContext::Forge);
    let line = state.completion_display(1.2);
    assert_eq!(line, "● Forged for 1.2s");
}

#[test]
fn spinner_cycles() {
    let mut state = ThinkingVerbState::new(VerbContext::General);
    state.start();
    let first = state.spinner_frame().to_string();
    for _ in 0..SPINNER_FRAMES.len() {
        state.tick_spinner();
    }
    let after_full_cycle = state.spinner_frame().to_string();
    assert_eq!(first, after_full_cycle, "Spinner should cycle back");
}

#[test]
fn verb_rotation() {
    let mut state = ThinkingVerbState::new(VerbContext::Forge);
    state.start();
    let first = state.status_display();
    state.rotate_verb();
    let second = state.status_display();
    assert_ne!(first, second, "Verb should rotate to different text");
}

#[test]
fn verb_status_display_format() {
    let mut state = ThinkingVerbState::new(VerbContext::Forge);
    state.start();
    let display = state.status_display();
    assert!(
        display.starts_with("Forging"),
        "Display should start with verb: {display}"
    );
    assert!(
        display.contains('◌'),
        "Display should contain spinner frame: {display}"
    );
}

#[test]
fn verb_completion_line() {
    let state = ThinkingVerbState::new(VerbContext::Codebase);
    let line = state.completion_line();
    assert_eq!(line, "● Scanned");
}

#[test]
fn verb_rotation_interval_is_2200ms() {
    assert_eq!(hydra_tui::constants::VERB_ROTATION_INTERVAL_MS, 2200);
}
