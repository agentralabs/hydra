//! O29: Autonomy Gradient — continuous 0-1 autonomy scoring per action.
//!
//! Replaces binary permission gates with a 4-dimensional continuous score:
//!   autonomy = confidence × reversibility × (1 - blast_radius) × history
//!
//! Each dimension can independently kill autonomy:
//! - Low confidence → don't trust the plan
//! - Low reversibility → can't undo mistakes
//! - High blast → affects too many people
//! - Bad history → failed before

use crate::judgment_gate::BlastRadius;

/// Continuous autonomy score with dimensional breakdown.
#[derive(Debug, Clone)]
pub struct AutonomyScore {
    pub value: f64,
    pub confidence: f64,
    pub reversibility: f64,
    pub blast_radius: f64,
    pub history: f64,
    pub decision: AutonomyDecision,
}

/// Action decision from autonomy gradient.
#[derive(Debug, Clone)]
pub enum AutonomyDecision {
    /// Score > 0.8: execute without asking.
    ActSilently,
    /// Score 0.5-0.8: execute and notify human.
    ActAndNotify { msg: String },
    /// Score 0.2-0.5: ask human before executing.
    AskFirst { question: String },
    /// Score < 0.2: refuse — too risky.
    Refuse { reason: String },
}

impl AutonomyDecision {
    pub fn label(&self) -> &str {
        match self {
            Self::ActSilently => "ACT",
            Self::ActAndNotify { .. } => "NOTIFY",
            Self::AskFirst { .. } => "ASK",
            Self::Refuse { .. } => "REFUSE",
        }
    }
    pub fn can_proceed(&self) -> bool {
        matches!(self, Self::ActSilently | Self::ActAndNotify { .. })
    }
}

/// Compute autonomy score for any action.
pub fn compute_autonomy(
    description: &str,
    confidence: f64,
    blast: &BlastRadius,
    prior_successes: u64,
    prior_failures: u64,
    reversible: bool,
) -> AutonomyScore {
    let conf = confidence.clamp(0.0, 1.0);
    let rev = score_reversibility(description, reversible);
    let blast_w = 1.0 - blast.weight().clamp(0.0, 1.0);
    let hist = if prior_successes + prior_failures == 0 { 0.5 }
        else { prior_successes as f64 / (prior_successes + prior_failures) as f64 };

    // Constitutional override: catastrophic = always ask (Law 6)
    if *blast == BlastRadius::Catastrophic {
        return AutonomyScore {
            value: 0.0, confidence: conf, reversibility: rev,
            blast_radius: blast.weight(), history: hist,
            decision: AutonomyDecision::AskFirst {
                question: format!("Catastrophic blast radius — approval required: {description}"),
            },
        };
    }

    let value = conf * rev * blast_w * hist;

    let decision = if value > 0.8 {
        AutonomyDecision::ActSilently
    } else if value > 0.5 {
        AutonomyDecision::ActAndNotify {
            msg: format!("Executing: {}", &description[..description.len().min(60)]),
        }
    } else if value > 0.2 {
        AutonomyDecision::AskFirst {
            question: format!("Should I {}? (autonomy={:.2}, rev={:.2}, blast={:.2})",
                &description[..description.len().min(40)], value, rev, blast_w),
        }
    } else {
        AutonomyDecision::Refuse {
            reason: format!("Too risky: conf={conf:.2} rev={rev:.2} blast={:.2} hist={hist:.2}",
                blast.weight()),
        }
    };

    AutonomyScore { value, confidence: conf, reversibility: rev,
        blast_radius: blast.weight(), history: hist, decision }
}

/// Score reversibility from 0.0 (permanent) to 1.0 (trivially undoable).
fn score_reversibility(description: &str, has_undo: bool) -> f64 {
    if has_undo { return 0.95; }
    let lower = description.to_lowercase();
    // Permanent actions
    if lower.contains("send email") || lower.contains("send message")
        || lower.contains("post to") || lower.contains("publish") {
        return 0.0;
    }
    // Hard to reverse
    if lower.contains("delete") || lower.contains("remove")
        || lower.contains("drop") || lower.contains("deploy") {
        return 0.15;
    }
    // Reversible via undo
    if lower.contains("save") || lower.contains("write") || lower.contains("edit") {
        return 0.8;
    }
    // Read-only
    if lower.contains("read") || lower.contains("open") || lower.contains("view")
        || lower.contains("search") || lower.contains("check") {
        return 1.0;
    }
    0.5 // unknown default
}

/// Quick helper: compute autonomy from genome query results.
pub fn autonomy_from_genome(
    description: &str,
    genome: &hydra_genome::GenomeStore,
    blast: &BlastRadius,
) -> AutonomyScore {
    let matches = genome.query(description);
    let (conf, successes, failures) = if let Some(entry) = matches.first() {
        (entry.effective_confidence(), entry.success_count, entry.use_count.saturating_sub(entry.success_count))
    } else {
        (0.5, 0, 0)
    };
    compute_autonomy(description, conf, blast, successes, failures, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_confidence_low_risk_acts() {
        let score = compute_autonomy("save file", 0.95, &BlastRadius::Contained, 10, 0, true);
        assert!(score.value > 0.8);
        assert!(matches!(score.decision, AutonomyDecision::ActSilently));
    }

    #[test]
    fn catastrophic_always_asks() {
        let score = compute_autonomy("deploy to prod", 0.99, &BlastRadius::Catastrophic, 100, 0, false);
        assert!(matches!(score.decision, AutonomyDecision::AskFirst { .. }));
    }

    #[test]
    fn low_confidence_irreversible_refuses() {
        let score = compute_autonomy("delete database", 0.1, &BlastRadius::Irreversible, 0, 5, false);
        assert!(score.value < 0.2);
        assert!(matches!(score.decision, AutonomyDecision::Refuse { .. }));
    }

    #[test]
    fn send_email_zero_reversibility() {
        let score = compute_autonomy("send email to all-hands", 0.9, &BlastRadius::Visible, 5, 0, false);
        assert!(score.reversibility == 0.0);
        assert!(score.value < 0.1);
    }
}
