//! PatternEngine -- the pattern library coordinator.

use crate::{
    classifier::ClassificationResult,
    constants::MAX_PATTERN_LIBRARY_SIZE,
    entry::{PatternEntry, PatternKind},
    errors::PatternError,
    matcher::{find_matches, find_warnings},
};
use hydra_axiom::primitives::AxiomPrimitive;

/// The pattern engine.
pub struct PatternEngine {
    library: Vec<PatternEntry>,
}

impl PatternEngine {
    pub fn new() -> Self {
        let mut engine = Self {
            library: Vec::new(),
        };
        engine.seed_base_patterns();
        engine
    }

    /// Add a pattern to the library.
    pub fn add_pattern(&mut self, entry: PatternEntry) -> Result<(), PatternError> {
        if self.library.len() >= MAX_PATTERN_LIBRARY_SIZE {
            return Err(PatternError::LibraryFull {
                max: MAX_PATTERN_LIBRARY_SIZE,
            });
        }
        self.library.push(entry);
        Ok(())
    }

    /// Find all patterns matching these primitives.
    pub fn match_primitives(&self, primitives: &[AxiomPrimitive]) -> Vec<ClassificationResult> {
        let sig = primitives_to_signature(primitives);
        find_matches(&self.library, &sig)
    }

    /// Find anti-pattern warnings for these primitives.
    pub fn check_for_warnings(&self, primitives: &[AxiomPrimitive]) -> Vec<ClassificationResult> {
        let sig = primitives_to_signature(primitives);
        find_warnings(&self.library, &sig)
    }

    /// Confirm a pattern was observed again (strengthens confidence).
    pub fn confirm_pattern(&mut self, pattern_id: &str) {
        if let Some(entry) = self.library.iter_mut().find(|e| e.id == pattern_id) {
            entry.confirm();
        }
    }

    pub fn library_size(&self) -> usize {
        self.library.len()
    }

    pub fn antipattern_count(&self) -> usize {
        self.library.iter().filter(|e| e.kind.is_warning()).count()
    }

    pub fn success_pattern_count(&self) -> usize {
        self.library.iter().filter(|e| !e.kind.is_warning()).count()
    }

