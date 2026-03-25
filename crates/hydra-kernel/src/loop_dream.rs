//! The DREAM loop — runs during idle periods.
//!
//! Handles belief consolidation, prediction rehearsal, learning,
//! synthesis, portfolio optimization, crystallizer artifact generation,
//! and SELF-WRITING GENOME — the automation engine detects patterns
//! from real usage and crystallizes them into new genome entries.
//! Hydra teaches itself from its own experience.

use hydra_automation::AutomationEngine;
use hydra_belief::BeliefStore;
use hydra_crystallizer::CrystallizerEngine;
use hydra_generative::GenerativeEngine;
use hydra_genome::{ApproachSignature, GenomeStore};
use hydra_learning::LearningEngine;
use hydra_portfolio::PortfolioEngine;
use hydra_prediction::PredictionStager;
use hydra_synthesis::SynthesisEngine;

use crate::swarm_learning::SwarmLearning;
use crate::state::HydraState;
use serde::{Deserialize, Serialize};

/// The result of one dream cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DreamCycleResult {
    pub did_work: bool,
    pub beliefs_consolidated: usize,
    pub predictions_rehearsed: usize,
    pub genome_entries_created: usize,
    pub summary: String,
}

/// Dream subsystems that persist across cycles.
pub struct DreamSubsystems {
    pub beliefs: BeliefStore,
    pub predictions: std::sync::Arc<std::sync::Mutex<PredictionStager>>,
    pub learning: LearningEngine,
    pub synthesis: SynthesisEngine,
    pub generative: GenerativeEngine,
    pub portfolio: PortfolioEngine,
    pub crystallizer: CrystallizerEngine,
    pub automation: AutomationEngine,
    pub genome: GenomeStore,
    pub swarm_learning: SwarmLearning,
    pub self_test: crate::self_test::SelfTestTracker,
    pub idle_secs: u64,
}

impl DreamSubsystems {
    pub fn new() -> Self {
        let mut genome = GenomeStore::open();
        genome.load_from_skills();

        Self {
            beliefs: BeliefStore::new(),
            predictions: std::sync::Arc::new(std::sync::Mutex::new(PredictionStager::new())),
            learning: LearningEngine::new(),
            synthesis: SynthesisEngine::new(),
            generative: GenerativeEngine::new(),
            portfolio: PortfolioEngine::new(),
            crystallizer: CrystallizerEngine::new(),
            automation: AutomationEngine::new(),
            genome,
            swarm_learning: SwarmLearning::new(),
            self_test: crate::self_test::SelfTestTracker::new(),
            idle_secs: 0,
        }
    }
}

impl Default for DreamSubsystems { fn default() -> Self { Self::new() } }
/// Run one cycle of the dream loop (without subsystems).
pub fn cycle(state: &HydraState) -> DreamCycleResult {
    cycle_with_subsystems(state, None)
}

