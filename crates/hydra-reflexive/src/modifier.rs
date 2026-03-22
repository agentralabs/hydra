//! Safe self-modification engine with constitutional checks and rollback.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use hydra_constitution::checker::ConstitutionChecker;
use hydra_constitution::constants::CONSTITUTIONAL_IDENTITY_ID;
use hydra_constitution::laws::LawCheckContext;

use crate::constants::ROLLBACK_WINDOW;
use crate::errors::ReflexiveError;
use crate::model::SelfModel;
use crate::snapshot::SelfSnapshot;

/// The kind of modification being proposed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModificationKind {
    /// Add a new capability.
    AddCapability,
    /// Remove a capability (must pass growth check).
    RemoveCapability,
    /// Update capability status.
    UpdateStatus,
    /// Full model restructure.
    Restructure,
}

impl std::fmt::Display for ModificationKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AddCapability => write!(f, "self.modify.add_capability"),
            Self::RemoveCapability => write!(f, "self.modify.remove_capability"),
            Self::UpdateStatus => write!(f, "self.modify.update_status"),
            Self::Restructure => write!(f, "self.modify.restructure"),
        }
    }
}

/// A proposed modification to the self-model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModificationProposal {
    /// Unique proposal identifier.
    pub id: String,
    /// What kind of modification.
    pub kind: ModificationKind,
    /// Human-readable description of what will change.
    pub description: String,
    /// The causal root that initiated this modification.
    pub causal_root: String,
    /// When this proposal was created.
    pub proposed_at: DateTime<Utc>,
}

impl ModificationProposal {
    /// Create a new modification proposal.
    pub fn new(
        kind: ModificationKind,
        description: impl Into<String>,
        causal_root: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            kind,
            description: description.into(),
            causal_root: causal_root.into(),
            proposed_at: Utc::now(),
        }
    }
}

/// Safe self-modification engine.
/// All modifications go through constitutional check, snapshot, apply, verify.
pub struct SafeModifier {
    checker: ConstitutionChecker,
    snapshots: Vec<SelfSnapshot>,
}

impl SafeModifier {
    /// Create a new safe modifier with the constitutional checker.
    pub fn new() -> Self {
        Self {
            checker: ConstitutionChecker::new(),
            snapshots: Vec::new(),
        }
    }

    /// Apply a modification proposal to the self-model.
    ///
    /// Steps:
    /// 1. Constitutional check (must pass)
    /// 2. Snapshot current state
    /// 3. Apply the modification via the closure
    /// 4. Verify growth invariant (total_ever must not decrease)
    /// 5. Rollback if invariant violated
    pub fn apply<F>(
        &mut self,
        model: &mut SelfModel,
        proposal: &ModificationProposal,
        modify_fn: F,
    ) -> Result<(), ReflexiveError>
    where
        F: FnOnce(&mut SelfModel) -> Result<(), ReflexiveError>,
    {
        // Step 1: Constitutional check
        let ctx =
            LawCheckContext::new(&proposal.id, proposal.kind.to_string()).with_causal_chain(vec![
                proposal.causal_root.clone(),
                CONSTITUTIONAL_IDENTITY_ID.to_string(),
            ]);

        let result = self.checker.check(&ctx);
        if !result.is_permitted() {
            let reason = result
                .first_violation()
                .map(|v| v.to_string())
                .unwrap_or_else(|| "unknown constitutional violation".to_string());
            return Err(ReflexiveError::ModificationBlocked { reason });
        }

        // Step 2: Snapshot current state
        let snapshot = SelfSnapshot::capture(model)?;
        let pre_total_ever = model.total_ever;

        // Step 3: Apply the modification
        let apply_result = modify_fn(model);

        // Step 4: Verify growth invariant
        if let Ok(()) = &apply_result {
            if model.total_ever < pre_total_ever {
                // Step 5: Rollback — growth invariant violated
                let restored = snapshot.restore()?;
                *model = restored;
                return Err(ReflexiveError::GrowthInvariantViolated {
                    before: pre_total_ever,
                    after: model.total_ever,
                });
            }
        }

        // Store snapshot for future rollback
        self.snapshots.push(snapshot);
        if self.snapshots.len() > ROLLBACK_WINDOW {
            self.snapshots.remove(0);
        }

        apply_result
    }

    /// Rollback the model to the last snapshot.
    pub fn rollback_last(&mut self, model: &mut SelfModel) -> Result<(), ReflexiveError> {
        let snapshot = self
            .snapshots
            .pop()
            .ok_or_else(|| ReflexiveError::RollbackNotFound {
                snapshot_id: "no snapshots available".to_string(),
            })?;
        let restored = snapshot.restore()?;
        *model = restored;
        Ok(())
    }

    /// Return the number of snapshots available for rollback.
    pub fn snapshot_count(&self) -> usize {
        self.snapshots.len()
    }
}

impl Default for SafeModifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::CapabilitySource;

    #[test]
    fn apply_adds_capability_and_stores_snapshot() {
        let mut model = SelfModel::bootstrap_layer1();
        let mut modifier = SafeModifier::new();

        let proposal = ModificationProposal::new(
            ModificationKind::AddCapability,
            "Add test capability",
            "test-root",
        );

        let result = modifier.apply(&mut model, &proposal, |m| {
            m.add_capability(
                "new-cap",
                CapabilitySource::Skill {
                    skill_id: "s1".into(),
                },
            )
        });

        assert!(result.is_ok());
        assert_eq!(model.capabilities.len(), 6);
        assert_eq!(modifier.snapshot_count(), 1);
    }

    #[test]
    fn rollback_restores_previous_state() {
        let mut model = SelfModel::bootstrap_layer1();
        let mut modifier = SafeModifier::new();
        let original_count = model.capabilities.len();

        let proposal = ModificationProposal::new(
            ModificationKind::AddCapability,
            "Add capability then rollback",
            "test-root",
        );

        modifier
            .apply(&mut model, &proposal, |m| {
                m.add_capability(
                    "temp-cap",
                    CapabilitySource::Skill {
                        skill_id: "s1".into(),
                    },
                )
            })
            .expect("apply");

        assert_eq!(model.capabilities.len(), original_count + 1);

        modifier.rollback_last(&mut model).expect("rollback");
        assert_eq!(model.capabilities.len(), original_count);
    }
}
