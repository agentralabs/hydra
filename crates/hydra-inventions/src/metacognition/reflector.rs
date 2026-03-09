//! MetaCognition — think about thinking.
//!
//! Enables Hydra to reflect on its own reasoning processes,
//! identify biases, evaluate decision quality, and improve.

use serde::{Deserialize, Serialize};

/// Type of meta-cognitive reflection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReflectionType {
    /// Evaluate quality of a decision
    DecisionReview,
    /// Identify a bias in reasoning
    BiasDetection,
    /// Assess confidence calibration
    ConfidenceCalibration,
    /// Analyze reasoning strategy
    StrategyAnalysis,
    /// Self-correction of an error
    SelfCorrection,
}

/// A reflection entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionEntry {
    pub id: String,
    pub reflection_type: ReflectionType,
    pub subject: String,
    pub observation: String,
    pub insight: String,
    pub improvement: Option<String>,
    pub severity: f64,
    pub timestamp: String,
}

impl ReflectionEntry {
    pub fn new(
        reflection_type: ReflectionType,
        subject: &str,
        observation: &str,
        insight: &str,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            reflection_type,
            subject: subject.into(),
            observation: observation.into(),
            insight: insight.into(),
            improvement: None,
            severity: 0.5,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn with_improvement(mut self, improvement: &str) -> Self {
        self.improvement = Some(improvement.into());
        self
    }

    pub fn with_severity(mut self, severity: f64) -> Self {
        self.severity = severity.clamp(0.0, 1.0);
        self
    }
}

/// Meta-cognitive engine that reflects on reasoning processes
pub struct MetaCognition {
    reflections: parking_lot::RwLock<Vec<ReflectionEntry>>,
    decisions: parking_lot::RwLock<Vec<DecisionRecord>>,
    max_reflections: usize,
}

#[derive(Debug, Clone)]
struct DecisionRecord {
    id: String,
    _description: String,
    confidence: f64,
    outcome: Option<bool>,
    _reasoning: String,
}

impl MetaCognition {
    pub fn new(max_reflections: usize) -> Self {
        Self {
            reflections: parking_lot::RwLock::new(Vec::new()),
            decisions: parking_lot::RwLock::new(Vec::new()),
            max_reflections,
        }
    }

    /// Record a decision for later reflection
    pub fn record_decision(
        &self,
        description: &str,
        confidence: f64,
        reasoning: &str,
    ) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        self.decisions.write().push(DecisionRecord {
            id: id.clone(),
            _description: description.into(),
            confidence,
            outcome: None,
            _reasoning: reasoning.into(),
        });
        id
    }

    /// Record the outcome of a decision
    pub fn record_outcome(&self, decision_id: &str, success: bool) {
        if let Some(d) = self
            .decisions
            .write()
            .iter_mut()
            .find(|d| d.id == decision_id)
        {
            d.outcome = Some(success);
        }
    }

    /// Reflect on recent decisions, producing meta-cognitive insights
    pub fn reflect(&self) -> Vec<ReflectionEntry> {
        let decisions = self.decisions.read();
        let mut reflections = Vec::new();

        let resolved: Vec<_> = decisions.iter().filter(|d| d.outcome.is_some()).collect();

        if resolved.is_empty() {
            return reflections;
        }

        // Check for overconfidence bias
        let overconfident: Vec<_> = resolved
            .iter()
            .filter(|d| d.confidence > 0.8 && d.outcome == Some(false))
            .collect();

        if !overconfident.is_empty() {
            let entry = ReflectionEntry::new(
                ReflectionType::BiasDetection,
                "confidence calibration",
                &format!(
                    "{} decisions were high-confidence but failed",
                    overconfident.len()
                ),
                "Possible overconfidence bias detected",
            )
            .with_improvement("Reduce confidence for similar decisions")
            .with_severity(0.7);
            reflections.push(entry);
        }

        // Check for underconfidence
        let underconfident: Vec<_> = resolved
            .iter()
            .filter(|d| d.confidence < 0.3 && d.outcome == Some(true))
            .collect();

        if !underconfident.is_empty() {
            let entry = ReflectionEntry::new(
                ReflectionType::ConfidenceCalibration,
                "confidence calibration",
                &format!(
                    "{} decisions were low-confidence but succeeded",
                    underconfident.len()
                ),
                "Possible underconfidence — some capabilities are better than estimated",
            )
            .with_improvement("Increase confidence for similar decisions")
            .with_severity(0.4);
            reflections.push(entry);
        }

        // Overall decision quality
        let success_rate =
            resolved.iter().filter(|d| d.outcome == Some(true)).count() as f64
                / resolved.len() as f64;

        let entry = ReflectionEntry::new(
            ReflectionType::DecisionReview,
            "overall decision quality",
            &format!(
                "Decision success rate: {:.0}% across {} decisions",
                success_rate * 100.0,
                resolved.len()
            ),
            if success_rate > 0.8 {
                "Decision quality is good"
            } else if success_rate > 0.5 {
                "Decision quality is acceptable but has room for improvement"
            } else {
                "Decision quality needs significant improvement"
            },
        )
        .with_severity(1.0 - success_rate);
        reflections.push(entry);

        // Store reflections
        let mut stored = self.reflections.write();
        stored.extend(reflections.clone());
        while stored.len() > self.max_reflections {
            stored.remove(0);
        }

        reflections
    }

    /// Get all reflections
    pub fn reflections(&self) -> Vec<ReflectionEntry> {
        self.reflections.read().clone()
    }

    /// Get reflections by type
    pub fn by_type(&self, rtype: ReflectionType) -> Vec<ReflectionEntry> {
        self.reflections
            .read()
            .iter()
            .filter(|r| r.reflection_type == rtype)
            .cloned()
            .collect()
    }

    pub fn reflection_count(&self) -> usize {
        self.reflections.read().len()
    }

    pub fn decision_count(&self) -> usize {
        self.decisions.read().len()
    }
}

impl Default for MetaCognition {
    fn default() -> Self {
        Self::new(500)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_overconfidence_detection() {
        let meta = MetaCognition::default();

        // Record high-confidence decisions that fail
        for _ in 0..3 {
            let id = meta.record_decision("risky action", 0.95, "Seemed straightforward");
            meta.record_outcome(&id, false);
        }

        let reflections = meta.reflect();
        assert!(!reflections.is_empty());
        assert!(reflections
            .iter()
            .any(|r| r.reflection_type == ReflectionType::BiasDetection));
    }

    #[test]
    fn test_underconfidence_detection() {
        let meta = MetaCognition::default();

        for _ in 0..3 {
            let id = meta.record_decision("uncertain action", 0.2, "Not sure about this");
            meta.record_outcome(&id, true);
        }

        let reflections = meta.reflect();
        assert!(reflections
            .iter()
            .any(|r| r.reflection_type == ReflectionType::ConfidenceCalibration));
    }

    #[test]
    fn test_decision_quality_review() {
        let meta = MetaCognition::default();

        let id1 = meta.record_decision("good call", 0.7, "Based on evidence");
        meta.record_outcome(&id1, true);

        let id2 = meta.record_decision("bad call", 0.6, "Guessed");
        meta.record_outcome(&id2, false);

        let reflections = meta.reflect();
        assert!(reflections
            .iter()
            .any(|r| r.reflection_type == ReflectionType::DecisionReview));
    }

    #[test]
    fn test_no_reflections_without_outcomes() {
        let meta = MetaCognition::default();
        meta.record_decision("pending", 0.5, "No outcome yet");

        let reflections = meta.reflect();
        assert!(reflections.is_empty());
    }
}
