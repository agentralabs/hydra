//! DiplomatEngine — the multi-instance coordination coordinator.
//! THE FINAL LAYER 6 CRATE.

use crate::{
    constants::MAX_STORED_SESSIONS,
    errors::DiplomatError,
    session::{DiplomacySession, SessionState},
    stance::Stance,
    synthesis::{synthesize, JointRecommendation},
};

/// The diplomat engine.
pub struct DiplomatEngine {
    sessions: Vec<DiplomacySession>,
}

impl DiplomatEngine {
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
        }
    }

    /// Open a new diplomacy session on a topic.
    pub fn open_session(&mut self, topic: impl Into<String>) -> String {
        let session = DiplomacySession::new(topic);
        let id = session.id.clone();
        if self.sessions.len() >= MAX_STORED_SESSIONS {
            self.sessions.remove(0);
        }
        self.sessions.push(session);
        id
    }

    /// Submit a stance to an open session.
    pub fn submit_stance(&mut self, session_id: &str, stance: Stance) -> Result<(), DiplomatError> {
        let session = self
            .sessions
            .iter_mut()
            .find(|s| s.id == session_id)
            .ok_or_else(|| DiplomatError::SessionClosed {
                id: session_id.to_string(),
            })?;
        session.submit_stance(stance)
    }

    /// Synthesize all stances into a joint recommendation.
    pub fn synthesize(&mut self, session_id: &str) -> Result<JointRecommendation, DiplomatError> {
        let session = self
            .sessions
            .iter()
            .find(|s| s.id == session_id)
            .ok_or_else(|| DiplomatError::SessionClosed {
                id: session_id.to_string(),
            })?;

        let rec = synthesize(session)?;

        // Mark session as concluded
        if let Some(s) = self.sessions.iter_mut().find(|s| s.id == session_id) {
            s.state = SessionState::Concluded {
                recommendation_id: rec.id.clone(),
            };
            s.closed_at = Some(chrono::Utc::now());
        }

        Ok(rec)
    }

    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    pub fn concluded_count(&self) -> usize {
        self.sessions
            .iter()
            .filter(|s| s.state.is_concluded())
            .count()
    }

    /// Summary for TUI.
    pub fn summary(&self) -> String {
        format!(
            "diplomat: sessions={} concluded={}",
            self.session_count(),
            self.concluded_count(),
        )
    }
}

impl Default for DiplomatEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_diplomacy_round() {
        let mut engine = DiplomatEngine::new();
        let sid = engine.open_session("enterprise-migration-strategy");

        engine
            .submit_stance(
                &sid,
                Stance::new(
                    "hydra-a",
                    "enterprise-migration-strategy",
                    "extract business logic first, then migrate data",
                    0.88,
                    vec!["5yr-migrations".into()],
                    vec!["data-integrity".into()],
                ),
            )
            .expect("submit a");
        engine
            .submit_stance(
                &sid,
                Stance::new(
                    "hydra-b",
                    "enterprise-migration-strategy",
                    "extract business logic first, then migrate data",
                    0.82,
                    vec!["3yr-ops".into()],
                    vec!["rollback-plan".into()],
                ),
            )
            .expect("submit b");
        engine
            .submit_stance(
                &sid,
                Stance::new(
                    "hydra-c",
                    "enterprise-migration-strategy",
                    "extract business logic first, then migrate data layer",
                    0.79,
                    vec!["2yr-migrations".into()],
                    vec![],
                ),
            )
            .expect("submit c");

        let rec = engine.synthesize(&sid).expect("should synthesize");
        assert!(rec.is_consensus());
        assert_eq!(rec.participant_count, 3);
        assert_eq!(engine.concluded_count(), 1);
    }

    #[test]
    fn minority_position_preserved_not_suppressed() {
        let mut engine = DiplomatEngine::new();
        let sid = engine.open_session("risk-assessment");

        engine
            .submit_stance(
                &sid,
                Stance::new(
                    "hydra-a",
                    "risk-assessment",
                    "risk is acceptable proceed",
                    0.85,
                    vec![],
                    vec![],
                ),
            )
            .expect("submit a");
        engine
            .submit_stance(
                &sid,
                Stance::new(
                    "hydra-b",
                    "risk-assessment",
                    "risk is acceptable proceed",
                    0.80,
                    vec![],
                    vec![],
                ),
            )
            .expect("submit b");
        engine
            .submit_stance(
                &sid,
                Stance::new(
                    "hydra-c",
                    "risk-assessment",
                    "risk is unacceptable stop",
                    0.75,
                    vec![],
                    vec![],
                ),
            )
            .expect("submit c");

        let rec = engine.synthesize(&sid).expect("should synthesize");
        assert!(!rec.minority_positions.is_empty());
        assert!(rec.minority_positions[0].contains("hydra-c"));
    }

    #[test]
    fn summary_format() {
        let engine = DiplomatEngine::new();
        let s = engine.summary();
        assert!(s.contains("diplomat:"));
    }
}
