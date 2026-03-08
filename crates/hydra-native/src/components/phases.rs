//! Phase indicator component data.

use crate::state::hydra::{CognitivePhase, PhaseState, PhaseStatus};

/// Visual state for a phase dot in the indicator
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseDot {
    pub phase: CognitivePhase,
    pub label: &'static str,
    pub state: PhaseState,
    pub css_class: &'static str,
    pub tokens_label: Option<String>,
    pub duration_label: Option<String>,
}

/// Build phase dots from current run state
pub fn build_phase_dots(phases: &[PhaseStatus]) -> Vec<PhaseDot> {
    CognitivePhase::ALL
        .iter()
        .map(|&phase| {
            let status = phases.iter().find(|p| p.phase == phase);
            let (state, tokens_label, duration_label) = match status {
                Some(ps) => (
                    ps.state.clone(),
                    ps.tokens_used.map(|t| format!("{}tok", t)),
                    ps.duration_ms.map(|d| format!("{}ms", d)),
                ),
                None => (PhaseState::Pending, None, None),
            };
            let css_class = match &state {
                PhaseState::Pending => "phase-pending",
                PhaseState::Running => "phase-running",
                PhaseState::Completed => "phase-completed",
                PhaseState::Failed => "phase-failed",
            };
            PhaseDot {
                phase,
                label: phase.label(),
                state,
                css_class,
                tokens_label,
                duration_label,
            }
        })
        .collect()
}

/// Connector state between two phases
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseConnector {
    pub from: CognitivePhase,
    pub to: CognitivePhase,
    pub active: bool,
    pub css_class: &'static str,
}

/// Build connectors between phase dots
pub fn build_connectors(phases: &[PhaseStatus]) -> Vec<PhaseConnector> {
    let phase_list = CognitivePhase::ALL;
    phase_list
        .windows(2)
        .map(|pair| {
            let from = pair[0];
            let to = pair[1];
            let from_completed = phases
                .iter()
                .any(|p| p.phase == from && p.state == PhaseState::Completed);
            let active = from_completed;
            PhaseConnector {
                from,
                to,
                active,
                css_class: if active {
                    "connector-active"
                } else {
                    "connector-inactive"
                },
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_pending() {
        let dots = build_phase_dots(&[]);
        assert_eq!(dots.len(), 5);
        assert!(dots.iter().all(|d| d.state == PhaseState::Pending));
        assert!(dots.iter().all(|d| d.css_class == "phase-pending"));
    }

    #[test]
    fn test_partial_progress() {
        let phases = vec![
            PhaseStatus {
                phase: CognitivePhase::Perceive,
                state: PhaseState::Completed,
                tokens_used: Some(100),
                duration_ms: Some(50),
            },
            PhaseStatus {
                phase: CognitivePhase::Think,
                state: PhaseState::Running,
                tokens_used: None,
                duration_ms: None,
            },
        ];
        let dots = build_phase_dots(&phases);
        assert_eq!(dots[0].state, PhaseState::Completed);
        assert_eq!(dots[0].tokens_label.as_deref(), Some("100tok"));
        assert_eq!(dots[1].state, PhaseState::Running);
        assert_eq!(dots[2].state, PhaseState::Pending);
    }

    #[test]
    fn test_connectors() {
        let phases = vec![PhaseStatus {
            phase: CognitivePhase::Perceive,
            state: PhaseState::Completed,
            tokens_used: None,
            duration_ms: None,
        }];
        let connectors = build_connectors(&phases);
        assert_eq!(connectors.len(), 4); // 5 phases → 4 connectors
        assert!(connectors[0].active); // Perceive→Think active
        assert!(!connectors[1].active); // Think→Decide inactive
    }
}
