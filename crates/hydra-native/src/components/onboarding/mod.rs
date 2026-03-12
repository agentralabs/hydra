//! Onboarding flow component data.

mod flow;

pub use flow::*;

use serde::{Deserialize, Serialize};

/// Which step of the onboarding the user is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OnboardingStep {
    Intro,
    AskName,
    AskApiKey,
    AskVoice,
    Complete,
}

/// Persistent state for the onboarding flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingState {
    pub step: OnboardingStep,
    pub user_name: Option<String>,
    pub api_key: Option<String>,
    pub api_provider: Option<String>,
    pub voice_enabled: bool,
}

/// View model produced for each onboarding step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingView {
    pub title: String,
    pub subtitle: String,
    pub input_placeholder: Option<String>,
    pub primary_button: String,
    pub secondary_button: Option<String>,
    pub step_index: usize,
    pub total_steps: usize,
}

impl OnboardingState {
    pub fn new() -> Self {
        Self {
            step: OnboardingStep::Intro,
            user_name: None,
            api_key: None,
            api_provider: None,
            voice_enabled: false,
        }
    }

    /// Build the view model for the current step.
    pub fn current_view(&self) -> OnboardingView {
        let total_steps = 5;
        match self.step {
            OnboardingStep::Intro => OnboardingView {
                title: "Hi! I'm Hydra.".into(),
                subtitle: "I help with tasks, remember things, and keep you organized.".into(),
                input_placeholder: None,
                primary_button: "Continue".into(),
                secondary_button: None,
                step_index: 0,
                total_steps,
            },
            OnboardingStep::AskName => OnboardingView {
                title: "What should I call you?".into(),
                subtitle: "I'd love to know your name.".into(),
                input_placeholder: Some("Your name".into()),
                primary_button: "Continue".into(),
                secondary_button: None,
                step_index: 1,
                total_steps,
            },
            OnboardingStep::AskApiKey => {
                let name = self.user_name.as_deref().unwrap_or("friend");
                OnboardingView {
                    title: format!("Connect to AI, {}!", name),
                    subtitle: "Enter an API key to power Hydra's brain. Anthropic (Claude) recommended.".into(),
                    input_placeholder: Some("sk-ant-api03-... or sk-...".into()),
                    primary_button: "Continue".into(),
                    secondary_button: Some("Skip for now".into()),
                    step_index: 2,
                    total_steps,
                }
            }
            OnboardingStep::AskVoice => {
                let name = self.user_name.as_deref().unwrap_or("friend");
                OnboardingView {
                    title: format!("Nice to meet you, {}!", name),
                    subtitle: "Want to talk by voice?".into(),
                    input_placeholder: None,
                    primary_button: "Yes".into(),
                    secondary_button: Some("Maybe later".into()),
                    step_index: 3,
                    total_steps,
                }
            }
            OnboardingStep::Complete => {
                let name = self.user_name.as_deref().unwrap_or("friend");
                OnboardingView {
                    title: format!("All set, {}!", name),
                    subtitle: "I'll be in your menu bar.".into(),
                    input_placeholder: None,
                    primary_button: "Got it!".into(),
                    secondary_button: None,
                    step_index: 4,
                    total_steps,
                }
            }
        }
    }

    /// Set the user's name (typically on the AskName step).
    pub fn set_name(&mut self, name: &str) {
        self.user_name = Some(name.to_owned());
    }

    /// Set the API key and auto-detect provider.
    pub fn set_api_key(&mut self, key: &str) {
        let provider = if key.starts_with("sk-ant-") {
            "anthropic"
        } else if key.starts_with("sk-") {
            "openai"
        } else {
            "unknown"
        };
        self.api_key = Some(key.to_owned());
        self.api_provider = Some(provider.to_owned());
    }

    /// Enable voice input.
    pub fn enable_voice(&mut self) {
        self.voice_enabled = true;
    }

    /// Move to the next step.
    pub fn advance(&mut self) {
        self.step = match self.step {
            OnboardingStep::Intro => OnboardingStep::AskName,
            OnboardingStep::AskName => OnboardingStep::AskApiKey,
            OnboardingStep::AskApiKey => OnboardingStep::AskVoice,
            OnboardingStep::AskVoice => OnboardingStep::Complete,
            OnboardingStep::Complete => OnboardingStep::Complete,
        };
    }

