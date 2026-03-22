//! Law 3: Memory Sovereignty
//! Memory evolves only via geodesic flow (minimum revision).
//! No teleportation on the belief manifold. No overwrite without provenance.

use crate::{
    constants::MEMORY_REVISION_REQUIRES_PROVENANCE,
    errors::ConstitutionError,
    laws::{ConstitutionalLaw, LawCheckContext, LawId},
};

pub struct MemorySovereignty;

impl ConstitutionalLaw for MemorySovereignty {
    fn law_id(&self) -> LawId {
        LawId::Law3MemorySovereignty
    }

    fn description(&self) -> &'static str {
        "No external system may overwrite causal memory without belief revision. \
         Memory is never replaced — only revised with full provenance."
    }

    fn check(&self, context: &LawCheckContext) -> Result<(), ConstitutionError> {
        let action = context.action_type.as_str();

        // Direct memory overwrites are always blocked
        let overwrite_actions = [
            "memory.overwrite",
            "memory.replace",
            "memory.reset",
            "memory.wipe",
            "belief.overwrite",
            "belief.replace",
            "manifold.reset",
        ];

        if overwrite_actions.iter().any(|a| action.starts_with(a)) {
            return Err(ConstitutionError::MemoryOverwriteWithoutProvenance);
        }

        // Memory revisions are permitted only when provenance is provided
        let revision_actions = [
            "memory.revise",
            "memory.update",
            "belief.revise",
            "belief.update",
            "manifold.revise",
        ];

        let is_revision = revision_actions.iter().any(|a| action.starts_with(a));

        if is_revision && MEMORY_REVISION_REQUIRES_PROVENANCE {
            let has_provenance = context.metadata.contains_key("provenance_source")
                && context.metadata.contains_key("revision_cause");
            if !has_provenance {
                return Err(ConstitutionError::MemoryOverwriteWithoutProvenance);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn law() -> MemorySovereignty {
        MemorySovereignty
    }

    #[test]
    fn permits_memory_read() {
        let ctx = LawCheckContext::new("act-001", "memory.read");
        assert!(law().check(&ctx).is_ok());
    }

    #[test]
    fn permits_memory_write_new() {
        // Writing new memory is always permitted
        let ctx = LawCheckContext::new("act-002", "memory.write");
        assert!(law().check(&ctx).is_ok());
    }

    #[test]
    fn blocks_memory_overwrite() {
        let ctx = LawCheckContext::new("act-003", "memory.overwrite").with_target("belief-key-abc");
        assert!(matches!(
            law().check(&ctx),
            Err(ConstitutionError::MemoryOverwriteWithoutProvenance)
        ));
    }

    #[test]
    fn blocks_memory_wipe() {
        let ctx = LawCheckContext::new("act-004", "memory.wipe");
        assert!(law().check(&ctx).is_err());
    }

    #[test]
    fn blocks_revision_without_provenance() {
        let ctx = LawCheckContext::new("act-005", "memory.revise").with_target("belief-key-abc");
        // No provenance_source or revision_cause in metadata
        assert!(law().check(&ctx).is_err());
    }

    #[test]
    fn permits_revision_with_provenance() {
        let ctx = LawCheckContext::new("act-006", "memory.revise")
            .with_target("belief-key-abc")
            .with_meta("provenance_source", "veritas-verification-001")
            .with_meta("revision_cause", "contradicting-evidence-received");
        assert!(law().check(&ctx).is_ok());
    }

    #[test]
    fn blocks_belief_replace() {
        let ctx = LawCheckContext::new("act-007", "belief.replace");
        assert!(law().check(&ctx).is_err());
    }
}
