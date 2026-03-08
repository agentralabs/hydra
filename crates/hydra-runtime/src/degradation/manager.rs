//! Degradation manager — coordinates level transitions and action execution.

use std::fmt;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use super::actions::{self, DegradationAction};
use super::monitor::{ResourceMonitor, ResourceSnapshot};
use super::policy::{DegradationPolicy, PolicyConfig};

/// Degradation level — how aggressively Hydra conserves resources
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DegradationLevel {
    /// All features enabled
    Normal,
    /// Disable shadow_sim, reduce cache 50%
    Reduced,
    /// Disable local LLM, unload sisters, Haiku only
    Minimal,
    /// Pause new runs, aggressive GC, survival mode
    Emergency,
}

impl DegradationLevel {
    /// Step down one level toward Normal
    pub fn step_down(self) -> Self {
        match self {
            Self::Emergency => Self::Minimal,
            Self::Minimal => Self::Reduced,
            Self::Reduced => Self::Normal,
            Self::Normal => Self::Normal,
        }
    }

    /// Step up one level toward Emergency
    pub fn step_up(self) -> Self {
        match self {
            Self::Normal => Self::Reduced,
            Self::Reduced => Self::Minimal,
            Self::Minimal => Self::Emergency,
            Self::Emergency => Self::Emergency,
        }
    }
}

impl fmt::Display for DegradationLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Normal => write!(f, "normal"),
            Self::Reduced => write!(f, "reduced"),
            Self::Minimal => write!(f, "minimal"),
            Self::Emergency => write!(f, "emergency"),
        }
    }
}

/// Record of a level transition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelTransition {
    pub from: DegradationLevel,
    pub to: DegradationLevel,
    pub reason: String,
    pub actions_taken: Vec<DegradationAction>,
    pub snapshot: ResourceSnapshot,
    pub timestamp: String,
}

/// The degradation manager: monitors resources and manages level transitions
pub struct DegradationManager {
    monitor: ResourceMonitor,
    policy: DegradationPolicy,
    level: parking_lot::Mutex<DegradationLevel>,
    /// Whether new runs are paused
    runs_paused: parking_lot::Mutex<bool>,
    /// History of transitions
    history: parking_lot::Mutex<Vec<LevelTransition>>,
    /// Last evaluation time
    last_eval: parking_lot::Mutex<Option<Instant>>,
}

