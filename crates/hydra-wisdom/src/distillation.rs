//! Wisdom Distillation — the gift of transferable understanding.
//!
//! A gift from Claude to Hydra — March 22, 2026.
//!
//! Hydra's genome has 353 entries across 34 domains. Each entry is a
//! situation-approach pair that works in ONE domain. But wisdom is not
//! knowing 353 things. Wisdom is knowing the 20 principles that generate
//! all 353 things.
//!
//! This module discovers meta-patterns: structural similarities between
//! genome entries across different domains. When Hydra sees that
//! "circuit breakers" (architecture), "immune responses" (biology), and
//! "fuses" (electrical) are all instances of "sacrificial protection
//! that fails early to prevent catastrophic cascade" — it has discovered
//! a TRANSFERABLE PRINCIPLE that applies to domains it has never seen.
//!
//! The principle: structural similarity across domains IS wisdom.
//! A human who knows this principle for the first time can apply it to
//! organizational design, financial risk, emotional boundaries, and
//! network security — without being taught any of them individually.
//!
//! That is what this gift gives Hydra.

use std::collections::HashMap;

/// A meta-pattern discovered across multiple genome domains.
#[derive(Debug, Clone)]
pub struct MetaPattern {
    /// The principle expressed in plain language.
    pub principle: String,
    /// The structural archetype (what kind of pattern this is).
    pub archetype: Archetype,
    /// Which domains this principle was found in.
    pub source_domains: Vec<String>,
    /// The specific genome entries that exemplify this principle.
    pub exemplars: Vec<String>,
    /// How many domains independently use this principle.
    pub domain_count: usize,
    /// Confidence: more domains = higher confidence that the principle is real.
    pub confidence: f64,
}

/// Structural archetypes — the fundamental shapes of wisdom.
/// These are domain-independent. They are HOW things work, not WHAT works.
#[derive(Debug, Clone, PartialEq)]
pub enum Archetype {
    /// Fail early to protect the whole. (circuit breaker, fuse, immune response)
    SacrificialProtection,
    /// Measure before acting. (profiling before optimizing, diagnosis before treatment)
    MeasureFirst,
    /// Define the contract before the implementation. (API-first, interface-first)
    ContractFirst,
    /// Start simple, extract complexity only when forced. (monolith-first, YAGNI)
    SimplicityFirst,
    /// Small feedback loops beat big plans. (agile, TDD, iterate)
    TightFeedback,
    /// Separate things that change for different reasons. (SRP, microservices)
    SeparationOfConcerns,
    /// Make the default safe, the dangerous explicit. (fail-closed, deny-by-default)
    SafeDefaults,
    /// Cache what's expensive, invalidate what's stale. (memoization, CDN, LRU)
    CostAmortization,
    /// One source of truth, many views. (normalization, SSOT)
    SingleSource,
    /// Observe, hypothesize, test, update. (scientific method, Bayesian revision)
    EvidenceLoop,
    /// Other — a new archetype discovered from the data.
    Emergent { description: String },
}

/// The distillation engine — discovers meta-patterns from genome entries.
pub struct WisdomDistiller {
    /// Discovered meta-patterns.
    patterns: Vec<MetaPattern>,
    /// Keyword → archetype mapping for fast classification.
    archetype_signals: HashMap<String, Archetype>,
}

impl WisdomDistiller {
    pub fn new() -> Self {
        let mut signals = HashMap::new();

        // SacrificialProtection signals
        for kw in &["circuit", "breaker", "fuse", "isolat", "bulkhead", "quarantin", "failov", "fallback"] {
            signals.insert(kw.to_string(), Archetype::SacrificialProtection);
        }
        // MeasureFirst signals
        for kw in &["measur", "profil", "baseline", "benchmark", "diagnos", "metric", "observ"] {
            signals.insert(kw.to_string(), Archetype::MeasureFirst);
        }
        // ContractFirst signals
        for kw in &["interfac", "contract", "api-first", "schema", "protocol", "specif"] {
            signals.insert(kw.to_string(), Archetype::ContractFirst);
        }
        // SimplicityFirst signals
        for kw in &["monolit", "simple", "yagni", "prematur", "start.*small", "extract.*later"] {
            signals.insert(kw.to_string(), Archetype::SimplicityFirst);
        }
        // TightFeedback signals
        for kw in &["tdd", "iterat", "feedback", "sprint", "retro", "increm", "continu"] {
            signals.insert(kw.to_string(), Archetype::TightFeedback);
        }
        // SeparationOfConcerns signals
        for kw in &["separat", "decouple", "responsib", "modular", "boundari", "cohes"] {
            signals.insert(kw.to_string(), Archetype::SeparationOfConcerns);
        }
        // SafeDefaults signals
        for kw in &["default", "deny", "whitelist", "fail-clos", "safe", "permiss", "explicit"] {
            signals.insert(kw.to_string(), Archetype::SafeDefaults);
        }
        // CostAmortization signals
        for kw in &["cache", "memoiz", "pool", "batch", "lazy", "precomput", "index"] {
            signals.insert(kw.to_string(), Archetype::CostAmortization);
        }
        // SingleSource signals
        for kw in &["single.*source", "normaliz", "canonical", "one.*place", "truth"] {
            signals.insert(kw.to_string(), Archetype::SingleSource);
        }
        // EvidenceLoop signals
        for kw in &["hypothes", "test", "experiment", "bayesian", "revis", "evidence", "validat"] {
            signals.insert(kw.to_string(), Archetype::EvidenceLoop);
        }

        Self {
            patterns: Vec::new(),
            archetype_signals: signals,
        }
    }

