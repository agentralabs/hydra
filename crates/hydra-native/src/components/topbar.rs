//! TopBar component data model.
//!
//! Defines the state and actions for the top navigation bar, including
//! cognitive phase indicators, mode switching, and session metadata.

use serde::{Deserialize, Serialize};

/// The five cognitive phases of the Hydra loop.
const PHASES: &[&str] = &["PERCEIVE", "THINK", "DECIDE", "ACT", "LEARN"];

/// A single phase indicator dot in the top bar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhaseDot {
    pub name: String,
    pub active: bool,
    pub completed: bool,
    pub css_class: String,
}

impl PhaseDot {
    /// Build a phase dot with the appropriate CSS class.
    pub fn new(name: &str, active: bool, completed: bool) -> Self {
        let css_class = if active {
            format!("phase-dot phase-dot--active phase-dot--{}", name.to_lowercase())
        } else if completed {
            format!("phase-dot phase-dot--completed phase-dot--{}", name.to_lowercase())
        } else {
            format!("phase-dot phase-dot--{}", name.to_lowercase())
        };
        Self {
            name: name.to_owned(),
            active,
            completed,
            css_class,
        }
    }
}

/// Top bar display state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopBarState {
    /// Current cognitive phase (e.g. "THINK").
    pub phase: String,
    /// Icon animation state (e.g. "idle", "breathing", "spinning").
    pub icon_state: String,
    /// Current operating mode (e.g. "autonomous", "supervised").
    pub mode: String,
    /// Whether the agent is connected to a provider.
    pub connected: bool,
    /// Whether the kill switch is engaged.
    pub kill_switch_active: bool,
    /// Remaining token budget as a percentage (0–100), if tracked.
    pub token_budget_percent: Option<u8>,
    /// Optional session title displayed in the top bar.
    pub session_title: Option<String>,
}

impl TopBarState {
    /// Produce phase dots for the five cognitive phases.
    ///
    /// Phases before the current one are marked completed, the current one is
    /// active, and later phases are neither.
    pub fn phase_dots(&self) -> Vec<PhaseDot> {
        let current_idx = PHASES
            .iter()
            .position(|p| p.eq_ignore_ascii_case(&self.phase));

        PHASES
            .iter()
            .enumerate()
            .map(|(i, &name)| match current_idx {
                Some(ci) if i < ci => PhaseDot::new(name, false, true),
                Some(ci) if i == ci => PhaseDot::new(name, true, false),
                _ => PhaseDot::new(name, false, false),
            })
            .collect()
    }
}

impl Default for TopBarState {
    fn default() -> Self {
        Self {
            phase: "PERCEIVE".into(),
            icon_state: "idle".into(),
            mode: "supervised".into(),
            connected: false,
            kill_switch_active: false,
            token_budget_percent: None,
            session_title: None,
        }
    }
}

/// Actions the top bar can dispatch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TopBarAction {
    ToggleSidebar,
    OpenSettings,
    SwitchMode(String),
    ToggleKillSwitch,
    OpenCommandPalette,
}

