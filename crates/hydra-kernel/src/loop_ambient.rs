//! The AMBIENT loop — background maintenance and monitoring.
//!
//! Runs on every tick (100ms by default).
//! Checks invariants, integrates the equation, and updates state.
//! Wired: hydra-metabolism (Lyapunov monitoring), hydra-continuity (morphic),
//!        hydra-signals (fabric health).

use hydra_continuity::ContinuityEngine;
use hydra_metabolism::MetabolismMonitor;
use hydra_signals::SignalFabric;

use crate::{equation::integrate_euler, invariants, state::HydraState};
use serde::{Deserialize, Serialize};

/// The result of one ambient tick.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmbientTickResult {
    /// The updated state after this tick.
    pub state: HydraState,
    /// Whether all invariants passed.
    pub invariants_ok: bool,
    /// Metabolism intervention level after this tick.
    pub intervention_level: String,
    /// Summary message.
    pub summary: String,
}

/// Ambient subsystems that persist across ticks.
pub struct AmbientSubsystems {
    pub metabolism: MetabolismMonitor,
    pub continuity: ContinuityEngine,
    pub fabric: SignalFabric,
    /// Connectivity health tracker — monitors reachability of external services.
    pub reach: hydra_reach_extended::ReachEngine,
    /// Inline checkpoint tracking (step count at last checkpoint).
    pub last_checkpoint_step: u64,
    /// Self-preservation: periodic data integrity verification (O23).
    pub integrity: crate::integrity::IntegrityMonitor,
    /// Self-evolution: detect gaps and generate new skills (Session 25).
    pub evolution: crate::evolution::EvolutionEngine,
    /// Universal Drop Gateway: single entry point for all external items.
    pub drop_gateway: crate::drop::DropGateway,
    /// Internal step counter — incremented each ambient tick.
    pub ambient_step: u64,
}

impl AmbientSubsystems {
    pub fn new() -> Self {
        Self {
            metabolism: MetabolismMonitor::new(),
            continuity: ContinuityEngine::new(),
            fabric: SignalFabric::new(),
            reach: hydra_reach_extended::ReachEngine::new(),
            last_checkpoint_step: 0,
            integrity: crate::integrity::IntegrityMonitor::new(),
            evolution: crate::evolution::EvolutionEngine::new(),
            drop_gateway: crate::drop::DropGateway::new(),
            ambient_step: 0,
        }
    }
}

impl Default for AmbientSubsystems {
    fn default() -> Self {
        Self::new()
    }
}

/// Run one tick of the ambient loop.
pub fn tick(state: &HydraState, dt: f64) -> AmbientTickResult {
    tick_with_subsystems(state, dt, None)
}