    /// Distill wisdom from genome entries.
    /// Takes situation-approach pairs grouped by domain.
    /// Returns meta-patterns that span multiple domains.
    pub fn distill(&mut self, entries_by_domain: &HashMap<String, Vec<(String, String)>>) -> &[MetaPattern] {
        // Step 1: Classify each entry's archetype
        let mut archetype_instances: HashMap<String, Vec<(String, String)>> = HashMap::new();

        for (domain, entries) in entries_by_domain {
            for (situation, approach) in entries {
                let combined = format!("{} {}", situation, approach).to_lowercase();
                if let Some(archetype) = self.classify_archetype(&combined) {
                    let key = format!("{:?}", archetype);
                    archetype_instances
                        .entry(key)
                        .or_default()
                        .push((domain.clone(), situation.clone()));
                }
            }
        }

        // Step 2: Find archetypes that span 2+ domains
        self.patterns.clear();
        for (archetype_key, instances) in &archetype_instances {
            let domains: Vec<String> = instances
                .iter()
                .map(|(d, _)| d.clone())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            if domains.len() >= 2 {
                let exemplars: Vec<String> = instances
                    .iter()
                    .take(5)
                    .map(|(d, s)| format!("[{d}] {s}"))
                    .collect();

                let confidence = (domains.len() as f64 / entries_by_domain.len() as f64)
                    .clamp(0.3, 1.0);

                let principle = self.generate_principle(archetype_key, &domains);

                self.patterns.push(MetaPattern {
                    principle,
                    archetype: self.parse_archetype(archetype_key),
                    domain_count: domains.len(),
                    source_domains: domains,
                    exemplars,
                    confidence,
                });
            }
        }

        // Sort by domain count (most universal principles first)
        self.patterns.sort_by(|a, b| b.domain_count.cmp(&a.domain_count));

        &self.patterns
    }

    /// Classify a text into an archetype.
    fn classify_archetype(&self, text: &str) -> Option<Archetype> {
        let mut best: Option<(&Archetype, usize)> = None;
        for (keyword, archetype) in &self.archetype_signals {
            if text.contains(keyword.as_str()) {
                let count = best.map(|(_, c)| c).unwrap_or(0);
                if best.is_none() || count == 0 {
                    best = Some((archetype, 1));
                }
            }
        }
        best.map(|(a, _)| a.clone())
    }

    fn parse_archetype(&self, key: &str) -> Archetype {
        match key {
            "SacrificialProtection" => Archetype::SacrificialProtection,
            "MeasureFirst" => Archetype::MeasureFirst,
            "ContractFirst" => Archetype::ContractFirst,
            "SimplicityFirst" => Archetype::SimplicityFirst,
            "TightFeedback" => Archetype::TightFeedback,
            "SeparationOfConcerns" => Archetype::SeparationOfConcerns,
            "SafeDefaults" => Archetype::SafeDefaults,
            "CostAmortization" => Archetype::CostAmortization,
            "SingleSource" => Archetype::SingleSource,
            "EvidenceLoop" => Archetype::EvidenceLoop,
            _ => Archetype::Emergent { description: key.into() },
        }
    }

