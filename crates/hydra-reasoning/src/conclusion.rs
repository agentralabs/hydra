//! Reasoning conclusion types shared across all five modes.

use serde::{Deserialize, Serialize};

/// The five reasoning modes Hydra employs simultaneously.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReasoningMode {
    /// Deductive: axiom-based logical inference.
    Deductive,
    /// Inductive: pattern generalization from genome history.
    Inductive,
    /// Abductive: best-explanation inference from observations.
    Abductive,
    /// Analogical: cross-domain structural pattern matching.
    Analogical,
    /// Adversarial: threat modeling and attack surface analysis.
    Adversarial,
}

impl ReasoningMode {
    /// Return a human-readable label for this mode.
    pub fn label(&self) -> &str {
        match self {
            Self::Deductive => "deductive",
            Self::Inductive => "inductive",
            Self::Abductive => "abductive",
            Self::Analogical => "analogical",
            Self::Adversarial => "adversarial",
        }
    }

    /// Return whether this mode can use an LLM.
    ///
    /// Only Abductive may flag LLM usage (when axiom mapping fails
    /// and confidence is below threshold). All other modes are zero-LLM.
    pub fn can_use_llm(&self) -> bool {
        matches!(self, Self::Abductive)
    }
}

/// A single conclusion from one reasoning mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningConclusion {
    /// Which reasoning mode produced this conclusion.
    pub mode: ReasoningMode,
    /// The conclusion statement.
    pub statement: String,
    /// Confidence in this conclusion (0.0 to 1.0).
    pub confidence: f64,
    /// Supporting evidence or reasoning chain.
    pub supporting: Vec<String>,
    /// Whether an LLM was used to reach this conclusion.
    pub used_llm: bool,
}

impl ReasoningConclusion {
    /// Create a new reasoning conclusion.
    pub fn new(
        mode: ReasoningMode,
        statement: impl Into<String>,
        confidence: f64,
        supporting: Vec<String>,
        used_llm: bool,
    ) -> Self {
        Self {
            mode,
            statement: statement.into(),
            confidence: confidence.clamp(0.0, 1.0),
            supporting,
            used_llm,
        }
    }

    /// Return a one-line summary for TUI display.
    pub fn summary(&self) -> String {
        let llm_tag = if self.used_llm { "LLM" } else { "pure" };
        format!(
            "[{}][{}] {:.2}: {}",
            self.mode.label(),
            llm_tag,
            self.confidence,
            self.statement,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_labels() {
        assert_eq!(ReasoningMode::Deductive.label(), "deductive");
        assert_eq!(ReasoningMode::Adversarial.label(), "adversarial");
    }

    #[test]
    fn only_abductive_can_use_llm() {
        assert!(!ReasoningMode::Deductive.can_use_llm());
        assert!(!ReasoningMode::Inductive.can_use_llm());
        assert!(ReasoningMode::Abductive.can_use_llm());
        assert!(!ReasoningMode::Analogical.can_use_llm());
        assert!(!ReasoningMode::Adversarial.can_use_llm());
    }

    #[test]
    fn conclusion_confidence_clamped() {
        let c = ReasoningConclusion::new(ReasoningMode::Deductive, "test", 1.5, vec![], false);
        assert!((c.confidence - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn conclusion_summary_format() {
        let c = ReasoningConclusion::new(
            ReasoningMode::Deductive,
            "risk exists",
            0.75,
            vec!["evidence".into()],
            false,
        );
        let s = c.summary();
        assert!(s.contains("deductive"));
        assert!(s.contains("pure"));
        assert!(s.contains("0.75"));
    }
}