/// Run one tick with optional subsystem access.
pub fn tick_with_subsystems(
    state: &HydraState,
    dt: f64,
    subsystems: Option<&mut AmbientSubsystems>,
) -> AmbientTickResult {
    let next_state = integrate_euler(state, dt);
    let invariant_results = invariants::check_all(&next_state);
    // Use a static counter since HydraState::initial() always has step_count=0
    static AMBIENT_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let step = AMBIENT_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    let intervention_level = if let Some(subs) = subsystems {
        // Metabolism: Lyapunov monitoring
        let level = match subs
            .metabolism
            .tick(next_state.lyapunov_value, next_state.growth_state.growth_rate)
        {
            Ok(level) => format!("{level:?}"),
            Err(e) => {
                eprintln!("hydra: ambient metabolism tick: {e}");
                "error".to_string()
            }
        };

        // Continuity: prove lineage and write checkpoints periodically
        if step % 100 == 0 && step > 0 {
            // Prove primary lineage is intact
            if let Err(e) = subs.continuity.prove_lineage("primary") {
                eprintln!("hydra: continuity lineage proof: {e}");
            }
        }
        if step % 1000 == 0 && step > 0 {
            eprintln!(
                "hydra: continuity lineages={} checkpoints={}",
                subs.continuity.lineage_count(),
                subs.continuity.total_checkpoint_count()
            );
        }

        // Resurrection: record checkpoint at milestones
        if step - subs.last_checkpoint_step >= 100 {
            let snapshot = hydra_resurrection::KernelStateSnapshot::new(
                next_state.lyapunov_value,
                step,
                vec![0.0],      // manifold coordinates
                0.9,            // average trust
                0.1,            // queue utilization
                next_state.growth_state.growth_rate,
            );
            match hydra_resurrection::Checkpoint::full(step, snapshot) {
                Ok(cp) => {
                    if let Err(e) = cp.verify_integrity() {
                        eprintln!("hydra: checkpoint integrity: {e}");
                    }
                }
                Err(e) => eprintln!("hydra: checkpoint create: {e}"),
            }
        }

        // Reach: track connectivity health every 500 ticks (~50 seconds)
        if step % 500 == 0 && step > 0 {
            let provider = std::env::var("HYDRA_LLM_PROVIDER").unwrap_or_else(|_| "anthropic".into());
            let endpoint = match provider.as_str() {
                "openai" => "https://api.openai.com",
                "ollama" => "http://localhost:11434",
                _ => "https://api.anthropic.com",
            };
            match subs.reach.reach(endpoint) {
                Ok(result) => {
                    eprintln!(
                        "hydra: reach health: {} active={} total={}",
                        endpoint,
                        subs.reach.active_session_count(),
                        subs.reach.total_session_count(),
                    );
                    let _ = result;
                }
                Err(e) => {
                    eprintln!("hydra: reach health check failed: {e}");
                }
            }
        }

        // Self-preservation (O23): integrity check every 30 minutes
        if subs.integrity.should_check() {
            let report = subs.integrity.check();
            if !report.is_healthy() {
                eprintln!("hydra: integrity issues detected: {}", report.summary());
            }
        }

        // Self-preservation (O23): cloud backup every 20000 ticks (~8 hours)
        if step % 20000 == 0 && step > 0 {
            let config = crate::backup_cloud::CloudBackupConfig::default();
            if config.enabled {
                let backup_dir = dirs::home_dir().unwrap_or_default().join(".hydra/backups");
                match crate::backup_cloud::upload_backup(&backup_dir, &config) {
                    Ok(r) => eprintln!("hydra: cloud backup: {} files → {}", r.files_uploaded, r.destination),
                    Err(e) => eprintln!("hydra: cloud backup skipped: {e}"),
                }
            }
        }

        // Self-evolution (Session 25): detect gaps and generate skills
        if step % 10000 == 0 && step > 0 {
            let mut genome = hydra_genome::GenomeStore::open();
            let result = subs.evolution.tick(&mut genome);
            if let crate::evolution::EvolutionResult::NewCapability { ref name, .. } = result {
                eprintln!("hydra: SELF-EVOLUTION — new capability: {name}");
            }
        }

        // Universal Drop Gateway: process files from ~/.hydra/drop/ every 50 ticks (~5s)
        if step % 50 == 0 {
            let records = subs.drop_gateway.tick();
            for r in &records {
                eprintln!("hydra-drop: {} → {:?}", r.filename, r.outcome);
            }
        }

        // Signal fabric: dispatch queued signals
        let dispatch = subs.fabric.dispatch();
        if dispatch.dispatched > 0 || dispatch.unrouted > 0 {
            eprintln!(
                "hydra: ambient signals dispatched={} unrouted={}",
                dispatch.dispatched, dispatch.unrouted
            );
        }

        // Track checkpoint milestones
        if step - subs.last_checkpoint_step >= 100 {
            subs.last_checkpoint_step = step;
            eprintln!(
                "hydra: ambient checkpoint milestone step={}",
                step
            );
        }

        level
    } else {
        "none".to_string()
    };

    let summary = if invariant_results.all_passed {
        format!(
            "ambient tick {} ok, V(Psi)={:.4}, intervention={}",
            step, next_state.lyapunov_value, intervention_level
        )
    } else {
        let failure = invariant_results
            .first_failure()
            .map(|f| format!("{}: {}", f.name, f.message))
            .unwrap_or_else(|| "unknown failure".to_string());
        format!(
            "ambient tick {} INVARIANT FAILED: {}",
            step, failure
        )
    };

    AmbientTickResult {
        state: next_state,
        invariants_ok: invariant_results.all_passed,
        intervention_level,
        summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_advances_state() {
        let state = HydraState::initial();
        let result = tick(&state, 0.1);
        assert_eq!(result.state.step_count, 1);
    }

    #[test]
    fn tick_checks_invariants() {
        let state = HydraState::initial();
        let result = tick(&state, 0.1);
        assert!(result.invariants_ok);
    }

    #[test]
    fn tick_detects_invariant_failure() {
        let mut state = HydraState::initial();
        state.lyapunov_value = -10.0;
        let result = tick(&state, 0.1);
        assert_eq!(result.state.step_count, 1);
    }

    #[test]
    fn tick_summary_contains_step() {
        let state = HydraState::initial();
        let result = tick(&state, 0.1);
        assert!(result.summary.contains("tick 1"));
    }

    #[test]
    fn multiple_ticks_accumulate() {
        let mut state = HydraState::initial();
        for _ in 0..10 {
            let result = tick(&state, 0.1);
            state = result.state;
        }
        assert_eq!(state.step_count, 10);
    }

    #[test]
    fn tick_with_subsystems_runs() {
        let state = HydraState::initial();
        let mut subs = AmbientSubsystems::new();
        let result = tick_with_subsystems(&state, 0.1, Some(&mut subs));
        assert_eq!(result.state.step_count, 1);
        assert_ne!(result.intervention_level, "none");
    }

    #[test]
    fn ambient_subsystems_default() {
        let subs = AmbientSubsystems::default();
        assert_eq!(subs.last_checkpoint_step, 0);
    }
}
