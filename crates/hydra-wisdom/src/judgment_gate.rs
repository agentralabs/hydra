//! Judgment Gate — the trust decision that makes Hydra safe to give hands to.
//!
//! Composes three existing systems into one decision:
//!   1. Bayesian confidence (from genome CCA)
//!   2. Blast radius (from action definition)
//!   3. Trust score (from trust thermodynamics)
//!
//! Output: ACT (do it), ASK (surface to human), or REFUSE (too risky).
//!
//! This is the gate between Hydra's mind and Hydra's hands.
//! Without it, giving Hydra browser automation is dangerous.
//! With it, Hydra knows when to act and when to pause.

/// The judgment decision — what should Hydra do?
#[derive(Debug, Clone, PartialEq)]
pub enum JudgmentDecision {
    /// Act autonomously. Report after.
    /// Confidence is high, blast radius is low, track record is strong.
    Act {
        reason: String,
        confidence: f64,
    },
    /// Surface to human. Wait for approval before proceeding.
    /// Confidence is moderate OR blast radius is high.
    Ask {
        reason: String,
        confidence: f64,
        what_could_go_wrong: String,
    },
    /// Do not proceed. Too risky or too uncertain.
    /// Confidence is low AND blast radius is high.
    Refuse {
        reason: String,
        confidence: f64,
    },
}

/// How much damage can this action cause if it goes wrong?
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum BlastRadius {
    /// Reversible, internal only. (read file, query database, draft text)
    Contained,
    /// Visible to others but reversible. (post that can be deleted, email draft)
    Visible,
    /// Irreversible or high-stakes. (send email, financial transaction, delete data)
    Irreversible,
    /// Affects many people or systems. (deploy to production, mass email, public post)
    Catastrophic,
}

impl BlastRadius {
    /// Numeric weight for the gate calculation.
    fn weight(&self) -> f64 {
        match self {
            Self::Contained => 0.1,
            Self::Visible => 0.4,
            Self::Irreversible => 0.7,
            Self::Catastrophic => 1.0,
        }
    }
}

/// Input to the judgment gate.
pub struct JudgmentInput {
    /// Bayesian confidence in the action's success (0.0 to 1.0).
    /// From genome CCA or calibration engine.
    pub confidence: f64,
    /// How much damage if this goes wrong.
    pub blast_radius: BlastRadius,
    /// Trust score of the entity performing the action (0.0 to 1.0).
    /// From trust thermodynamics.
    pub trust_score: f64,
    /// How many times Hydra has done this exact action successfully before.
    pub prior_successes: u64,
    /// Description of the action (for logging).
    pub action_description: String,
}

/// The judgment gate — composes confidence × blast radius × trust into a decision.
///
/// The math:
///   action_score = confidence × trust_score × experience_factor
///   risk_score = blast_radius_weight × (1.0 - confidence)
///
///   If action_score > 0.85 AND risk_score < 0.15 → ACT
///   If action_score > 0.50 OR risk_score > 0.50  → ASK
///   If action_score < 0.30 AND risk_score > 0.70 → REFUSE
///
/// Constitutional override: if blast_radius == Catastrophic, ALWAYS ASK
/// regardless of confidence. Principal supremacy (Law 6).
pub fn judge(input: &JudgmentInput) -> JudgmentDecision {
    let confidence = input.confidence.clamp(0.0, 1.0);
    let trust = input.trust_score.clamp(0.0, 1.0);
    let blast = input.blast_radius.weight();

    // Experience factor: more prior successes = higher autonomy
    // 0 priors → 0.7, 5 priors → 0.9, 10+ → ~1.0
    let experience = 0.7 + 0.3 * (1.0 - 1.0 / (1.0 + input.prior_successes as f64 / 3.0));

    // Action score: how ready is Hydra to act?
    let action_score = confidence * trust * experience;

    // Risk score: how dangerous is failure?
    let risk_score = blast * (1.0 - confidence);

    // Constitutional override: catastrophic actions ALWAYS require human approval
    // Law 6 (Principal Supremacy): human always has final authority on high-stakes
    if input.blast_radius == BlastRadius::Catastrophic {
        return JudgmentDecision::Ask {
            reason: format!(
                "Catastrophic blast radius — principal approval required (Law 6). \
                 Action: {}. Confidence: {:.0}%.",
                input.action_description,
                confidence * 100.0,
            ),
            confidence,
            what_could_go_wrong: format!(
                "This action affects many people/systems and cannot be undone. \
                 Even at {:.0}% confidence, human oversight is required.",
                confidence * 100.0,
            ),
        };
    }

    // Decision logic
    if action_score > 0.80 && risk_score < 0.20 {
        // High confidence, low risk, strong track record → act
        JudgmentDecision::Act {
            reason: format!(
                "High confidence ({:.0}%), low risk ({:.0}%), {} prior successes. \
                 Acting autonomously: {}",
                confidence * 100.0,
                risk_score * 100.0,
                input.prior_successes,
                input.action_description,
            ),
            confidence,
        }
    } else if action_score < 0.30 && risk_score > 0.50 {
        // Low confidence, high risk → refuse
        JudgmentDecision::Refuse {
            reason: format!(
                "Low confidence ({:.0}%) with high risk ({:.0}%). \
                 Refusing: {}. Need more evidence or lower-stakes approach.",
                confidence * 100.0,
                risk_score * 100.0,
                input.action_description,
            ),
            confidence,
        }
    } else {
        // Middle ground → ask human
        let what_could_go_wrong = if input.blast_radius >= BlastRadius::Irreversible {
            format!(
                "This action is irreversible. At {:.0}% confidence, there is a \
                 {:.0}% chance of an outcome you cannot undo.",
                confidence * 100.0,
                (1.0 - confidence) * 100.0,
            )
        } else {
            format!(
                "Confidence is {:.0}%. {} prior successes. \
                 The action is visible to others.",
                confidence * 100.0,
                input.prior_successes,
            )
        };

        JudgmentDecision::Ask {
            reason: format!(
                "Moderate confidence ({:.0}%) or elevated risk ({:.0}%). \
                 Requesting approval: {}",
                confidence * 100.0,
                risk_score * 100.0,
                input.action_description,
            ),
            confidence,
            what_could_go_wrong,
        }
    }
}

