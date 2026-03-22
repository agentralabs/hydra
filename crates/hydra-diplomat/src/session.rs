//! DiplomacySession — a structured coordination round.
//! Stances submitted -> synthesis -> joint recommendation.

use crate::{constants::*, errors::DiplomatError, stance::Stance};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Session state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SessionState {
    Open,
    Synthesizing,
    Concluded { recommendation_id: String },
    Failed { reason: String },
}

impl SessionState {
    pub fn is_open(&self) -> bool {
        matches!(self, Self::Open)
    }
    pub fn is_concluded(&self) -> bool {
        matches!(self, Self::Concluded { .. })
    }
    pub fn label(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Synthesizing => "synthesizing",
            Self::Concluded { .. } => "concluded",
            Self::Failed { .. } => "failed",
        }
    }
}

/// One diplomacy session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiplomacySession {
    pub id: String,
    pub topic: String,
    pub state: SessionState,
    pub stances: Vec<Stance>,
    pub session_hash: String,
    pub opened_at: chrono::DateTime<chrono::Utc>,
    pub closed_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl DiplomacySession {
    pub fn new(topic: impl Into<String>) -> Self {
        let now = chrono::Utc::now();
        let topic = topic.into();
        let hash = {
            let mut h = Sha256::new();
            h.update(topic.as_bytes());
            h.update(now.to_rfc3339().as_bytes());
            hex::encode(h.finalize())
        };
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            topic,
            state: SessionState::Open,
            stances: Vec::new(),
            session_hash: hash,
            opened_at: now,
            closed_at: None,
        }
    }

    /// Submit a stance from one participant.
    pub fn submit_stance(&mut self, stance: Stance) -> Result<(), DiplomatError> {
        if !self.state.is_open() {
            return Err(DiplomatError::SessionClosed {
                id: self.id.clone(),
            });
        }
        if self.stances.iter().any(|s| s.peer_id == stance.peer_id) {
            return Err(DiplomatError::DuplicateStance {
                peer_id: stance.peer_id.clone(),
            });
        }
        if self.stances.len() >= MAX_PARTICIPANTS {
            return Err(DiplomatError::InsufficientParticipants {
                count: MAX_PARTICIPANTS + 1,
                min: MIN_PARTICIPANTS,
            });
        }
        self.stances.push(stance);
        Ok(())
    }

    pub fn participant_count(&self) -> usize {
        self.stances.len()
    }
    pub fn verify_hash(&self) -> bool {
        self.session_hash.len() == 64
    }
}
