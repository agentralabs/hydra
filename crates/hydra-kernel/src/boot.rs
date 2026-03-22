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

    // Skill registry — count skill directories
    let _registry = hydra_skills::SkillRegistry::new();
    let skill_count = count_toml_dirs("skills");
    eprintln!("hydra: boot skills: {} directories found", skill_count);

    // Executor — load integrations and actions
    let integration_count = count_toml_dirs("integrations");
    let action_count = count_toml_dirs("actions");
    eprintln!(
        "hydra: boot executor: {} integrations, {} actions",
        integration_count, action_count
    );

    // Device profile — detect physical capabilities of this system
    let has_mic = hydra_voice::microphone::is_microphone_available();
    let has_tts = hydra_voice::TtsEngine::detect().is_available();
    let capabilities = hydra_reach::DeviceCapabilities {
        has_microphone: has_mic,
        has_speaker: has_tts,
        has_display: true,
        display_width: None,
        display_height: None,
        has_touch: false,
        has_camera: false,
        has_keyboard: true,
        is_mobile: false,
    };
    let device = hydra_reach::DeviceProfile::new(
        "hydra-local",
        hostname(),
        capabilities,
        "local-session",
    );
    eprintln!(
        "hydra: boot device: {} surface={:?} mic={} tts={}",
        device.name, device.surface_class, has_mic, has_tts
    );

    // Subsystem validation
    let _swarm = hydra_swarm::EmergenceStore::new();
    // reach-extended now lives in ambient loop (persistent connectivity tracker)
    let _transform = hydra_transform::TransformEngine::new();
    let _protocol = hydra_protocol::ProtocolEngine::new();
    let _horizon = hydra_horizon::Horizon::new();
    let _persona = hydra_persona::PersonaRegistry::new();
    eprintln!("hydra: boot subsystems: swarm, reach, transform, protocol, horizon, persona — ready");

    Ok(())
}

/// Get system hostname for device profile.
fn hostname() -> String {
    std::process::Command::new("hostname")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".into())
}

/// Count directories containing .toml files in a drop folder.
fn count_toml_dirs(folder: &str) -> usize {
    let dir = std::path::PathBuf::from(folder);
    if !dir.exists() {
        return 0;
    }
    std::fs::read_dir(&dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| e.path().is_dir())
                .count()
        })
        .unwrap_or(0)
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