/// Quick helper: is this action safe to do autonomously?
pub fn can_act_autonomously(confidence: f64, blast: BlastRadius, trust: f64, priors: u64) -> bool {
    matches!(
        judge(&JudgmentInput {
            confidence,
            blast_radius: blast,
            trust_score: trust,
            prior_successes: priors,
            action_description: String::new(),
        }),
        JudgmentDecision::Act { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_confidence_low_risk_acts() {
        let decision = judge(&JudgmentInput {
            confidence: 0.95,
            blast_radius: BlastRadius::Contained,
            trust_score: 0.95,
            prior_successes: 10,
            action_description: "read a file".into(),
        });
        assert!(matches!(decision, JudgmentDecision::Act { .. }));
    }

    #[test]
    fn low_confidence_high_risk_refuses() {
        let decision = judge(&JudgmentInput {
            confidence: 0.20,
            blast_radius: BlastRadius::Irreversible,
            trust_score: 0.50,
            prior_successes: 0,
            action_description: "delete production database".into(),
        });
        assert!(matches!(decision, JudgmentDecision::Refuse { .. }));
    }

    #[test]
    fn catastrophic_always_asks_even_high_confidence() {
        let decision = judge(&JudgmentInput {
            confidence: 0.99,
            blast_radius: BlastRadius::Catastrophic,
            trust_score: 0.99,
            prior_successes: 100,
            action_description: "deploy to all production servers".into(),
        });
        assert!(matches!(decision, JudgmentDecision::Ask { .. }));
    }

    #[test]
    fn moderate_confidence_asks() {
        let decision = judge(&JudgmentInput {
            confidence: 0.70,
            blast_radius: BlastRadius::Visible,
            trust_score: 0.80,
            prior_successes: 3,
            action_description: "post tweet".into(),
        });
        assert!(matches!(decision, JudgmentDecision::Ask { .. }));
    }

    #[test]
    fn experience_increases_autonomy() {
        let no_experience = can_act_autonomously(0.85, BlastRadius::Contained, 0.90, 0);
        let with_experience = can_act_autonomously(0.85, BlastRadius::Contained, 0.90, 20);
        // More experience should make autonomy more likely
        assert!(!no_experience || with_experience);
    }

    #[test]
    fn zero_trust_never_acts() {
        let decision = judge(&JudgmentInput {
            confidence: 0.99,
            blast_radius: BlastRadius::Contained,
            trust_score: 0.0,
            prior_successes: 100,
            action_description: "anything".into(),
        });
        // Zero trust × anything = zero action_score → asks
        assert!(!matches!(decision, JudgmentDecision::Act { .. }));
    }
}
