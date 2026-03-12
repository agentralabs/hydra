//! Extended onboarding flow (Step 4.4).

use serde::{Deserialize, Serialize};

/// Extended onboarding step enum covering the full setup flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OnboardingFlowStep {
    Welcome,
    ApiKeySetup,
    ModelSelection,
    ModePreference,
    Complete,
}

impl OnboardingFlowStep {
    fn index(self) -> usize {
        match self {
            Self::Welcome => 0,
            Self::ApiKeySetup => 1,
            Self::ModelSelection => 2,
            Self::ModePreference => 3,
            Self::Complete => 4,
        }
    }

    fn from_index(idx: usize) -> Option<Self> {
        match idx {
            0 => Some(Self::Welcome),
            1 => Some(Self::ApiKeySetup),
            2 => Some(Self::ModelSelection),
            3 => Some(Self::ModePreference),
            4 => Some(Self::Complete),
            _ => None,
        }
    }
}

/// Full onboarding flow wrapping all setup steps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingFlow {
    pub current_step: OnboardingFlowStep,
    pub total_steps: usize,
    pub api_key_entered: bool,
    pub selected_model: String,
    pub selected_mode: String,
    pub user_name: String,
}

impl OnboardingFlow {
    /// Create a new onboarding flow starting at Welcome.
    pub fn new() -> Self {
        Self {
            current_step: OnboardingFlowStep::Welcome,
            total_steps: 5,
            api_key_entered: false,
            selected_model: String::new(),
            selected_mode: String::new(),
            user_name: String::new(),
        }
    }

    /// Advance to the next step. Returns `false` if already at the end.
    pub fn next_step(&mut self) -> bool {
        let idx = self.current_step.index();
        if idx + 1 >= self.total_steps {
            return false;
        }
        if let Some(next) = OnboardingFlowStep::from_index(idx + 1) {
            self.current_step = next;
            true
        } else {
            false
        }
    }

    /// Go back to the previous step. Returns `false` if already at the beginning.
    pub fn prev_step(&mut self) -> bool {
        let idx = self.current_step.index();
        if idx == 0 {
            return false;
        }
        if let Some(prev) = OnboardingFlowStep::from_index(idx - 1) {
            self.current_step = prev;
            true
        } else {
            false
        }
    }

    /// Progress percentage (0–100) based on current step index.
    pub fn progress_percent(&self) -> u8 {
        let idx = self.current_step.index() as u32;
        let total = (self.total_steps as u32).saturating_sub(1).max(1);
        ((idx * 100) / total).min(100) as u8
    }

    /// Whether the current step has the required input to proceed.
    pub fn can_proceed(&self) -> bool {
        match self.current_step {
            OnboardingFlowStep::Welcome => true,
            OnboardingFlowStep::ApiKeySetup => self.api_key_entered,
            OnboardingFlowStep::ModelSelection => !self.selected_model.is_empty(),
            OnboardingFlowStep::ModePreference => !self.selected_mode.is_empty(),
            OnboardingFlowStep::Complete => true,
        }
    }

    /// Whether all steps have been completed.
    pub fn complete(&self) -> bool {
        self.current_step == OnboardingFlowStep::Complete
    }
}

impl Default for OnboardingFlow {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flow_starts_at_welcome() {
        let flow = OnboardingFlow::new();
        assert_eq!(flow.current_step, OnboardingFlowStep::Welcome);
        assert_eq!(flow.total_steps, 5);
        assert!(!flow.api_key_entered);
        assert!(flow.selected_model.is_empty());
        assert!(!flow.complete());
    }

    #[test]
    fn test_flow_next_step_advances() {
        let mut flow = OnboardingFlow::new();
        assert!(flow.next_step());
        assert_eq!(flow.current_step, OnboardingFlowStep::ApiKeySetup);
        assert!(flow.next_step());
        assert_eq!(flow.current_step, OnboardingFlowStep::ModelSelection);
        assert!(flow.next_step());
        assert_eq!(flow.current_step, OnboardingFlowStep::ModePreference);
        assert!(flow.next_step());
        assert_eq!(flow.current_step, OnboardingFlowStep::Complete);
        // Cannot advance past Complete
        assert!(!flow.next_step());
        assert_eq!(flow.current_step, OnboardingFlowStep::Complete);
    }

    #[test]
    fn test_flow_prev_step() {
        let mut flow = OnboardingFlow::new();
        // Cannot go back from Welcome
        assert!(!flow.prev_step());
        flow.next_step(); // -> ApiKeySetup
        flow.next_step(); // -> ModelSelection
        assert!(flow.prev_step());
        assert_eq!(flow.current_step, OnboardingFlowStep::ApiKeySetup);
        assert!(flow.prev_step());
        assert_eq!(flow.current_step, OnboardingFlowStep::Welcome);
    }

    #[test]
    fn test_flow_progress_percent() {
        let mut flow = OnboardingFlow::new();
        assert_eq!(flow.progress_percent(), 0);
        flow.next_step(); // step 1
        assert_eq!(flow.progress_percent(), 25);
        flow.next_step(); // step 2
        assert_eq!(flow.progress_percent(), 50);
        flow.next_step(); // step 3
        assert_eq!(flow.progress_percent(), 75);
        flow.next_step(); // step 4 (Complete)
        assert_eq!(flow.progress_percent(), 100);
    }

    #[test]
    fn test_flow_can_proceed_validation() {
        let mut flow = OnboardingFlow::new();
        // Welcome — always true
        assert!(flow.can_proceed());

        flow.next_step(); // -> ApiKeySetup
        assert!(!flow.can_proceed()); // no key yet
        flow.api_key_entered = true;
        assert!(flow.can_proceed());

        flow.next_step(); // -> ModelSelection
        assert!(!flow.can_proceed()); // no model
        flow.selected_model = "gpt-4".to_string();
        assert!(flow.can_proceed());

        flow.next_step(); // -> ModePreference
        assert!(!flow.can_proceed()); // no mode
        flow.selected_mode = "autonomous".to_string();
        assert!(flow.can_proceed());

        flow.next_step(); // -> Complete
        assert!(flow.complete());
        assert!(flow.can_proceed());
    }
}
