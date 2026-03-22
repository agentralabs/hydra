//! AutomationEngine — the behavior crystallization coordinator.
//! Observes -> Detects patterns -> Proposes -> Generates skill.

use crate::{
    constants::*,
    errors::AutomationError,
    generator::SkillGenerator,
    observation::ExecutionObservation,
    pattern::BehaviorPattern,
    proposal::{CrystallizationProposal, ProposalState},
};
use std::collections::HashMap;

/// The automation engine.
pub struct AutomationEngine {
    observations: Vec<ExecutionObservation>,
    patterns: HashMap<String, BehaviorPattern>,
    proposals: Vec<CrystallizationProposal>,
    generator: SkillGenerator,
    crystalized_count: usize,
}

impl AutomationEngine {
    pub fn new() -> Self {
        Self {
            observations: Vec::new(),
            patterns: HashMap::new(),
            proposals: Vec::new(),
            generator: SkillGenerator::new(),
            crystalized_count: 0,
        }
    }

    /// Record an execution observation.
    /// Called by the executor after every action completes.
    pub fn observe(&mut self, obs: ExecutionObservation) -> Option<String> {
        // Prune if at capacity
        if self.observations.len() >= MAX_OBSERVATIONS {
            self.observations.remove(0);
        }

        let signature = obs.signature();
        self.observations.push(obs.clone());

        // Update or create pattern
        if let Some(pattern) = self.patterns.get_mut(&signature) {
            pattern.add_observation(&obs);
        } else {
            let pattern = BehaviorPattern::new(&obs);
            self.patterns.insert(signature.clone(), pattern);
        }

        // Check if pattern now meets threshold
        let pattern = self.patterns.get(&signature)?;
        if pattern.is_crystallizable() {
            // Don't create duplicate proposals
            let already_proposed = self
                .proposals
                .iter()
                .any(|p| p.pattern_id == pattern.id && !p.state.is_terminal());
            if !already_proposed {
                return Some(self.create_proposal(&signature));
            }
        }
        None
    }

    fn create_proposal(&mut self, signature: &str) -> String {
        let pattern = self
            .patterns
            .get(signature)
            .expect("pattern must exist when creating proposal");
        let proposal = CrystallizationProposal::from_pattern(pattern);
        let message = proposal.message.clone();
        self.proposals.push(proposal);
        message
    }

    /// Principal approves a proposal — generate the skill.
    pub fn approve(&mut self, proposal_id: &str) -> Result<String, AutomationError> {
        // Find proposal
        let proposal = self
            .proposals
            .iter_mut()
            .find(|p| p.id == proposal_id)
            .ok_or_else(|| AutomationError::ProposalNotFound {
                id: proposal_id.to_string(),
            })?;

        proposal.approve();
        let pattern_id = proposal.pattern_id.clone();
        let proposal_id_owned = proposal.id.clone();

        // Find pattern
        let pattern = self
            .patterns
            .values()
            .find(|p| p.id == pattern_id)
            .ok_or_else(|| AutomationError::ProposalNotFound {
                id: pattern_id.clone(),
            })?
            .clone();

        // Generate skill
        let pkg =
            self.generator
                .generate(&pattern)
                .map_err(|e| AutomationError::GenerationFailed {
                    reason: e.to_string(),
                })?;

        let skill_name = pkg.skill_name.clone();

        // Mark crystallized
        if let Some(p) = self
            .proposals
            .iter_mut()
            .find(|p| p.id == proposal_id_owned)
        {
            p.mark_crystallized(&skill_name);
        }

        self.crystalized_count += 1;

        // In production: write TOML files to SKILL_OUTPUT_DIR
        // and call hydra-skills.hot_load(skill_name)
        // In this implementation: return the skill name

        Ok(skill_name)
    }

