use std::time::Duration;

use hydra_core::types::CognitivePhase;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CheckpointLevel {
    None,
    Minimal,
    Full,
    Atomic,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TimeoutBehavior {
    UseDefaults,
    Escalate,
    AskUser,
    DeferToBackground,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorBehavior {
    SkipAndContinue,
    RetryThenFail,
    AskUser,
    RollbackAndFail,
    LogAndContinue,
}

#[derive(Debug, Clone)]
pub struct PhaseConfig {
    pub timeout: Duration,
    pub user_interruptible: bool,
    pub system_interruptible: bool,
    pub checkpoint_level: CheckpointLevel,
    pub timeout_behavior: TimeoutBehavior,
    pub error_behavior: ErrorBehavior,
}

pub fn default_phase_config(phase: CognitivePhase) -> PhaseConfig {
    match phase {
        CognitivePhase::Perceive => PhaseConfig {
            timeout: Duration::from_secs(10),
            user_interruptible: true,
            system_interruptible: true,
            checkpoint_level: CheckpointLevel::Minimal,
            timeout_behavior: TimeoutBehavior::UseDefaults,
            error_behavior: ErrorBehavior::SkipAndContinue,
        },
        CognitivePhase::Think => PhaseConfig {
            timeout: Duration::from_secs(60),
            user_interruptible: true,
            system_interruptible: false,
            checkpoint_level: CheckpointLevel::Full,
            timeout_behavior: TimeoutBehavior::Escalate,
            error_behavior: ErrorBehavior::RetryThenFail,
        },
        CognitivePhase::Decide => PhaseConfig {
            timeout: Duration::from_secs(30),
            user_interruptible: true,
            system_interruptible: false,
            checkpoint_level: CheckpointLevel::Full,
            timeout_behavior: TimeoutBehavior::AskUser,
            error_behavior: ErrorBehavior::AskUser,
        },
        CognitivePhase::Act => PhaseConfig {
            timeout: Duration::from_secs(300),
            user_interruptible: true,
            system_interruptible: false,
            checkpoint_level: CheckpointLevel::Atomic,
            timeout_behavior: TimeoutBehavior::Escalate,
            error_behavior: ErrorBehavior::RollbackAndFail,
        },
        CognitivePhase::Learn => PhaseConfig {
            timeout: Duration::from_secs(10),
            user_interruptible: false,
            system_interruptible: true,
            checkpoint_level: CheckpointLevel::Minimal,
            timeout_behavior: TimeoutBehavior::DeferToBackground,
            error_behavior: ErrorBehavior::LogAndContinue,
        },
    }
}

#[derive(Debug, Clone)]
pub struct KernelConfig {
    pub phase_configs: std::collections::HashMap<CognitivePhase, PhaseConfig>,
    pub max_recursion_depth: usize,
    pub max_think_iterations: usize,
    pub token_budget: u64,
}

impl Default for KernelConfig {
    fn default() -> Self {
        let mut phase_configs = std::collections::HashMap::new();
        for phase in [
            CognitivePhase::Perceive,
            CognitivePhase::Think,
            CognitivePhase::Decide,
            CognitivePhase::Act,
            CognitivePhase::Learn,
        ] {
            phase_configs.insert(phase, default_phase_config(phase));
        }
        Self {
            phase_configs,
            max_recursion_depth: 5,
            max_think_iterations: 10,
            token_budget: 100_000,
        }
    }
}

impl KernelConfig {
    pub fn phase_timeout(&self, phase: CognitivePhase) -> Duration {
        self.phase_configs
            .get(&phase)
            .map(|c| c.timeout)
            .unwrap_or(Duration::from_secs(30))
    }

    pub fn set_phase_timeout(&mut self, phase: CognitivePhase, timeout: Duration) {
        if let Some(config) = self.phase_configs.get_mut(&phase) {
            config.timeout = timeout;
        }
    }
}
