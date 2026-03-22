//! Stance — one participant's position in a diplomacy session.
//! Each instance contributes independently.
//! No participant is "in charge."

use serde::{Deserialize, Serialize};

/// One instance's position on the shared topic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stance {
    pub id: String,
    pub peer_id: String,
    pub topic: String,
    pub position: String,
    pub confidence: f64,
    pub evidence_labels: Vec<String>,
    pub key_concerns: Vec<String>,
    pub submitted_at: chrono::DateTime<chrono::Utc>,
}

impl Stance {
    pub fn new(
        peer_id: impl Into<String>,
        topic: impl Into<String>,
        position: impl Into<String>,
        confidence: f64,
        evidence_labels: Vec<String>,
        key_concerns: Vec<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            peer_id: peer_id.into(),
            topic: topic.into(),
            position: position.into(),
            confidence: confidence.clamp(0.0, 1.0),
            evidence_labels,
            key_concerns,
            submitted_at: chrono::Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stance_created() {
        let s = Stance::new(
            "hydra-a",
            "deployment-strategy",
            "Use canary deployment with 10% traffic split",
            0.88,
            vec!["3yr-ops".into()],
            vec!["rollback-speed".into()],
        );
        assert!(!s.id.is_empty());
        assert_eq!(s.confidence, 0.88);
    }
}
