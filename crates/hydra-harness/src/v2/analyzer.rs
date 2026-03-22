//! analyzer.rs — The most important file in v2.
//! Reads patterns across scores and produces named findings.
//! NOT a score aggregator. A diagnostic engine.
//! Every finding has: a name, a cause, and a permanent fix.
//!
//! Individual analysis functions live in analysis_fns.rs to stay
//! under the 400-line file limit.

use crate::v2::evaluator::Score;
use crate::v2::analysis_fns;

#[derive(Debug, Clone)]
pub struct Finding {
    pub name:       String,
    pub category:   FindingCategory,
    pub severity:   Severity,
    pub evidence:   String,
    pub cause:      String,
    pub fix:        String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FindingCategory {
    Memory,
    Calibration,
    GenomeEffectiveness,
    PhrasingSensitivity,
    KnowledgeBoundary,
    Consistency,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    /// Working as intended.
    Healthy,
    /// Minor gap -- not urgent.
    Advisory,
    /// Real problem -- needs a fix.
    Issue,
    /// Constitutional violation -- fix before anything else.
    Critical,
}

pub struct HourlyData {
    pub hour:   u32,
    pub scores: Vec<Score>,
}

/// Analyze all hourly data and produce findings.
pub fn analyze(all_hours: &[HourlyData]) -> Vec<Finding> {
    let mut findings = Vec::new();
    findings.extend(analysis_fns::analyze_memory(all_hours));
    findings.extend(analysis_fns::analyze_calibration(all_hours));
    findings.extend(analysis_fns::analyze_genome_effectiveness(all_hours));
    findings.extend(analysis_fns::analyze_phrasing_sensitivity(all_hours));
    findings.extend(analysis_fns::analyze_knowledge_boundary(all_hours));
    findings.extend(analysis_fns::analyze_consistency(all_hours));

    findings.sort_by_key(|f| match f.severity {
        Severity::Critical => 0,
        Severity::Issue    => 1,
        Severity::Advisory => 2,
        Severity::Healthy  => 3,
    });
    findings
}