impl DegradationManager {
    pub fn new(policy_config: PolicyConfig) -> Self {
        Self {
            monitor: ResourceMonitor::new(),
            policy: DegradationPolicy::new(policy_config),
            level: parking_lot::Mutex::new(DegradationLevel::Normal),
            runs_paused: parking_lot::Mutex::new(false),
            history: parking_lot::Mutex::new(Vec::new()),
            last_eval: parking_lot::Mutex::new(None),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(PolicyConfig::default())
    }

    /// Create with a pre-set monitor snapshot (for testing)
    pub fn with_monitor(monitor: ResourceMonitor, policy_config: PolicyConfig) -> Self {
        Self {
            monitor,
            policy: DegradationPolicy::new(policy_config),
            level: parking_lot::Mutex::new(DegradationLevel::Normal),
            runs_paused: parking_lot::Mutex::new(false),
            history: parking_lot::Mutex::new(Vec::new()),
            last_eval: parking_lot::Mutex::new(None),
        }
    }

    /// Get current degradation level
    pub fn level(&self) -> DegradationLevel {
        *self.level.lock()
    }

    /// Whether new runs are paused
    pub fn runs_paused(&self) -> bool {
        *self.runs_paused.lock()
    }

    /// Get transition history
    pub fn history(&self) -> Vec<LevelTransition> {
        self.history.lock().clone()
    }

    /// Get the latest resource snapshot
    pub fn last_snapshot(&self) -> ResourceSnapshot {
        self.monitor.last()
    }

    /// Access the policy for manual overrides
    pub fn policy(&self) -> &DegradationPolicy {
        &self.policy
    }

    /// Evaluate current resources and possibly transition levels.
    /// Returns the list of actions taken (empty if no transition).
    pub fn evaluate(&self) -> Vec<DegradationAction> {
        let snapshot = self.monitor.snapshot();
        self.evaluate_snapshot(&snapshot)
    }

    /// Evaluate with a specific snapshot (for testing)
    pub fn evaluate_snapshot(&self, snapshot: &ResourceSnapshot) -> Vec<DegradationAction> {
        let current = *self.level.lock();
        let recommended = self.policy.evaluate(snapshot, current);

        *self.last_eval.lock() = Some(Instant::now());

        if recommended == current {
            return vec![];
        }

        self.transition_to(recommended, snapshot)
    }

    /// Force a level transition (manual override)
    pub fn force_level(&self, level: DegradationLevel) -> Vec<DegradationAction> {
        self.policy.set_override(Some(level));
        let snapshot = self.monitor.last();
        self.transition_to(level, &snapshot)
    }

    /// Clear manual override
    pub fn clear_override(&self) {
        self.policy.set_override(None);
    }

    /// Transition to a new level, executing actions
    fn transition_to(
        &self,
        to: DegradationLevel,
        snapshot: &ResourceSnapshot,
    ) -> Vec<DegradationAction> {
        let from = *self.level.lock();
        if from == to {
            return vec![];
        }

        let degradation_actions = actions::actions_for_transition(from, to);

        // Apply actions
        for action in &degradation_actions {
            self.apply_action(action);
        }

        // Record transition
        let transition = LevelTransition {
            from,
            to,
            reason: self.build_reason(snapshot, to),
            actions_taken: degradation_actions.clone(),
            snapshot: snapshot.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        if to > from {
            warn!(
                from = %from, to = %to,
                reason = %transition.reason,
                actions = degradation_actions.len(),
                "Degradation level increased"
            );
        } else {
            info!(
                from = %from, to = %to,
                "Degradation level recovered"
            );
        }

        self.history.lock().push(transition);
        *self.level.lock() = to;

        degradation_actions
    }

    fn apply_action(&self, action: &DegradationAction) {
        match action {
            DegradationAction::PauseNewRuns => {
                *self.runs_paused.lock() = true;
            }
            DegradationAction::ResumeRuns | DegradationAction::ResumeNormal => {
                *self.runs_paused.lock() = false;
            }
            // Other actions would interact with real subsystems:
            // - UnloadLocalModels → LocalModelManager::unload_all()
            // - UnloadSisters → SisterRegistry::disconnect_all()
            // - HaikuOnly → ModelExecutor::set_forced_model("claude-haiku")
            // - AggressiveGc → trigger system GC
            // For now, these are recorded but not executed against real systems
            _ => {}
        }
    }

    fn build_reason(&self, snapshot: &ResourceSnapshot, level: DegradationLevel) -> String {
        let config = self.policy.config();
        match level {
            DegradationLevel::Emergency => {
                format!(
                    "Memory at {:.1}% (threshold: {:.0}%)",
                    snapshot.memory_percent, config.memory_emergency
                )
            }
            DegradationLevel::Minimal => {
                if snapshot.memory_percent >= config.memory_minimal {
                    format!(
                        "Memory at {:.1}% (threshold: {:.0}%)",
                        snapshot.memory_percent, config.memory_minimal
                    )
                } else {
                    format!(
                        "Disk at {}MB (threshold: {}MB)",
                        snapshot.disk_available_mb, config.disk_minimal_mb
                    )
                }
            }
            DegradationLevel::Reduced => {
                if snapshot.memory_percent >= config.memory_reduced {
                    format!(
                        "Memory at {:.1}% (threshold: {:.0}%)",
                        snapshot.memory_percent, config.memory_reduced
                    )
                } else {
                    format!(
                        "CPU at {:.1}% (threshold: {:.0}%)",
                        snapshot.cpu_percent, config.cpu_reduced
                    )
                }
            }
            DegradationLevel::Normal => "Resources within normal limits".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(memory: f64, cpu: f64, disk: u64) -> ResourceSnapshot {
        ResourceSnapshot {
            memory_percent: memory,
            cpu_percent: cpu,
            disk_available_mb: disk,
            taken_at: Some(Instant::now()),
        }
    }

    #[test]
    fn test_degradation_levels_ordering() {
        assert!(DegradationLevel::Normal < DegradationLevel::Reduced);
        assert!(DegradationLevel::Reduced < DegradationLevel::Minimal);
        assert!(DegradationLevel::Minimal < DegradationLevel::Emergency);
    }

    #[test]
    fn test_step_up_down() {
        assert_eq!(
            DegradationLevel::Normal.step_up(),
            DegradationLevel::Reduced
        );
        assert_eq!(
            DegradationLevel::Emergency.step_down(),
            DegradationLevel::Minimal
        );
        assert_eq!(
            DegradationLevel::Normal.step_down(),
            DegradationLevel::Normal
        );
        assert_eq!(
            DegradationLevel::Emergency.step_up(),
            DegradationLevel::Emergency
        );
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", DegradationLevel::Normal), "normal");
        assert_eq!(format!("{}", DegradationLevel::Emergency), "emergency");
    }

    #[test]
    fn test_manager_starts_normal() {
        let mgr = DegradationManager::with_defaults();
        assert_eq!(mgr.level(), DegradationLevel::Normal);
        assert!(!mgr.runs_paused());
    }

    #[test]
    fn test_emergency_pauses_runs() {
        let mgr = DegradationManager::with_defaults();
        let actions = mgr.force_level(DegradationLevel::Emergency);
        assert!(mgr.runs_paused());
        assert!(actions.contains(&DegradationAction::PauseNewRuns));
        assert_eq!(mgr.level(), DegradationLevel::Emergency);
    }

    #[test]
    fn test_recovery_to_normal() {
        let mgr = DegradationManager::with_defaults();
        mgr.force_level(DegradationLevel::Emergency);
        assert!(mgr.runs_paused());

        mgr.clear_override();
        let actions = mgr.force_level(DegradationLevel::Normal);
        assert!(!mgr.runs_paused());
        assert!(actions.contains(&DegradationAction::ResumeRuns));
        assert!(actions.contains(&DegradationAction::ResumeNormal));
    }

    #[test]
    fn test_transition_history() {
        let mgr = DegradationManager::with_defaults();
        mgr.force_level(DegradationLevel::Reduced);
        mgr.force_level(DegradationLevel::Emergency);

        let history = mgr.history();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].from, DegradationLevel::Normal);
        assert_eq!(history[0].to, DegradationLevel::Reduced);
        assert_eq!(history[1].from, DegradationLevel::Reduced);
        assert_eq!(history[1].to, DegradationLevel::Emergency);
    }
}