    /// Principal declines a proposal.
    pub fn decline(&mut self, proposal_id: &str) -> Result<(), AutomationError> {
        self.proposals
            .iter_mut()
            .find(|p| p.id == proposal_id)
            .ok_or_else(|| AutomationError::ProposalNotFound {
                id: proposal_id.to_string(),
            })?
            .decline();
        Ok(())
    }

    /// All pending proposals (what Hydra is asking the principal).
    pub fn pending_proposals(&self) -> Vec<&CrystallizationProposal> {
        self.proposals
            .iter()
            .filter(|p| p.state == ProposalState::Pending)
            .collect()
    }

    pub fn observation_count(&self) -> usize {
        self.observations.len()
    }
    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }
    pub fn proposal_count(&self) -> usize {
        self.proposals.len()
    }
    pub fn crystallized_count(&self) -> usize {
        self.crystalized_count
    }

    /// Summary for TUI.
    pub fn summary(&self) -> String {
        format!(
            "automation: observations={} patterns={} proposals={} crystallized={}",
            self.observations.len(),
            self.patterns.len(),
            self.pending_proposals().len(),
            self.crystalized_count,
        )
    }
}

impl Default for AutomationEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn obs(action: &str, success: bool) -> ExecutionObservation {
        ExecutionObservation::new(
            action,
            "test intent",
            HashMap::new(),
            "engineering",
            500,
            success,
        )
    }

    #[test]
    fn below_threshold_no_proposal() {
        let mut engine = AutomationEngine::new();
        for _ in 0..(CRYSTALLIZATION_THRESHOLD - 1) {
            let result = engine.observe(obs("deploy.staging", true));
            assert!(result.is_none());
        }
        assert_eq!(engine.pending_proposals().len(), 0);
    }

    #[test]
    fn at_threshold_proposal_created() {
        let mut engine = AutomationEngine::new();
        let mut proposal_msg = None;
        for _i in 0..CRYSTALLIZATION_THRESHOLD {
            proposal_msg = engine.observe(obs("deploy.staging", true));
        }
        assert!(proposal_msg.is_some());
        assert_eq!(engine.pending_proposals().len(), 1);
        let msg = proposal_msg.expect("should have proposal message");
        assert!(msg.contains("Shall I"));
    }

    #[test]
    fn no_duplicate_proposals() {
        let mut engine = AutomationEngine::new();
        for _ in 0..(CRYSTALLIZATION_THRESHOLD + 5) {
            engine.observe(obs("deploy.staging", true));
        }
        // Should not create multiple proposals for same pattern
        assert_eq!(engine.pending_proposals().len(), 1);
    }

    #[test]
    fn approve_generates_skill() {
        let mut engine = AutomationEngine::new();
        for _ in 0..CRYSTALLIZATION_THRESHOLD {
            engine.observe(obs("deploy.staging", true));
        }
        let proposal_id = engine.pending_proposals()[0].id.clone();
        let skill_name = engine
            .approve(&proposal_id)
            .expect("approve should succeed");
        assert!(skill_name.starts_with("auto-"));
        assert_eq!(engine.crystallized_count(), 1);
        assert_eq!(engine.pending_proposals().len(), 0);
    }

    #[test]
    fn decline_removes_from_pending() {
        let mut engine = AutomationEngine::new();
        for _ in 0..CRYSTALLIZATION_THRESHOLD {
            engine.observe(obs("deploy.staging", true));
        }
        let proposal_id = engine.pending_proposals()[0].id.clone();
        engine
            .decline(&proposal_id)
            .expect("decline should succeed");
        assert_eq!(engine.pending_proposals().len(), 0);
        assert_eq!(engine.crystallized_count(), 0);
    }

    #[test]
    fn summary_format() {
        let engine = AutomationEngine::new();
        let s = engine.summary();
        assert!(s.contains("automation:"));
        assert!(s.contains("observations="));
        assert!(s.contains("crystallized="));
    }

    #[test]
    fn not_found_error() {
        let mut engine = AutomationEngine::new();
        let result = engine.approve("nonexistent");
        assert!(result.is_err());
    }
}