/// Build a [`TopBarState`] from raw signal values.
///
/// This is the bridge between runtime signals and the pure data model,
/// making it easy to test without a live UI.
pub fn build_topbar_state(
    phase: &str,
    icon_state: &str,
    mode: &str,
    connected: bool,
    kill_switch_active: bool,
    token_budget_percent: Option<u8>,
    session_title: Option<String>,
) -> TopBarState {
    TopBarState {
        phase: phase.to_owned(),
        icon_state: icon_state.to_owned(),
        mode: mode.to_owned(),
        connected,
        kill_switch_active,
        token_budget_percent,
        session_title,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_state() {
        let state = TopBarState::default();
        assert_eq!(state.phase, "PERCEIVE");
        assert_eq!(state.icon_state, "idle");
        assert_eq!(state.mode, "supervised");
        assert!(!state.connected);
        assert!(!state.kill_switch_active);
        assert!(state.token_budget_percent.is_none());
        assert!(state.session_title.is_none());
    }

    #[test]
    fn test_phase_dots_count() {
        let state = TopBarState::default();
        let dots = state.phase_dots();
        assert_eq!(dots.len(), 5);
    }

    #[test]
    fn test_phase_dots_names() {
        let state = TopBarState::default();
        let dots = state.phase_dots();
        let names: Vec<&str> = dots.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names, vec!["PERCEIVE", "THINK", "DECIDE", "ACT", "LEARN"]);
    }

    #[test]
    fn test_phase_dots_first_phase_active() {
        let state = TopBarState { phase: "PERCEIVE".into(), ..Default::default() };
        let dots = state.phase_dots();
        assert!(dots[0].active);
        assert!(!dots[0].completed);
        for dot in &dots[1..] {
            assert!(!dot.active);
            assert!(!dot.completed);
        }
    }

    #[test]
    fn test_phase_dots_middle_phase() {
        let state = TopBarState { phase: "DECIDE".into(), ..Default::default() };
        let dots = state.phase_dots();
        // PERCEIVE, THINK completed
        assert!(dots[0].completed);
        assert!(dots[1].completed);
        // DECIDE active
        assert!(dots[2].active);
        assert!(!dots[2].completed);
        // ACT, LEARN inactive
        assert!(!dots[3].active);
        assert!(!dots[3].completed);
        assert!(!dots[4].active);
        assert!(!dots[4].completed);
    }

    #[test]
    fn test_phase_dots_last_phase() {
        let state = TopBarState { phase: "LEARN".into(), ..Default::default() };
        let dots = state.phase_dots();
        for dot in &dots[..4] {
            assert!(dot.completed);
            assert!(!dot.active);
        }
        assert!(dots[4].active);
        assert!(!dots[4].completed);
    }

    #[test]
    fn test_phase_dots_unknown_phase() {
        let state = TopBarState { phase: "UNKNOWN".into(), ..Default::default() };
        let dots = state.phase_dots();
        for dot in &dots {
            assert!(!dot.active);
            assert!(!dot.completed);
        }
    }

    #[test]
    fn test_phase_dots_case_insensitive() {
        let state = TopBarState { phase: "think".into(), ..Default::default() };
        let dots = state.phase_dots();
        assert!(dots[0].completed);
        assert!(dots[1].active);
    }

    #[test]
    fn test_phase_dot_css_class_active() {
        let dot = PhaseDot::new("THINK", true, false);
        assert_eq!(dot.css_class, "phase-dot phase-dot--active phase-dot--think");
    }

    #[test]
    fn test_phase_dot_css_class_completed() {
        let dot = PhaseDot::new("ACT", false, true);
        assert_eq!(dot.css_class, "phase-dot phase-dot--completed phase-dot--act");
    }

    #[test]
    fn test_phase_dot_css_class_inactive() {
        let dot = PhaseDot::new("LEARN", false, false);
        assert_eq!(dot.css_class, "phase-dot phase-dot--learn");
    }

    #[test]
    fn test_build_topbar_state() {
        let state = build_topbar_state(
            "ACT",
            "spinning",
            "autonomous",
            true,
            false,
            Some(42),
            Some("Deploy v2".into()),
        );
        assert_eq!(state.phase, "ACT");
        assert_eq!(state.icon_state, "spinning");
        assert_eq!(state.mode, "autonomous");
        assert!(state.connected);
        assert!(!state.kill_switch_active);
        assert_eq!(state.token_budget_percent, Some(42));
        assert_eq!(state.session_title.as_deref(), Some("Deploy v2"));
    }

    #[test]
    fn test_build_topbar_state_minimal() {
        let state = build_topbar_state("PERCEIVE", "idle", "supervised", false, false, None, None);
        assert!(state.token_budget_percent.is_none());
        assert!(state.session_title.is_none());
    }

    #[test]
    fn test_topbar_action_variants() {
        let actions = vec![
            TopBarAction::ToggleSidebar,
            TopBarAction::OpenSettings,
            TopBarAction::SwitchMode("autonomous".into()),
            TopBarAction::ToggleKillSwitch,
            TopBarAction::OpenCommandPalette,
        ];
        assert_eq!(actions.len(), 5);
    }

    #[test]
    fn test_topbar_action_equality() {
        assert_eq!(TopBarAction::ToggleSidebar, TopBarAction::ToggleSidebar);
        assert_eq!(
            TopBarAction::SwitchMode("a".into()),
            TopBarAction::SwitchMode("a".into())
        );
        assert_ne!(
            TopBarAction::SwitchMode("a".into()),
            TopBarAction::SwitchMode("b".into())
        );
    }

    #[test]
    fn test_serialization_roundtrip() {
        let state = build_topbar_state(
            "THINK",
            "breathing",
            "supervised",
            true,
            true,
            Some(80),
            Some("Session".into()),
        );
        let json = serde_json::to_string(&state).unwrap();
        let back: TopBarState = serde_json::from_str(&json).unwrap();
        assert_eq!(back.phase, "THINK");
        assert_eq!(back.token_budget_percent, Some(80));
        assert!(back.kill_switch_active);
    }

    #[test]
    fn test_action_serialization_roundtrip() {
        let action = TopBarAction::SwitchMode("autonomous".into());
        let json = serde_json::to_string(&action).unwrap();
        let back: TopBarAction = serde_json::from_str(&json).unwrap();
        assert_eq!(back, action);
    }

    #[test]
    fn test_phase_dots_all_completed_before_last() {
        let state = TopBarState { phase: "LEARN".into(), ..Default::default() };
        let dots = state.phase_dots();
        let completed_count = dots.iter().filter(|d| d.completed).count();
        assert_eq!(completed_count, 4);
    }
}
