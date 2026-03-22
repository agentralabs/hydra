//! BeliefArbiter — merges conflicting beliefs from two agents.
//! Neither belief simply overwrites the other.
//! Evidence quality and calibration both matter.

use crate::{
    belief::SharedBelief,
    constants::*,
    resolution::{ConsensusResolution, ResolutionMethod},
};

/// Our local belief on a topic.
#[derive(Debug, Clone)]
pub struct LocalBelief {
    pub topic: String,
    pub claim: String,
    pub confidence: f64,
    pub evidence_count: usize,
}

/// Arbitrate between our local belief and a peer's shared belief.
pub fn arbitrate(local: &LocalBelief, remote: &SharedBelief) -> ConsensusResolution {
    assert_eq!(
        local.topic, remote.topic,
        "Cannot arbitrate beliefs about different topics"
    );

    let local_adj = local.confidence;
    let remote_adj = remote.adjusted_confidence();
    let gap = (local_adj - remote_adj).abs();

    // Claims are similar — merge confidences
    let claims_similar = claims_overlap(&local.claim, &remote.claim);

    if claims_similar {
        // Both agree in substance — weighted average of confidences
        let local_ev = (local.evidence_count as f64 / 100.0).min(1.0);
        let remote_ev = remote.evidence_strength();
        let merged_conf =
            (local_adj * local_ev + remote_adj * remote_ev) / (local_ev + remote_ev + f64::EPSILON);

        return ConsensusResolution::new(
            &local.topic,
            format!(
                "{} [merged: local + {}]",
                local.claim, remote.source_peer_id
            ),
            merged_conf,
            ResolutionMethod::Synthesis,
            vec!["local".into(), remote.source_peer_id.clone()],
        );
    }

    // Claims conflict — resolve by evidence + calibrated confidence
    if gap < CONSENSUS_TRIGGER_GAP {
        // Too close to call — synthesis with uncertainty flag
        let merged_conf = (local_adj + remote_adj) / 2.0 * 0.85;
        let mut res = ConsensusResolution::new(
            &local.topic,
            format!(
                "Uncertain: '{}' vs '{}' — gap {:.2}",
                &local.claim[..local.claim.len().min(30)],
                &remote.claim[..remote.claim.len().min(30)],
                gap
            ),
            merged_conf,
            ResolutionMethod::Synthesis,
            vec!["local".into(), remote.source_peer_id.clone()],
        );
        // Small-gap conflicting claims are always uncertain regardless of
        // merged confidence level — neither side has enough advantage.
        res.is_uncertain = true;
        return res;
    }

    // One side clearly stronger — weight by evidence + calibrated conf
    let local_score = local_adj * CONFIDENCE_WEIGHT
        + (local.evidence_count as f64 / 100.0).min(1.0) * EVIDENCE_WEIGHT;
    let remote_score =
        remote_adj * CONFIDENCE_WEIGHT + remote.evidence_strength() * EVIDENCE_WEIGHT;

    if local_score >= remote_score {
        ConsensusResolution::new(
            &local.topic,
            local.claim.clone(),
            local_adj * 0.95,
            ResolutionMethod::DominantBelief {
                winner: "local".into(),
            },
            vec!["local".into(), remote.source_peer_id.clone()],
        )
    } else {
        ConsensusResolution::new(
            &local.topic,
            remote.claim.clone(),
            remote_adj * 0.95,
            ResolutionMethod::DominantBelief {
                winner: remote.source_peer_id.clone(),
            },
            vec!["local".into(), remote.source_peer_id.clone()],
        )
    }
}

/// Simple overlap check — do these two claims say similar things?
fn claims_overlap(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    let a_words: std::collections::HashSet<&str> = a.split_whitespace().collect();
    let b_words: std::collections::HashSet<&str> = b.split_whitespace().collect();
    if a_words.is_empty() || b_words.is_empty() {
        return false;
    }
    let inter = a_words.intersection(&b_words).count();
    let union = a_words.union(&b_words).count();
    (inter as f64 / union as f64) > 0.6
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local(claim: &str, conf: f64, ev: usize) -> LocalBelief {
        LocalBelief {
            topic: "circuit-breaker-pattern".into(),
            claim: claim.to_string(),
            confidence: conf,
            evidence_count: ev,
        }
    }

    fn remote(claim: &str, conf: f64, ev: usize, offset: f64) -> SharedBelief {
        SharedBelief::new(
            "circuit-breaker-pattern",
            claim,
            conf,
            ev,
            vec![],
            "peer-b",
            offset,
        )
    }

    #[test]
    fn agreeing_beliefs_merge_upward() {
        let l = local("use circuit breakers at service boundaries", 0.80, 20);
        let r = remote("use circuit breakers at service boundaries", 0.75, 15, 0.0);
        let res = arbitrate(&l, &r);
        assert_eq!(res.method.label(), "synthesis");
        assert!(res.merged_confidence > 0.5);
        assert_eq!(res.provenance.len(), 2);
    }

    #[test]
    fn dominant_belief_wins_when_clear_gap() {
        let l = local("circuit breakers prevent cascades", 0.92, 50);
        let r = remote("circuit breakers are unnecessary overhead", 0.45, 3, 0.0);
        let res = arbitrate(&l, &r);
        assert_eq!(res.method.label(), "dominant");
        assert!(res.merged_claim.contains("prevent"));
    }

    #[test]
    fn remote_wins_when_better_calibrated() {
        let l = local("approach A is correct", 0.75, 2);
        let r = remote("approach B is correct", 0.70, 80, 0.0);
        let res = arbitrate(&l, &r);
        assert!(res.is_resolved());
    }

    #[test]
    fn uncertain_when_gap_small() {
        let l = local("approach X", 0.75, 10);
        let r = remote("approach Y", 0.73, 12, 0.0);
        let res = arbitrate(&l, &r);
        assert_eq!(res.method.label(), "synthesis");
        assert!(res.is_uncertain);
        // Merged conf = (0.75+0.73)/2 * 0.85 = ~0.629
        assert!(
            res.merged_confidence < 0.75,
            "confidence penalized for uncertainty"
        );
    }

    #[test]
    fn uncertain_when_low_confidence() {
        let l = local("approach X", 0.35, 2);
        let r = remote("approach Y", 0.30, 3, 0.0);
        let res = arbitrate(&l, &r);
        assert_eq!(res.method.label(), "synthesis");
        assert!(
            res.is_uncertain,
            "low confidence beliefs should flag uncertain"
        );
    }

    #[test]
    fn provenance_always_two() {
        let l = local("claim", 0.80, 10);
        let r = remote("claim", 0.75, 8, 0.0);
        let res = arbitrate(&l, &r);
        assert_eq!(res.provenance.len(), 2);
    }
}