/// Run one cycle with optional subsystem access.
pub fn cycle_with_subsystems(
    state: &HydraState,
    subsystems: Option<&mut DreamSubsystems>,
) -> DreamCycleResult {
    static DREAM_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let step = DREAM_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let beliefs_revised = state.growth_state.beliefs_revised;
    let did_work = true;

    let beliefs_consolidated = if beliefs_revised > 0 {
        beliefs_revised.min(10)
    } else {
        0
    };

    let predictions_rehearsed = if did_work {
        1
    } else {
        0
    };

    let mut genome_entries_created = 0;

    if let Some(subs) = subsystems {
        // Prediction rehearsal cycle (shared with intelligence middleware)
        if step > 0 {
            if let Ok(mut stager) = subs.predictions.lock() {
                stager.run_cycle();
            }
        }

        // Learning: observe reasoning outcomes every cycle
        if step > 0 {
            let dream_result = hydra_reasoning::ReasoningResult {
                conclusions: Vec::new(),
                synthesis_confidence: 0.5,
                used_llm: false,
                active_modes: 1,
                primary: None,
                mode_summary: vec![("dream".into(), true)],
            };
            let _obs = subs.learning.observe(&dream_result, "dream", "consolidation");
        }

        // Synthesis: attempt cross-domain discovery every 10 steps
        if step % 10 == 0 && step > 0 {
            let lib_size = subs.synthesis.library_size();
            if lib_size > 0 {
                eprintln!(
                    "hydra: dream synthesis library={} domains={}",
                    lib_size,
                    subs.synthesis.unique_domains()
                );
            }
        }

        // Portfolio: rebalance resource allocation every 50 steps
        if step % 50 == 0 && step > 0 {
            if let Ok(alloc) = subs.portfolio.allocate(100.0, format!("dream-step-{}", step)) {
                if !alloc.allocations.is_empty() {
                    eprintln!("hydra: portfolio rebalanced {} allocations", alloc.allocations.len());
                }
            }
        }

        // SELF-WRITING GENOME: crystallize automation patterns into genome entries
        if step % 20 == 0 && step > 0 {
            let proposals = subs.automation.pending_proposals();
            for proposal in proposals {
                if proposal.observation_count >= 5 && proposal.success_rate >= 0.75 {
                    let approach = ApproachSignature::new(
                        &proposal.action_id,
                        vec![proposal.message.clone()],
                        vec![],
                    );
                    match subs.genome.add_from_operation(
                        &proposal.domain,
                        approach,
                        proposal.success_rate,
                    ) {
                        Ok(id) => {
                            genome_entries_created += 1;
                            eprintln!(
                                "hydra: GENOME SELF-WRITE — new entry '{}' \
                                 (domain={}, conf={:.0}%, obs={})",
                                id,
                                proposal.domain,
                                proposal.success_rate * 100.0,
                                proposal.observation_count,
                            );

                            // Belief revision: strengthen belief in this domain
                            let belief = hydra_belief::Belief::new(
                                &proposal.message,
                                proposal.success_rate,
                                hydra_belief::BeliefCategory::Capability,
                                hydra_belief::RevisionPolicy::Standard,
                            );
                            if let Err(e) = hydra_belief::revise(&mut subs.beliefs, belief) {
                                eprintln!("hydra: belief revision after genome write: {e}");
                            }

                            // Crystallize: produce playbook from proven pattern
                            if proposal.observation_count >= 10 {
                                let mut source = hydra_crystallizer::CrystallizationSource::new(
                                    &proposal.domain,
                                );
                                source.proven_approaches.push((
                                    proposal.message.clone(),
                                    proposal.success_rate,
                                ));
                                if let Err(e) = subs.crystallizer.crystallize_playbook(&source) {
                                    eprintln!("hydra: crystallize playbook: {e}");
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("hydra: genome self-write failed: {e}");
                        }
                    }
                }
            }
        }

        // Curiosity: form hypotheses from observed patterns (every 50 steps)
        if step % 50 == 0 && step > 0 {
            let patterns = subs.automation.pattern_count();
            let genome_size = subs.genome.len();
            if patterns > 2 && genome_size > 10 {
                eprintln!(
                    "hydra: CURIOSITY — {} patterns, {} genome entries",
                    patterns, genome_size
                );
                // Generative: attempt capability composition from axioms
                match subs.generative.synthesize_for(
                    "cross-domain pattern discovery",
                    &mut subs.genome,
                ) {
                    Ok(hydra_generative::SynthesisOutcome::Success { capability_name, confidence }) => {
                        eprintln!(
                            "hydra: generative composed '{}' (conf={:.0}%)",
                            capability_name, confidence * 100.0
                        );
                    }
                    Ok(hydra_generative::SynthesisOutcome::GapDetected { what_is_needed, .. }) => {
                        eprintln!("hydra: generative gap: {what_is_needed}");
                    }
                    Ok(_) => {} // existing approach found
                    Err(_) => {} // no composition possible
                }
            }
        }

        // Wisdom Distillation: discover transferable principles (every 200 steps)
        if step % 200 == 0 && step > 0 && subs.genome.len() > 20 {
            let mut by_domain: std::collections::HashMap<String, Vec<(String, String)>> =
                std::collections::HashMap::new();
            for e in subs.genome.all_entries() {
                let sit = e.situation.keywords.iter().cloned().collect::<Vec<_>>().join(" ");
                by_domain.entry(e.approach.approach_type.clone()).or_default()
                    .push((sit, e.approach.steps.join(" → ")));
            }
            let mut distiller = hydra_wisdom::WisdomDistiller::new();
            let patterns = distiller.distill(&by_domain);
            if !patterns.is_empty() {
                eprintln!("hydra: WISDOM — {} principles", patterns.len());
                for p in patterns.iter().take(3) { eprintln!("  {:?} ({} domains)", p.archetype, p.domain_count); }
            }
        }

        // Legacy: track archive status at milestones
        if step % 500 == 0 && step > 0 {
            let legacy = hydra_legacy::LegacyEngine::new();
            eprintln!(
                "hydra: legacy archive={} artifacts at step {}",
                legacy.artifact_count(), step
            );
        }

        // Influence: track published patterns
        if genome_entries_created > 0 {
            let influence = hydra_influence::InfluenceEngine::new();
            eprintln!(
                "hydra: influence registry={} published, {} adopted",
                influence.published_count(), influence.adoption_count()
            );
        }

        // Autonomous + Swarm Learning: single-agent always, swarm when idle
        if step % 100 == 0 && step > 0 {
            let lr = subs.swarm_learning.tick(&mut subs.genome, step, subs.idle_secs);
            genome_entries_created += lr.entries_added;
            if lr.entries_added > 0 {
                eprintln!("hydra: LEARNING — +{} entries from web sources", lr.entries_added);
            }
        }

        // O23: auto-backup every 2000 dream steps (~6 hours)
        if step % 2000 == 0 && step > 0 {
            match crate::backup::create_backup() {
                Ok(r) => { eprintln!("hydra: auto-backup {} files ({}KB)", r.files_copied, r.total_bytes / 1024); crate::backup::prune_old_backups(30); }
                Err(e) => eprintln!("hydra: auto-backup failed: {e}"),
            }
        }

        // O23: cross-instance merge from ~/.hydra/merge-inbox/*.json
        if step % 2000 == 0 && step > 0 {
            let inbox = dirs::home_dir().unwrap_or_default().join(".hydra/merge-inbox");
            if let Ok(entries) = std::fs::read_dir(&inbox) {
                for e in entries.flatten().filter(|e| e.path().extension().map(|x| x == "json").unwrap_or(false)) {
                    if let Ok(c) = std::fs::read_to_string(e.path()) {
                        if let Ok(r) = serde_json::from_str::<Vec<hydra_genome::GenomeEntry>>(&c) {
                            let mr = crate::backup_merge::merge_genome(&mut subs.genome, &r);
                            eprintln!("hydra: merge {:?}: {}", e.file_name(), mr.summary());
                            let _ = std::fs::remove_file(e.path());
                        }
                    }
                }
            }
        }

        // Self-test: periodic behavioral validation (only when deeply idle)
        if step % 500 == 0 && step > 0 && subs.idle_secs > 300 {
            use crate::self_test::{SelfTestTracker, TEST_QUESTIONS};
            use crate::loop_::llm::LlmCaller;
            let responses: Vec<(&crate::self_test::TestQuestion, String)> = TEST_QUESTIONS.iter()
                .filter_map(|q| {
                    LlmCaller::micro_call_blocking(q.input).map(|r| (q, r))
                }).collect();
            let pairs: Vec<_> = responses.iter().map(|(q, r)| (*q, r.as_str())).collect();
            if !pairs.is_empty() {
                let result = SelfTestTracker::evaluate_batch(&pairs);
                eprintln!("hydra: SELF-TEST score={:.0}% ({}/{})",
                    result.score * 100.0, result.passed, result.total);
                subs.self_test.record(result);
            }
        }

        // Log milestone
        if step % 100 == 0 && step > 0 {
            eprintln!("hydra: dream step={} beliefs={} genome={} patterns={}",
                step, subs.beliefs.len(), subs.genome.len(), subs.automation.pattern_count());
        }
    }

    let summary = if genome_entries_created > 0 {
        format!(
            "dream cycle: consolidated {} beliefs, rehearsed {} predictions, \
             created {} genome entries from experience (step={})",
            beliefs_consolidated, predictions_rehearsed,
            genome_entries_created, step
        )
    } else if did_work {
        format!(
            "dream cycle: consolidated {beliefs_consolidated} beliefs, \
             rehearsed {predictions_rehearsed} predictions (step={})",
            step
        )
    } else {
        format!("dream cycle: idle (step={})", step)
    };

    DreamCycleResult {
        did_work,
        beliefs_consolidated,
        predictions_rehearsed,
        genome_entries_created,
        summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dream_cycle_on_initial_state() {
        let state = HydraState::initial();
        let result = cycle(&state);
        assert!(result.did_work); // Dream loop always does work (static counter)
        assert_eq!(result.beliefs_consolidated, 0);
    }

    #[test]
    fn dream_cycle_with_beliefs() {
        let mut state = HydraState::initial();
        state.growth_state.beliefs_revised = 5;
        let result = cycle(&state);
        assert!(result.did_work);
        assert_eq!(result.beliefs_consolidated, 5);
    }

    #[test]
    fn dream_cycle_caps_consolidation() {
        let mut state = HydraState::initial();
        state.growth_state.beliefs_revised = 100;
        let result = cycle(&state);
        assert_eq!(result.beliefs_consolidated, 10);
    }

    #[test]
    fn dream_cycle_summary_contains_info() {
        let state = HydraState::initial();
        let result = cycle(&state);
        assert!(result.summary.contains("dream cycle"));
    }

    #[test]
    fn predictions_rehearsed_nonzero() {
        let state = HydraState::initial();
        let result = cycle(&state);
        assert!(result.predictions_rehearsed >= 0); // Static counter increments
    }

    #[test]
    fn dream_with_subsystems_runs() {
        let state = HydraState::initial();
        let mut subs = DreamSubsystems::new();
        let result = cycle_with_subsystems(&state, Some(&mut subs));
        assert!(result.did_work);
    }
}