    /// Whether the onboarding is finished.
    pub fn is_complete(&self) -> bool {
        self.step == OnboardingStep::Complete
    }
}

impl Default for OnboardingState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let state = OnboardingState::new();
        assert_eq!(state.step, OnboardingStep::Intro);
        assert!(state.user_name.is_none());
        assert!(!state.voice_enabled);
        assert!(!state.is_complete());
    }

    #[test]
    fn test_set_name() {
        let mut state = OnboardingState::new();
        state.set_name("Ada");
        assert_eq!(state.user_name.as_deref(), Some("Ada"));
    }

    #[test]
    fn test_advance_through_steps() {
        let mut state = OnboardingState::new();

        state.advance(); // Intro -> AskName
        assert_eq!(state.step, OnboardingStep::AskName);

        state.set_name("Ada");

        state.advance(); // AskName -> AskApiKey
        assert_eq!(state.step, OnboardingStep::AskApiKey);

        state.advance(); // AskApiKey -> AskVoice
        assert_eq!(state.step, OnboardingStep::AskVoice);
        assert!(!state.is_complete());

        state.advance(); // AskVoice -> Complete
        assert_eq!(state.step, OnboardingStep::Complete);
        assert!(state.is_complete());
    }

    #[test]
    fn test_advance_past_complete_is_noop() {
        let mut state = OnboardingState::new();
        state.advance(); // Intro -> AskName
        state.advance(); // AskName -> AskApiKey
        state.advance(); // AskApiKey -> AskVoice
        state.advance(); // AskVoice -> Complete
        assert!(state.is_complete());

        state.advance(); // should stay Complete
        assert!(state.is_complete());
    }

    #[test]
    fn test_voice_toggle() {
        let mut state = OnboardingState::new();
        assert!(!state.voice_enabled);
        state.enable_voice();
        assert!(state.voice_enabled);
    }

    #[test]
    fn test_current_view_intro() {
        let state = OnboardingState::new();
        let view = state.current_view();
        assert_eq!(view.step_index, 0);
        assert_eq!(view.total_steps, 5);
        assert!(view.title.contains("Hydra"));
    }

    #[test]
    fn test_current_view_ask_name() {
        let mut state = OnboardingState::new();
        state.advance(); // Intro -> AskName
        let view = state.current_view();
        assert_eq!(view.step_index, 1);
        assert_eq!(view.total_steps, 5);
        assert!(view.input_placeholder.is_some());
    }

    #[test]
    fn test_current_view_ask_api_key() {
        let mut state = OnboardingState::new();
        state.advance(); // Intro -> AskName
        state.set_name("Ada");
        state.advance(); // AskName -> AskApiKey
        let view = state.current_view();
        assert_eq!(view.step_index, 2);
        assert_eq!(view.total_steps, 5);
        assert!(view.title.contains("Ada"));
        assert!(view.input_placeholder.is_some());
        assert!(view.secondary_button.is_some()); // "Skip for now"
    }

    #[test]
    fn test_current_view_ask_voice_with_name() {
        let mut state = OnboardingState::new();
        state.advance(); // Intro -> AskName
        state.set_name("Ada");
        state.advance(); // AskName -> AskApiKey
        state.advance(); // AskApiKey -> AskVoice
        let view = state.current_view();
        assert_eq!(view.step_index, 3);
        assert!(view.title.contains("Ada"));
        assert!(view.secondary_button.is_some());
    }

    #[test]
    fn test_current_view_complete() {
        let mut state = OnboardingState::new();
        state.advance(); // Intro -> AskName
        state.set_name("Ada");
        state.advance(); // AskName -> AskApiKey
        state.advance(); // AskApiKey -> AskVoice
        state.advance(); // AskVoice -> Complete
        let view = state.current_view();
        assert_eq!(view.step_index, 4);
        assert!(view.title.contains("Ada"));
        assert_eq!(view.primary_button, "Got it!");
    }

    #[test]
    fn test_set_api_key_anthropic() {
        let mut state = OnboardingState::new();
        state.set_api_key("sk-ant-api03-test");
        assert_eq!(state.api_key.as_deref(), Some("sk-ant-api03-test"));
        assert_eq!(state.api_provider.as_deref(), Some("anthropic"));
    }

    #[test]
    fn test_set_api_key_openai() {
        let mut state = OnboardingState::new();
        state.set_api_key("sk-test-key");
        assert_eq!(state.api_provider.as_deref(), Some("openai"));
    }
}