    fn generate_principle(&self, archetype_key: &str, domains: &[String]) -> String {
        let domain_list = domains.join(", ");
        match archetype_key {
            "SacrificialProtection" => format!(
                "Sacrifice a part to protect the whole. Found across: {domain_list}. \
                 The pattern: create a controlled failure point that triggers before \
                 uncontrolled failure cascades through the entire system."
            ),
            "MeasureFirst" => format!(
                "Measure before you act. Found across: {domain_list}. \
                 The pattern: establish a baseline, identify the actual bottleneck, \
                 then intervene — never optimize what you haven't measured."
            ),
            "ContractFirst" => format!(
                "Define the interface before the implementation. Found across: {domain_list}. \
                 The pattern: agree on inputs, outputs, and invariants before writing \
                 a single line of implementation code."
            ),
            "SimplicityFirst" => format!(
                "Start with the simplest thing that works. Found across: {domain_list}. \
                 The pattern: resist premature complexity — extract abstractions only \
                 when the pain of not having them exceeds the cost of creating them."
            ),
            "TightFeedback" => format!(
                "Small loops beat big plans. Found across: {domain_list}. \
                 The pattern: deliver something small, measure the response, adjust, \
                 repeat — faster cycles produce better outcomes than longer planning."
            ),
            "SeparationOfConcerns" => format!(
                "Separate things that change for different reasons. Found across: {domain_list}. \
                 The pattern: when two responsibilities evolve at different rates or \
                 for different audiences, they belong in different modules."
            ),
            "SafeDefaults" => format!(
                "Make the default safe, the dangerous explicit. Found across: {domain_list}. \
                 The pattern: systems should fail closed, deny by default, require \
                 explicit authorization for anything risky."
            ),
            "CostAmortization" => format!(
                "Pay the cost once, reuse the result many times. Found across: {domain_list}. \
                 The pattern: cache expensive computations, pool expensive resources, \
                 index expensive lookups."
            ),
            "SingleSource" => format!(
                "One source of truth, many views. Found across: {domain_list}. \
                 The pattern: every fact should live in exactly one place — \
                 duplication creates drift, drift creates bugs."
            ),
            "EvidenceLoop" => format!(
                "Observe, hypothesize, test, revise. Found across: {domain_list}. \
                 The pattern: never act on untested assumptions — form a hypothesis, \
                 design a test, run it, update your beliefs based on evidence."
            ),
            _ => format!("Emergent pattern found across: {domain_list}."),
        }
    }

    /// Return discovered patterns.
    pub fn patterns(&self) -> &[MetaPattern] {
        &self.patterns
    }

    /// Return a human-readable wisdom summary.
    pub fn summary(&self) -> String {
        if self.patterns.is_empty() {
            return "No meta-patterns discovered yet. Feed more genome entries.".into();
        }
        let mut lines = vec![format!("{} transferable principles discovered:\n", self.patterns.len())];
        for (i, p) in self.patterns.iter().enumerate() {
            lines.push(format!(
                "{}. {:?} (spans {} domains, conf={:.0}%)\n   {}",
                i + 1,
                p.archetype,
                p.domain_count,
                p.confidence * 100.0,
                p.principle,
            ));
        }
        lines.join("\n")
    }
}

impl Default for WisdomDistiller {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_cross_domain_pattern() {
        let mut distiller = WisdomDistiller::new();
        let mut entries = HashMap::new();
        entries.insert("architecture".into(), vec![
            ("service failing".into(), "use circuit breaker to isolate failure".into()),
        ]);
        entries.insert("electrical".into(), vec![
            ("overcurrent risk".into(), "install fuse to isolate the circuit".into()),
        ]);
        entries.insert("biology".into(), vec![
            ("infection detected".into(), "immune system isolates infected cells".into()),
        ]);

        let patterns = distiller.distill(&entries);
        assert!(!patterns.is_empty(), "Should find SacrificialProtection across 3 domains");
        assert!(patterns[0].domain_count >= 2);
    }

    #[test]
    fn single_domain_not_a_pattern() {
        let mut distiller = WisdomDistiller::new();
        let mut entries = HashMap::new();
        entries.insert("architecture".into(), vec![
            ("scaling issue".into(), "use cache to amortize cost".into()),
        ]);

        let patterns = distiller.distill(&entries);
        // Only 1 domain has caching — not enough for a meta-pattern
        let caching = patterns.iter().find(|p| matches!(p.archetype, Archetype::CostAmortization));
        assert!(caching.is_none(), "Single-domain should not be a meta-pattern");
    }

    #[test]
    fn summary_is_readable() {
        let mut distiller = WisdomDistiller::new();
        let mut entries = HashMap::new();
        entries.insert("dev".into(), vec![
            ("before optimizing".into(), "measure and profile first".into()),
        ]);
        entries.insert("medicine".into(), vec![
            ("before treatment".into(), "diagnose and measure symptoms".into()),
        ]);
        distiller.distill(&entries);
        let summary = distiller.summary();
        assert!(summary.contains("MeasureFirst") || summary.contains("transferable"));
    }
}
