use hydra_core::types::{OnboardingState, OnboardingStep};

/// The 30-second onboarding flow
/// Step 1 (5s): Welcome
/// Step 2 (10s): Ask name
/// Step 3 (10s): Ask voice
/// Step 4 (5s): Complete
///
/// NO API KEYS. NO CONFIGURATION. NO FORMS.
pub struct OnboardingFlow {
    state: OnboardingState,
}

impl OnboardingFlow {
    pub fn new() -> Self {
        Self {
            state: OnboardingState::default(),
        }
    }

    pub fn state(&self) -> &OnboardingState {
        &self.state
    }

    pub fn current_step(&self) -> &OnboardingStep {
        &self.state.current_step
    }

    pub fn is_complete(&self) -> bool {
        self.state.completed
    }

    /// Get the prompt for the current step
    pub fn current_prompt(&self) -> &'static str {
        match self.state.current_step {
            OnboardingStep::Welcome => "Hi! I'm Hydra, your AI companion.",
            OnboardingStep::AskName => "What's your name?",
            OnboardingStep::AskVoice => "Would you like to enable voice? (Yes / Maybe later)",
            OnboardingStep::Complete => "All set! Let's get started.",
        }
    }

    /// Advance to the next step with user input
    pub fn advance(&mut self, input: Option<&str>) -> OnboardingStep {
        match self.state.current_step {
            OnboardingStep::Welcome => {
                self.state.current_step = OnboardingStep::AskName;
            }
            OnboardingStep::AskName => {
                if let Some(name) = input {
                    let name = name.trim();
                    if !name.is_empty() {
                        self.state.user_name = Some(name.to_string());
                    }
                }
                self.state.current_step = OnboardingStep::AskVoice;
            }
            OnboardingStep::AskVoice => {
                if let Some(answer) = input {
                    let answer = answer.trim().to_lowercase();
                    self.state.voice_enabled =
                        Some(answer == "yes" || answer == "y" || answer == "yeah");
                } else {
                    self.state.voice_enabled = Some(false);
                }
                self.state.current_step = OnboardingStep::Complete;
                self.state.completed = true;
            }
            OnboardingStep::Complete => {
                // Already complete, no-op
            }
        }
        self.state.current_step.clone()
    }

    /// Total number of steps
    pub fn total_steps(&self) -> usize {
        4
    }

    /// Current step index (0-based)
    pub fn step_index(&self) -> usize {
        match self.state.current_step {
            OnboardingStep::Welcome => 0,
            OnboardingStep::AskName => 1,
            OnboardingStep::AskVoice => 2,
            OnboardingStep::Complete => 3,
        }
    }

    /// Get user's name if provided
    pub fn user_name(&self) -> Option<&str> {
        self.state.user_name.as_deref()
    }

    /// Get voice preference if set
    pub fn voice_enabled(&self) -> Option<bool> {
        self.state.voice_enabled
    }
}

impl Default for OnboardingFlow {
    fn default() -> Self {
        Self::new()
    }
}