    /// Seed the library with base patterns known at build time.
    fn seed_base_patterns(&mut self) {
        // CASCADE FAILURE -- anti-pattern
        let mut cascade = PatternEntry::new(
            "Cascade Failure",
            "Small correlated failures compound into system-wide outage. \
             Seen in: deployment pipelines, financial markets, microservices.",
            PatternKind::AntiPattern {
                failure_mode: "system-wide cascade".into(),
                warning_signs: vec![
                    "increasing error rate across multiple services".into(),
                    "retry storms amplifying load".into(),
                    "cascading timeouts".into(),
                ],
            },
            &[
                AxiomPrimitive::Risk,
                AxiomPrimitive::CausalLink,
                AxiomPrimitive::Dependency,
            ],
            vec!["engineering".into(), "finance".into(), "operations".into()],
            "Install circuit breakers at dependency boundaries. \
             Implement backpressure. Add bulkheads between services.",
        );
        for _ in 0..3 {
            cascade.confirm();
        }
        let _ = self.add_pattern(cascade);

        // TRUST ESCALATION -- anti-pattern
        let mut trust_esc = PatternEntry::new(
            "Trust Escalation",
            "Gradual expansion of trust scope beyond original intent. \
             Seen in: API integrations, agent permissions, OAuth scopes.",
            PatternKind::AntiPattern {
                failure_mode: "privilege escalation".into(),
                warning_signs: vec![
                    "permissions expanding over time".into(),
                    "trust boundaries becoming unclear".into(),
                ],
            },
            &[AxiomPrimitive::TrustRelation, AxiomPrimitive::Risk],
            vec!["security".into(), "engineering".into()],
            "Audit trust scope. Enforce least privilege. \
             Review permission grants periodically.",
        );
        for _ in 0..3 {
            trust_esc.confirm();
        }
        let _ = self.add_pattern(trust_esc);

        // CIRCUIT BREAKER -- success pattern
        let mut circuit = PatternEntry::new(
            "Circuit Breaker",
            "Detect failure threshold, open the circuit, allow recovery, \
             then test with limited traffic. Universal solution to cascade failure.",
            PatternKind::SuccessPattern {
                success_condition: "isolated failures, system remains stable".into(),
                key_steps: vec![
                    "set failure threshold".into(),
                    "monitor for threshold breach".into(),
                    "open circuit on breach".into(),
                    "allow recovery time".into(),
                    "test with probe requests".into(),
                    "close circuit on success".into(),
                ],
            },
            &[
                AxiomPrimitive::Risk,
                AxiomPrimitive::Constraint,
                AxiomPrimitive::Dependency,
            ],
            vec!["engineering".into(), "finance".into()],
            "Apply circuit breaker pattern: \
             threshold -> open -> half-open -> closed.",
        );
        for _ in 0..5 {
            circuit.confirm();
        }
        let _ = self.add_pattern(circuit);

        // DEPENDENCY PINNING -- success pattern
        let mut pinning = PatternEntry::new(
            "Dependency Pinning",
            "Pin all dependency versions. Verify hashes. \
             Test updates in isolation before merging.",
            PatternKind::SuccessPattern {
                success_condition: "reproducible builds, no supply chain attacks".into(),
                key_steps: vec![
                    "pin exact versions".into(),
                    "verify content hashes".into(),
                    "automate update PRs".into(),
                    "test in isolation".into(),
                ],
            },
            &[AxiomPrimitive::Dependency, AxiomPrimitive::TrustRelation],
            vec!["engineering".into(), "security".into()],
            "Pin all dependencies. Use lock files. Verify integrity hashes.",
        );
        for _ in 0..4 {
            pinning.confirm();
        }
        let _ = self.add_pattern(pinning);
    }

    /// Summary for TUI.
    pub fn summary(&self) -> String {
        format!(
            "patterns: library={} anti={} success={}",
            self.library_size(),
            self.antipattern_count(),
            self.success_pattern_count(),
        )
    }
}

fn primitives_to_signature(primitives: &[AxiomPrimitive]) -> Vec<String> {
    let mut sig: Vec<String> = primitives.iter().map(|p| p.label().to_string()).collect();
    sig.sort();
    sig.dedup();
    sig
}

impl Default for PatternEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeds_base_patterns_on_new() {
        let engine = PatternEngine::new();
        assert!(engine.library_size() >= 4);
        assert!(engine.antipattern_count() >= 2);
        assert!(engine.success_pattern_count() >= 2);
    }

    #[test]
    fn cascade_primitives_match_cascade_pattern() {
        let engine = PatternEngine::new();
        let prims = vec![
            AxiomPrimitive::Risk,
            AxiomPrimitive::CausalLink,
            AxiomPrimitive::Dependency,
        ];
        let matches = engine.match_primitives(&prims);
        assert!(!matches.is_empty());
    }

    #[test]
    fn cascade_triggers_warning() {
        let engine = PatternEngine::new();
        let prims = vec![
            AxiomPrimitive::Risk,
            AxiomPrimitive::CausalLink,
            AxiomPrimitive::Dependency,
        ];
        let warnings = engine.check_for_warnings(&prims);
        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| w.pattern_name.contains("Cascade")));
    }

    #[test]
    fn trust_risk_matches_trust_escalation() {
        let engine = PatternEngine::new();
        let prims = vec![AxiomPrimitive::TrustRelation, AxiomPrimitive::Risk];
        let warnings = engine.check_for_warnings(&prims);
        assert!(!warnings.is_empty());
    }

    #[test]
    fn summary_format() {
        let engine = PatternEngine::new();
        let s = engine.summary();
        assert!(s.contains("patterns:"));
        assert!(s.contains("anti="));
        assert!(s.contains("success="));
    }
}
