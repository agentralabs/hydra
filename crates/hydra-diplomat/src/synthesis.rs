//! StanceSynthesizer — produces a joint recommendation from multiple stances.
//! Agreement is measured. Disagreements are preserved, never suppressed.

use crate::{constants::MIN_AGREEMENT_FRACTION, errors::DiplomatError, session::DiplomacySession};
#[allow(unused_imports)]
use crate::stance::Stance; // Used in tests via `use super::*`
use serde::{Deserialize, Serialize};

/// One participant's agreement/disagreement with the joint recommendation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantResponse {
    pub peer_id: String,
    pub agrees: bool,
    pub note: Option<String>,
}

/// The synthesized joint recommendation from all stances.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JointRecommendation {
    pub id: String,
    pub topic: String,
    pub recommendation: String,
    pub merged_confidence: f64,
    pub agreement_fraction: f64,
    pub participant_count: usize,
    pub participant_responses: Vec<ParticipantResponse>,
    /// Minority positions — preserved, never suppressed.
    pub minority_positions: Vec<String>,
    pub synthesized_at: chrono::DateTime<chrono::Utc>,
}

impl JointRecommendation {
    pub fn is_consensus(&self) -> bool {
        self.agreement_fraction >= MIN_AGREEMENT_FRACTION
    }

    pub fn summary_line(&self) -> String {
        format!(
            "[{}] agreement={:.0}% conf={:.2} participants={}",
            self.topic,
            self.agreement_fraction * 100.0,
            self.merged_confidence,
            self.participant_count,
        )
    }
}

/// Synthesize stances into a joint recommendation.
pub fn synthesize(session: &DiplomacySession) -> Result<JointRecommendation, DiplomatError> {
    if session.stances.len() < crate::constants::MIN_PARTICIPANTS {
        return Err(DiplomatError::InsufficientParticipants {
            count: session.stances.len(),
            min: crate::constants::MIN_PARTICIPANTS,
        });
    }

    let total_conf: f64 = session.stances.iter().map(|s| s.confidence).sum();
    let avg_conf = total_conf / session.stances.len() as f64;

    // The dominant position is the one with the highest confidence
    let dominant = session
        .stances
        .iter()
        .max_by(|a, b| {
            a.confidence
                .partial_cmp(&b.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .expect("at least MIN_PARTICIPANTS stances");

    // Count agreements (stances with similar positions)
    let agreements = session
        .stances
        .iter()
        .filter(|s| positions_compatible(&s.position, &dominant.position))
        .count();

    let agreement_fraction = agreements as f64 / session.stances.len() as f64;

    if agreement_fraction < MIN_AGREEMENT_FRACTION {
        return Err(DiplomatError::NoAgreement {
            reason: format!(
                "Only {:.0}% agreement (need {:.0}%)",
                agreement_fraction * 100.0,
                MIN_AGREEMENT_FRACTION * 100.0,
            ),
        });
    }

    // Minority positions — preserved
    let minority_positions: Vec<String> = session
        .stances
        .iter()
        .filter(|s| !positions_compatible(&s.position, &dominant.position))
        .map(|s| format!("{}: {}", s.peer_id, &s.position[..s.position.len().min(60)]))
        .collect();

    // All key concerns from all participants
    let all_concerns: Vec<String> = session
        .stances
        .iter()
        .flat_map(|s| s.key_concerns.iter().cloned())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let recommendation = format!(
        "{}. Key concerns noted: {}",
        dominant.position,
        if all_concerns.is_empty() {
            "none".into()
        } else {
            all_concerns.join("; ")
        }
    );

    let participant_responses: Vec<ParticipantResponse> = session
        .stances
        .iter()
        .map(|s| {
            let agrees = positions_compatible(&s.position, &dominant.position);
            ParticipantResponse {
                peer_id: s.peer_id.clone(),
                agrees,
                note: if !agrees {
                    Some(format!(
                        "Minority position: {}",
                        &s.position[..s.position.len().min(40)]
                    ))
                } else {
                    None
                },
            }
        })
        .collect();

    Ok(JointRecommendation {
        id: uuid::Uuid::new_v4().to_string(),
        topic: session.topic.clone(),
        recommendation,
        merged_confidence: avg_conf,
        agreement_fraction,
        participant_count: session.stances.len(),
        participant_responses,
        minority_positions,
        synthesized_at: chrono::Utc::now(),
    })
}

fn positions_compatible(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    let a_w: std::collections::HashSet<&str> = a.split_whitespace().collect();
    let b_w: std::collections::HashSet<&str> = b.split_whitespace().collect();
    if a_w.is_empty() || b_w.is_empty() {
        return false;
    }
    let inter = a_w.intersection(&b_w).count();
    let union = a_w.union(&b_w).count();
    (inter as f64 / union as f64) >= 0.40
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::DiplomacySession;

    fn make_session_with_stances(topic: &str, positions: &[(&str, &str, f64)]) -> DiplomacySession {
        let mut s = DiplomacySession::new(topic);
        for (peer, pos, conf) in positions {
            s.submit_stance(Stance::new(*peer, topic, *pos, *conf, vec![], vec![]))
                .expect("submit stance");
        }
        s
    }

    #[test]
    fn synthesis_succeeds_with_majority() {
        let session = make_session_with_stances(
            "deploy-strategy",
            &[
                (
                    "hydra-a",
                    "use canary deployment with gradual rollout",
                    0.88,
                ),
                (
                    "hydra-b",
                    "use canary deployment with gradual rollout",
                    0.84,
                ),
                (
                    "hydra-c",
                    "use canary deployment with gradual rollout",
                    0.79,
                ),
            ],
        );
        let rec = synthesize(&session).expect("should synthesize");
        assert!(rec.is_consensus());
        assert_eq!(rec.participant_count, 3);
        assert!(rec.minority_positions.is_empty());
    }

    #[test]
    fn minority_positions_preserved() {
        let session = make_session_with_stances(
            "deploy-strategy",
            &[
                (
                    "hydra-a",
                    "use canary deployment with gradual rollout",
                    0.88,
                ),
                (
                    "hydra-b",
                    "use canary deployment with gradual rollout",
                    0.84,
                ),
                ("hydra-c", "big bang release is acceptable here", 0.55),
            ],
        );
        let rec = synthesize(&session).expect("should synthesize");
        assert_eq!(rec.minority_positions.len(), 1);
        assert!(rec.minority_positions[0].contains("hydra-c"));
    }

    #[test]
    fn no_agreement_when_split() {
        let session = make_session_with_stances(
            "critical-question",
            &[
                (
                    "hydra-a",
                    "deploy immediately to production without delay",
                    0.80,
                ),
                (
                    "hydra-b",
                    "halt everything pending full security audit review",
                    0.80,
                ),
            ],
        );
        let r = synthesize(&session);
        // 50% agreement (each only agrees with itself), need 60%
        assert!(r.is_err());
    }
}
