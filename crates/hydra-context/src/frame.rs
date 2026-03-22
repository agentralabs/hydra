//! Context frame — combines all five context windows into one view.

use crate::active::build_active;
use crate::anomaly::AnomalyContext;
use crate::gap::GapContext;
use crate::historical::{build_historical, SessionHistory};
use crate::predicted::{build_predicted, StagedIntent};
use crate::window::ContextWindow;
use hydra_comprehension::ComprehendedInput;
use serde::{Deserialize, Serialize};

/// The complete context frame combining all five windows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextFrame {
    /// Active context from the current input.
    pub active: ContextWindow,
    /// Historical context from the session.
    pub historical: ContextWindow,
    /// Predicted context from staged intents.
    pub predicted: ContextWindow,
    /// Gap context — what Hydra knows it does not know.
    pub gaps: ContextWindow,
    /// Anomaly context — unexpected patterns detected.
    pub anomalies: ContextWindow,
}

impl ContextFrame {
    /// Build a complete context frame from all five sources.
    pub fn build(
        input: &ComprehendedInput,
        history: &SessionHistory,
        staged: &[StagedIntent],
        gap_ctx: &GapContext,
        anomaly_ctx: &AnomalyContext,
    ) -> Self {
        Self {
            active: build_active(input),
            historical: build_historical(history),
            predicted: build_predicted(staged),
            gaps: gap_ctx.build_window(),
            anomalies: anomaly_ctx.build_window(),
        }
    }

    /// Return a TUI-friendly summary of this frame.
    pub fn summary(&self) -> String {
        format!(
            "context: active={} historical={} predicted={} gaps={} anomalies={}",
            self.active.len(),
            self.historical.len(),
            self.predicted.len(),
            self.gaps.len(),
            self.anomalies.len(),
        )
    }

    /// Return the total number of items across all windows.
    pub fn total_items(&self) -> usize {
        self.active.len()
            + self.historical.len()
            + self.predicted.len()
            + self.gaps.len()
            + self.anomalies.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anomaly::AnomalySignal;
    use crate::gap::GapSignal;
    use hydra_comprehension::{
        ConstraintStatus, Domain, Horizon, InputSource, ResonanceResult, TemporalContext,
    };

    fn make_input() -> ComprehendedInput {
        ComprehendedInput {
            raw: "deploy the api service now".to_string(),
            primary_domain: Domain::Engineering,
            all_domains: vec![(Domain::Engineering, 0.6)],
            primitives: vec![],
            temporal: TemporalContext {
                urgency: 0.8,
                horizon: Horizon::Immediate,
                constraint_status: ConstraintStatus::None,
            },
            resonance: ResonanceResult::empty(),
            source: InputSource::PrincipalText,
            confidence: 0.7,
            used_llm: false,
        }
    }

    #[test]
    fn full_frame_builds() {
        let input = make_input();
        let mut history = SessionHistory::new();
        history.add(make_input());
        let staged = vec![StagedIntent::new("next deploy", 0.6, "session")];
        let mut gaps = GapContext::new();
        gaps.add_gap(GapSignal::new("missing config", 0.7));
        let mut anomalies = AnomalyContext::new();
        anomalies.add_anomaly(AnomalySignal::new("unusual pattern", 0.8));

        let frame = ContextFrame::build(&input, &history, &staged, &gaps, &anomalies);
        assert!(!frame.active.is_empty());
        assert!(!frame.historical.is_empty());
        assert!(!frame.predicted.is_empty());
        assert!(!frame.gaps.is_empty());
        assert!(!frame.anomalies.is_empty());
        assert!(frame.total_items() > 0);
    }

    #[test]
    fn summary_format() {
        let input = make_input();
        let history = SessionHistory::new();
        let gaps = GapContext::new();
        let anomalies = AnomalyContext::new();
        let frame = ContextFrame::build(&input, &history, &[], &gaps, &anomalies);
        let s = frame.summary();
        assert!(s.contains("active="));
        assert!(s.contains("historical="));
    }
}
