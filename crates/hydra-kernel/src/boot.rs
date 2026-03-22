//! Boot sequence — brings the kernel from cold to alive.
//!
//! The boot sequence runs 7 phases in order.
//! Each phase must succeed before the next begins.
//! If any phase fails, the kernel does not start.

use crate::{
    constants::BOOT_TIMEOUT_SECONDS,
    errors::KernelError,
    state::{BootPhase, HydraState, KernelPhase},
};
use serde::{Deserialize, Serialize};

/// The result of a successful boot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootResult {
    /// The initial kernel state after boot.
    pub state: HydraState,
    /// The final kernel phase (should be Alive).
    pub phase: KernelPhase,
    /// How long boot took in milliseconds.
    pub boot_duration_ms: u64,
    /// Which phases completed successfully.
    pub phases_completed: Vec<String>,
}

/// Run the full boot sequence.
pub async fn run_boot_sequence() -> Result<BootResult, KernelError> {
    let start = std::time::Instant::now();
    let mut phases_completed = Vec::new();
    let timeout = std::time::Duration::from_secs(BOOT_TIMEOUT_SECONDS);

    type PhaseFn = fn() -> Result<(), String>;
    let phases: Vec<(BootPhase, PhaseFn)> = vec![
        (BootPhase::ConstitutionVerify, verify_constitution),
        (BootPhase::AnimusInit, init_animus),
        (BootPhase::MemoryResume, resume_memory),
        (BootPhase::BeliefRehydrate, rehydrate_beliefs),
        (BootPhase::FleetReconnect, reconnect_fleet),
        (BootPhase::PredictionStage, stage_predictions),
        (BootPhase::TuiReady, tui_ready),
    ];

    // Self-repair before boot phases — heal any damage from last run
    let repairs = crate::self_repair::self_repair();
    if !repairs.is_empty() {
        let repaired = repairs.iter().filter(|(_, ok)| *ok).count();
        let unresolved = repairs.len() - repaired;
        eprintln!(
            "hydra: boot self-repair: {} fixed, {} unresolved",
            repaired, unresolved
        );
    }

    for (phase, f) in phases {
        run_phase(&phase, f).await?;
        phases_completed.push(phase.to_string());
    }

    let elapsed = start.elapsed();
    if elapsed > timeout {
        return Err(KernelError::BootFailed {
            phase: "timeout-check".to_string(),
            reason: format!(
                "Boot took {}ms, exceeds {}s timeout",
                elapsed.as_millis(),
                BOOT_TIMEOUT_SECONDS
            ),
        });
    }

    let state = HydraState::initial();
    eprintln!(
        "hydra: boot complete in {}ms ({} phases)",
        elapsed.as_millis(),
        phases_completed.len()
    );

    Ok(BootResult {
        state,
        phase: KernelPhase::Alive,
        boot_duration_ms: elapsed.as_millis() as u64,
        phases_completed,
    })
}

async fn run_phase<F>(phase: &BootPhase, f: F) -> Result<(), KernelError>
where
    F: FnOnce() -> Result<(), String>,
{
    f().map_err(|reason| KernelError::BootFailed {
        phase: phase.to_string(),
        reason,
    })
}

fn verify_constitution() -> Result<(), String> {
    let checker = hydra_constitution::ConstitutionChecker::new();
    if checker.law_count() != 7 {
        return Err(format!("Expected 7 laws, found {}", checker.law_count()));
    }
    Ok(())
}

fn init_animus() -> Result<(), String> {
    let signal = hydra_animus::Signal::constitutional_identity();
    hydra_animus::validate_for_bus(&signal)
        .map_err(|e| format!("Animus validation failed: {e}"))
}

fn resume_memory() -> Result<(), String> {
    let bridge = hydra_memory::HydraMemoryBridge::new();
    eprintln!("hydra: boot memory session={}", bridge.session_id());
    Ok(())
}

fn rehydrate_beliefs() -> Result<(), String> {
    let _store = hydra_belief::BeliefStore::new();
    eprintln!("hydra: boot beliefs rehydrated");
    Ok(())
}

fn reconnect_fleet() -> Result<(), String> {
    let _registry = hydra_fleet::FleetRegistry::new();
    eprintln!("hydra: boot fleet registry ready");
    Ok(())
}

fn stage_predictions() -> Result<(), String> {
    let _stager = hydra_prediction::PredictionStager::new();
    eprintln!("hydra: boot predictions staged");
    Ok(())
}

fn tui_ready() -> Result<(), String> {
    // Detect environment at boot
    let detector = hydra_environment::EnvironmentDetector::new();
    match detector.detect_current() {
        Ok(profile) => {
            eprintln!("hydra: boot environment: {:?}", profile.class);
        }
        Err(e) => {
            eprintln!("hydra: boot environment detection failed: {e}");
        }
    }

    // Skill registry ready
    let _registry = hydra_skills::SkillRegistry::new();
    eprintln!("hydra: boot skills registry ready");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn boot_sequence_succeeds() {
        let result = run_boot_sequence().await;
        assert!(result.is_ok());
        let boot = result.expect("boot should succeed");
        assert_eq!(boot.phases_completed.len(), 7);
        assert_eq!(boot.phase, KernelPhase::Alive);
    }

    #[tokio::test]
    async fn boot_produces_initial_state() {
        let boot = run_boot_sequence().await.expect("boot should succeed");
        assert!(boot.state.is_stable());
        assert_eq!(boot.state.step_count, 0);
    }

    #[test]
    fn verify_constitution_passes() {
        assert!(verify_constitution().is_ok());
    }

    #[test]
    fn init_animus_passes() {
        assert!(init_animus().is_ok());
    }
}
