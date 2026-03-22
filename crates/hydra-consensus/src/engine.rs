//! ConsensusEngine — the coordinator.

use crate::{
    arbiter::{arbitrate, LocalBelief},
    belief::SharedBelief,
    constants::MAX_BELIEFS_PER_SESSION,
    errors::ConsensusError,
    resolution::ConsensusResolution,
};

/// The consensus engine.
pub struct ConsensusEngine {
    resolutions: Vec<ConsensusResolution>,
}

impl ConsensusEngine {
    pub fn new() -> Self {
        Self {
            resolutions: Vec::new(),
        }
    }

    /// Resolve a conflict between our belief and a peer's.
    pub fn resolve(
        &mut self,
        local_topic: &str,
        local_claim: &str,
        local_confidence: f64,
        local_evidence: usize,
        remote: &SharedBelief,
    ) -> Result<&ConsensusResolution, ConsensusError> {
        if self.resolutions.len() >= MAX_BELIEFS_PER_SESSION {
            self.resolutions.remove(0);
        }

        let local = LocalBelief {
            topic: local_topic.to_string(),
            claim: local_claim.to_string(),
            confidence: local_confidence,
            evidence_count: local_evidence,
        };

        let resolution = arbitrate(&local, remote);
        self.resolutions.push(resolution);
        Ok(self.resolutions.last().expect("just pushed"))
    }

    pub fn resolution_count(&self) -> usize {
        self.resolutions.len()
    }

    pub fn uncertain_count(&self) -> usize {
        self.resolutions.iter().filter(|r| r.is_uncertain).count()
    }

    /// Summary for TUI.
    pub fn summary(&self) -> String {
        format!(
            "consensus: resolutions={} uncertain={}",
            self.resolution_count(),
            self.uncertain_count(),
        )
    }
}

impl Default for ConsensusEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_conflicting_beliefs() {
        let mut engine = ConsensusEngine::new();
        let remote = SharedBelief::new(
            "k8s-rolling-update",
            "maxSurge=25% is the safe default",
            0.78,
            30,
            vec![],
            "peer-b",
            -0.05,
        );
        let r = engine
            .resolve(
                "k8s-rolling-update",
                "maxSurge=50% improves deployment speed",
                0.65,
                8,
                &remote,
            )
            .expect("should resolve");
        assert!(r.is_resolved());
        assert_eq!(engine.resolution_count(), 1);
    }

    #[test]
    fn summary_format() {
        let engine = ConsensusEngine::new();
        let s = engine.summary();
        assert!(s.contains("consensus:"));
        assert!(s.contains("resolutions="));
    }
}
