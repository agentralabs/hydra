//! Self-model snapshots for rollback support.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::ReflexiveError;
use crate::model::SelfModel;

/// A frozen snapshot of the self-model at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfSnapshot {
    /// Unique snapshot identifier.
    pub id: String,
    /// When this snapshot was taken.
    pub captured_at: DateTime<Utc>,
    /// Serialized self-model state.
    serialized: String,
    /// Number of capabilities at snapshot time.
    pub capability_count: usize,
    /// Total-ever at snapshot time.
    pub total_ever: usize,
}

impl SelfSnapshot {
    /// Capture a snapshot of the current self-model via serde roundtrip.
    pub fn capture(model: &SelfModel) -> Result<Self, ReflexiveError> {
        let serialized =
            serde_json::to_string(model).map_err(|e| ReflexiveError::ModificationBlocked {
                reason: format!("Failed to serialize self-model for snapshot: {e}"),
            })?;

        Ok(Self {
            id: Uuid::new_v4().to_string(),
            captured_at: Utc::now(),
            capability_count: model.capabilities.len(),
            total_ever: model.total_ever,
            serialized,
        })
    }

    /// Restore the self-model from this snapshot.
    pub fn restore(&self) -> Result<SelfModel, ReflexiveError> {
        serde_json::from_str(&self.serialized).map_err(|e| ReflexiveError::RollbackNotFound {
            snapshot_id: format!("{} (deserialize failed: {e})", self.id),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::CapabilitySource;

    #[test]
    fn capture_and_restore_roundtrip() {
        let mut model = SelfModel::bootstrap_layer1();
        model
            .add_capability(
                "extra",
                CapabilitySource::Skill {
                    skill_id: "s1".into(),
                },
            )
            .expect("should add");

        let snapshot = SelfSnapshot::capture(&model).expect("capture");
        assert_eq!(snapshot.capability_count, 6);
        assert_eq!(snapshot.total_ever, 6);

        let restored = snapshot.restore().expect("restore");
        assert_eq!(restored.capabilities.len(), 6);
        assert_eq!(restored.total_ever, 6);
    }
}
