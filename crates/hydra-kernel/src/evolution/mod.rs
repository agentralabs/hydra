//! META1 Self-Evolution — Hydra detects gaps and generates new skills autonomously.
//! Pipeline: DETECT → DESIGN → WRITE → VERIFY → LOAD → LEARN.
//! Generated skills are TOML-based (loadable without recompilation).
//! Self-generated entries start at confidence 0.5, must earn trust through use.

pub mod detector;
pub mod designer;
pub mod verifier;
pub mod writer;

/// A detected capability gap — domain where Hydra consistently fails.
#[derive(Debug, Clone)]
pub struct CapabilityGap {
    pub domain: String,
    pub failure_count: u64,
    pub existing_entries: usize,
    pub suggested_approach: String,
}

/// Result of one evolution cycle.
#[derive(Debug)]
pub enum EvolutionResult {
    NoGaps,
    NewCapability { name: String, domain: String, entries_added: usize },
    VerificationFailed(String),
    WriteFailed(String),
}

/// The self-evolution engine — runs in the ambient loop.
pub struct EvolutionEngine {
    detector: detector::GapDetector,
    cycle_count: u64,
}

impl EvolutionEngine {
    pub fn new() -> Self {
        Self { detector: detector::GapDetector::new(), cycle_count: 0 }
    }

    /// Run one evolution cycle. Detects gaps, designs + writes + verifies + loads skills.
    pub fn tick(&mut self, genome: &mut hydra_genome::GenomeStore) -> EvolutionResult {
        self.cycle_count += 1;

        // 1. Detect gaps
        let gaps = self.detector.detect(genome);
        if gaps.is_empty() { return EvolutionResult::NoGaps; }

        // 2. Pick highest-impact gap
        let gap = &gaps[0];
        eprintln!("hydra-evolution: cycle {} — gap detected: {} ({} failures)",
            self.cycle_count, gap.domain, gap.failure_count);

        // 3. Design skill blueprint
        let blueprint = designer::design_skill(gap, genome);

        // 4. Write skill TOML
        let path = match writer::write_skill(&blueprint) {
            Ok(p) => p,
            Err(e) => { eprintln!("hydra-evolution: write failed: {e}"); return EvolutionResult::WriteFailed(e); }
        };

        // 5. Verify
        let valid_count = match verifier::validate_skill(&path, genome) {
            Ok(n) => n,
            Err(e) => { eprintln!("hydra-evolution: verification failed: {e}"); return EvolutionResult::VerificationFailed(e); }
        };

        // 5.5 GUARDRAIL: evolution approval gate
        let gr_config = crate::guardrail::config::GuardrailConfig::load();
        if !gr_config.is_path_allowed(&path) {
            eprintln!("hydra-evolution: BLOCKED by guardrail — forbidden path: {path}");
            crate::guardrail::audit::record_quick(
                crate::guardrail::audit::AuditEventType::BoundaryViolation,
                &format!("Evolution blocked: {path}"));
            return EvolutionResult::WriteFailed(format!("Path blocked by guardrail: {path}"));
        }
        let blast = if blueprint.approaches.len() > 5 { "Visible" } else { "Contained" };
        if crate::guardrail::evolution_gate::needs_approval(blast, &gr_config) {
            let proposal = crate::guardrail::evolution_gate::EvolutionProposal {
                id: format!("evo-{}-{}", self.cycle_count, chrono::Utc::now().timestamp()),
                name: blueprint.name.clone(), domain: gap.domain.clone(),
                entries: valid_count, blast_radius: blast.into(), skill_path: path.clone(),
                proposed_at: chrono::Utc::now(),
                status: crate::guardrail::evolution_gate::ProposalStatus::Pending,
            };
            crate::guardrail::evolution_gate::queue_proposal(&proposal);
            crate::guardrail::audit::record_quick(
                crate::guardrail::audit::AuditEventType::EvolutionProposed,
                &format!("Queued: {} ({})", blueprint.name, proposal.id));
            eprintln!("hydra-evolution: QUEUED for owner approval — {}", blueprint.name);
            return EvolutionResult::VerificationFailed(
                format!("Awaiting owner approval: {}", blueprint.name));
        }

        // 6. Load into genome at confidence 0.5
        for (situation, approach) in &blueprint.approaches {
            let sig = hydra_genome::ApproachSignature::new(
                "self-evolved", vec![approach.clone()], vec!["evolution".into()]);
            let desc = format!("self-evolved:{} {}", blueprint.domain, situation);
            match genome.add_from_operation(&desc, sig, 0.5) {
                Ok(id) => eprintln!("hydra-evolution: loaded entry {id}"),
                Err(e) => eprintln!("hydra-evolution: load failed: {e}"),
            }
        }

        eprintln!("hydra-evolution: NEW CAPABILITY — {} ({} entries)", blueprint.name, valid_count);
        EvolutionResult::NewCapability {
            name: blueprint.name,
            domain: gap.domain.clone(),
            entries_added: valid_count,
        }
    }

    pub fn evolved_count(&self) -> u64 { self.cycle_count }
}

impl Default for EvolutionEngine {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_creates() {
        let e = EvolutionEngine::new();
        assert_eq!(e.evolved_count(), 0);
    }

    #[test]
    fn no_gaps_in_empty_genome() {
        let mut genome = hydra_genome::GenomeStore::new();
        let mut engine = EvolutionEngine::new();
        let result = engine.tick(&mut genome);
        assert!(matches!(result, EvolutionResult::NoGaps));
    }

    #[test]
    fn capability_gap_fields() {
        let gap = CapabilityGap {
            domain: "test".into(), failure_count: 10,
            existing_entries: 3, suggested_approach: "improve".into(),
        };
        assert_eq!(gap.failure_count, 10);
    }
}
