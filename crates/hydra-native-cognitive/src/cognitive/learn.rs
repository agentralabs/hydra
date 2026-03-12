//! LEARN phase — belief decay, reconfirmation, and lifecycle management.
//!
//! Phase 3, C3: Beliefs that aren't re-confirmed lose confidence over time.
//! This prevents Hydra from acting on stale assumptions.

use hydra_belief::belief::{Belief, BeliefCategory};

/// Applies time-based confidence decay to all beliefs.
/// Uses exponential decay with category-specific floors so beliefs never
/// fully vanish — some things remain probably true even without reconfirmation.
///
/// Called during the LEARN phase and by the consolidation daemon.
pub fn apply_belief_decay(beliefs: &mut Vec<Belief>, decay_rate_per_day: f64) {
    let now = chrono::Utc::now();

    for belief in beliefs.iter_mut() {
        if !belief.active {
            continue;
        }

        let age_days = (now - belief.updated_at).num_seconds().max(0) as f64 / 86400.0;
        if age_days < 0.01 {
            continue; // Skip beliefs updated less than ~15 minutes ago
        }

        let decay_factor = (1.0 - decay_rate_per_day).powf(age_days);

        // Category-specific confidence floors
        let floor = match belief.category {
            BeliefCategory::Preference => 0.3,
            BeliefCategory::Fact       => 0.5,
            BeliefCategory::Convention => 0.4,
            BeliefCategory::Correction => 0.6, // Corrections are high-signal
        };

        let new_confidence = (belief.confidence as f64 * decay_factor).max(floor) as f32;

        if new_confidence < belief.confidence {
            belief.confidence = new_confidence;
        }
    }
}

/// Re-confirms a belief, resetting its decay clock and boosting confidence.
/// Call this when user mentions something consistent with an existing belief.
pub fn reconfirm_belief(belief: &mut Belief) {
    belief.confirm(); // Uses the existing confirm() method which bumps confidence +0.02
}

/// Deactivate beliefs that have decayed below a threshold.
/// Returns the number of beliefs deactivated.
pub fn gc_expired_beliefs(beliefs: &mut Vec<Belief>, min_confidence: f32) -> usize {
    let mut count = 0;
    for belief in beliefs.iter_mut() {
        if belief.active && belief.confidence < min_confidence {
            belief.active = false;
            count += 1;
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydra_belief::belief::{BeliefSource};

    fn make_belief(category: BeliefCategory, confidence: f32, days_old: i64) -> Belief {
        let mut b = Belief::new(category, "test subject", "test content", BeliefSource::UserStated);
        b.confidence = confidence;
        // Set updated_at to N days ago
        b.updated_at = chrono::Utc::now() - chrono::Duration::days(days_old);
        b
    }

    #[test]
    fn test_decay_reduces_confidence() {
        let mut beliefs = vec![make_belief(BeliefCategory::Fact, 0.95, 30)];
        apply_belief_decay(&mut beliefs, 0.05);
        assert!(beliefs[0].confidence < 0.95, "30-day-old belief should lose confidence");
    }

    #[test]
    fn test_decay_respects_fact_floor() {
        let mut beliefs = vec![make_belief(BeliefCategory::Fact, 0.95, 365)];
        apply_belief_decay(&mut beliefs, 0.05);
        assert!(
            beliefs[0].confidence >= 0.5,
            "Fact floor is 0.5, got {}",
            beliefs[0].confidence
        );
    }

    #[test]
    fn test_decay_respects_preference_floor() {
        let mut beliefs = vec![make_belief(BeliefCategory::Preference, 0.8, 365)];
        apply_belief_decay(&mut beliefs, 0.05);
        assert!(
            beliefs[0].confidence >= 0.3,
            "Preference floor is 0.3, got {}",
            beliefs[0].confidence
        );
    }

    #[test]
    fn test_decay_respects_correction_floor() {
        let mut beliefs = vec![make_belief(BeliefCategory::Correction, 0.99, 365)];
        apply_belief_decay(&mut beliefs, 0.05);
        assert!(
            beliefs[0].confidence >= 0.6,
            "Correction floor is 0.6, got {}",
            beliefs[0].confidence
        );
    }

    #[test]
    fn test_decay_skips_inactive() {
        let mut beliefs = vec![make_belief(BeliefCategory::Fact, 0.5, 30)];
        beliefs[0].active = false;
        let before = beliefs[0].confidence;
        apply_belief_decay(&mut beliefs, 0.05);
        assert_eq!(beliefs[0].confidence, before, "Inactive beliefs should not decay");
    }

    #[test]
    fn test_decay_skips_recent() {
        let mut beliefs = vec![make_belief(BeliefCategory::Fact, 0.95, 0)];
        apply_belief_decay(&mut beliefs, 0.05);
        assert!(
            (beliefs[0].confidence - 0.95).abs() < 0.01,
            "Just-updated belief should not decay significantly"
        );
    }

    #[test]
    fn test_reconfirm_boosts_confidence() {
        let mut b = make_belief(BeliefCategory::Fact, 0.6, 30);
        let before = b.confidence;
        reconfirm_belief(&mut b);
        assert!(b.confidence > before, "Reconfirmation should boost confidence");
        assert_eq!(b.confirmations, 1);
    }

    #[test]
    fn test_gc_deactivates_low_confidence() {
        let mut beliefs = vec![
            make_belief(BeliefCategory::Fact, 0.05, 0),
            make_belief(BeliefCategory::Fact, 0.8, 0),
            make_belief(BeliefCategory::Fact, 0.02, 0),
        ];
        let count = gc_expired_beliefs(&mut beliefs, 0.1);
        assert_eq!(count, 2);
        assert!(!beliefs[0].active);
        assert!(beliefs[1].active);
        assert!(!beliefs[2].active);
    }

    #[test]
    fn test_gc_ignores_already_inactive() {
        let mut beliefs = vec![make_belief(BeliefCategory::Fact, 0.05, 0)];
        beliefs[0].active = false;
        let count = gc_expired_beliefs(&mut beliefs, 0.1);
        assert_eq!(count, 0, "Already-inactive beliefs shouldn't count");
    }
}
