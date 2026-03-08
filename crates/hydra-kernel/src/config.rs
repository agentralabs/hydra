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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_kernel_config_has_all_phases() {
        let config = KernelConfig::default();
        assert!(config.phase_configs.contains_key(&CognitivePhase::Perceive));
        assert!(config.phase_configs.contains_key(&CognitivePhase::Think));
        assert!(config.phase_configs.contains_key(&CognitivePhase::Decide));
        assert!(config.phase_configs.contains_key(&CognitivePhase::Act));
        assert!(config.phase_configs.contains_key(&CognitivePhase::Learn));
        assert_eq!(config.phase_configs.len(), 5);
    }

    #[test]
    fn test_default_recursion_depth() {
        let config = KernelConfig::default();
        assert_eq!(config.max_recursion_depth, 5);
    }

    #[test]
    fn test_default_token_budget() {
        let config = KernelConfig::default();
        assert_eq!(config.token_budget, 100_000);
    }

    #[test]
    fn test_perceive_timeout_is_10s() {
        let config = KernelConfig::default();
        assert_eq!(config.phase_timeout(CognitivePhase::Perceive), Duration::from_secs(10));
    }

    #[test]
    fn test_think_timeout_is_60s() {
        let config = KernelConfig::default();
        assert_eq!(config.phase_timeout(CognitivePhase::Think), Duration::from_secs(60));
    }

    #[test]
    fn test_act_timeout_is_300s() {
        let config = KernelConfig::default();
        assert_eq!(config.phase_timeout(CognitivePhase::Act), Duration::from_secs(300));
    }

    #[test]
    fn test_set_phase_timeout_overrides() {
        let mut config = KernelConfig::default();
        config.set_phase_timeout(CognitivePhase::Think, Duration::from_secs(120));
        assert_eq!(config.phase_timeout(CognitivePhase::Think), Duration::from_secs(120));
        // Other phases unchanged
        assert_eq!(config.phase_timeout(CognitivePhase::Perceive), Duration::from_secs(10));
    }

    #[test]
    fn test_perceive_error_behavior_is_skip_and_continue() {
        let config = default_phase_config(CognitivePhase::Perceive);
        assert_eq!(config.error_behavior, ErrorBehavior::SkipAndContinue);
    }

    #[test]
    fn test_learn_error_behavior_is_log_and_continue() {
        let config = default_phase_config(CognitivePhase::Learn);
        assert_eq!(config.error_behavior, ErrorBehavior::LogAndContinue);
    }

    #[test]
    fn test_act_checkpoint_level_is_atomic() {
        let config = default_phase_config(CognitivePhase::Act);
        assert_eq!(config.checkpoint_level, CheckpointLevel::Atomic);
    }

    #[test]
    fn test_decide_is_user_interruptible() {
        let config = default_phase_config(CognitivePhase::Decide);
        assert!(config.user_interruptible);
        assert!(!config.system_interruptible);
    }

    #[test]
    fn test_learn_is_not_user_interruptible() {
        let config = default_phase_config(CognitivePhase::Learn);
        assert!(!config.user_interruptible);
        assert!(config.system_interruptible);
    }

    #[test]
    fn test_phase_timeout_returns_default_for_missing_phase() {
        // Create a config with no phases configured
        let config = KernelConfig {
            phase_configs: std::collections::HashMap::new(),
            max_recursion_depth: 5,
            max_think_iterations: 10,
            token_budget: 100_000,
        };
        // Should fall back to 30s
        assert_eq!(config.phase_timeout(CognitivePhase::Think), Duration::from_secs(30));
    }
}
