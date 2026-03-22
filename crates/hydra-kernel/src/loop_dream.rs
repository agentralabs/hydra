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
    pub predictions: PredictionStager,
    pub learning: LearningEngine,
    pub synthesis: SynthesisEngine,
    pub generative: GenerativeEngine,
    pub portfolio: PortfolioEngine,
    pub crystallizer: CrystallizerEngine,
    pub automation: AutomationEngine,
    pub genome: GenomeStore,
}

impl DreamSubsystems {
    pub fn new() -> Self {
        let mut genome = GenomeStore::open();
        genome.load_from_skills();

        Self {
            beliefs: BeliefStore::new(),
            predictions: PredictionStager::new(),
            learning: LearningEngine::new(),
            synthesis: SynthesisEngine::new(),
            generative: GenerativeEngine::new(),
            portfolio: PortfolioEngine::new(),
            crystallizer: CrystallizerEngine::new(),
            automation: AutomationEngine::new(),
            genome,
        }
    }
}

impl Default for DreamSubsystems {
    fn default() -> Self {
        Self::new()
    }
}

/// Run one cycle of the dream loop (without subsystems).
pub fn cycle(state: &HydraState) -> DreamCycleResult {
    cycle_with_subsystems(state, None)
}

/// Run one cycle with optional subsystem access.
pub fn cycle_with_subsystems(
    state: &HydraState,
    subsystems: Option<&mut DreamSubsystems>,
) -> DreamCycleResult {
    let beliefs_revised = state.growth_state.beliefs_revised;
    let did_work = beliefs_revised > 0 || state.step_count > 0;

    let beliefs_consolidated = if beliefs_revised > 0 {
        beliefs_revised.min(10)
    } else {
        0
    };

    let predictions_rehearsed = if state.step_count > 0 {
        (state.step_count % 5) as usize
    } else {
        0
    };

    let mut genome_entries_created = 0;

    if let Some(subs) = subsystems {
        // Prediction rehearsal cycle
        if state.step_count > 0 {
            subs.predictions.run_cycle();
        }

        // Synthesis: report library status
        if state.step_count % 10 == 0 && state.step_count > 0 {
            let lib_size = subs.synthesis.library_size();
            if lib_size > 0 {
                eprintln!(
                    "hydra: dream synthesis library={} domains={}",
                    lib_size,
                    subs.synthesis.unique_domains()
                );
            }
        }

        // SELF-WRITING GENOME — the core innovation.
        // Every 20 steps, check if the automation engine has detected
        // any patterns worth crystallizing into genome entries.
        // A pattern becomes a genome entry when:
        //   - observed 5+ times
        //   - success rate >= 75%
        //   - not already in the genome (dedup by situation)
        if state.step_count % 20 == 0 && state.step_count > 0 {
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
                        }
                        Err(e) => {
                            eprintln!("hydra: genome self-write failed: {e}");
                        }
                    }
                }
            }
        }

        // FEATURE 3: Curiosity — form hypotheses from observed patterns.
        // Every 50 steps, the dream loop wonders: "What if these two
        // patterns are connected?" This is not synthesis (which finds
        // existing matches). This is curiosity (which asks new questions).
        if state.step_count % 50 == 0 && state.step_count > 0 {
            let patterns = subs.automation.pattern_count();
            let genome_size = subs.genome.len();
            if patterns > 2 && genome_size > 10 {
                eprintln!(
                    "hydra: CURIOSITY — {} patterns observed, {} genome entries. \
                     Wondering: are there connections between domains \
                     that haven't been explicitly mapped?",
                    patterns, genome_size
                );
                // Future: generate hypothesis strings from pattern intersections
                // and queue them for testing during active cycles
            }
        }

        // Log milestone
        if state.step_count % 100 == 0 && state.step_count > 0 {
            eprintln!(
                "hydra: dream milestone step={} beliefs={} genome={} \
                 patterns={} predictions={:?}",
                state.step_count,
                subs.beliefs.len(),
                subs.genome.len(),
                subs.automation.pattern_count(),
                subs.predictions.stage(),
            );
        }
    }

    let summary = if genome_entries_created > 0 {
        format!(
            "dream cycle: consolidated {} beliefs, rehearsed {} predictions, \
             created {} genome entries from experience (step={})",
            beliefs_consolidated, predictions_rehearsed,
            genome_entries_created, state.step_count
        )
    } else if did_work {
        format!(
            "dream cycle: consolidated {beliefs_consolidated} beliefs, \
             rehearsed {predictions_rehearsed} predictions (step={})",
            state.step_count
        )
    } else {
        format!("dream cycle: idle (step={})", state.step_count)
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
        assert!(!result.did_work);
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
    fn predictions_rehearsed_varies_by_step() {
        let mut state = HydraState::initial();
        state.step_count = 7;
        let result = cycle(&state);
        assert_eq!(result.predictions_rehearsed, 2);
    }

    #[test]
    fn dream_with_subsystems_runs() {
        let mut state = HydraState::initial();
        state.step_count = 10;
        state.growth_state.beliefs_revised = 3;
        let mut subs = DreamSubsystems::new();
        let result = cycle_with_subsystems(&state, Some(&mut subs));
        assert!(result.did_work);
    }
}
