use std::sync::atomic::{AtomicU8, Ordering};

use hydra_core::types::IconState;

/// State machine for the living icon (8 states)
pub struct IconStateMachine {
    current: AtomicU8,
}

impl IconStateMachine {
    pub fn new() -> Self {
        Self {
            current: AtomicU8::new(Self::state_to_u8(IconState::Idle)),
        }
    }

    /// Get current icon state
    pub fn current(&self) -> IconState {
        Self::u8_to_state(self.current.load(Ordering::SeqCst))
    }

    /// Transition to a new state
    pub fn transition(&self, new_state: IconState) {
        self.current
            .store(Self::state_to_u8(new_state), Ordering::SeqCst);
    }

    /// Check if a transition is valid from the current state
    pub fn can_transition(&self, target: IconState) -> bool {
        let current = self.current();
        // Offline can only transition to Idle (reconnect)
        if current == IconState::Offline {
            return target == IconState::Idle;
        }
        // All other transitions are valid
        true
    }

    /// Transition only if valid
    pub fn try_transition(&self, target: IconState) -> bool {
        if self.can_transition(target) {
            self.transition(target);
            true
        } else {
            false
        }
    }

    /// Get the animation descriptor for the current state
    pub fn current_animation(&self) -> &'static str {
        self.current().animation_description()
    }

    fn state_to_u8(state: IconState) -> u8 {
        match state {
            IconState::Idle => 0,
            IconState::Listening => 1,
            IconState::Working => 2,
            IconState::NeedsAttention => 3,
            IconState::ApprovalNeeded => 4,
            IconState::Success => 5,
            IconState::Error => 6,
            IconState::Offline => 7,
        }
    }

    fn u8_to_state(val: u8) -> IconState {
        match val {
            0 => IconState::Idle,
            1 => IconState::Listening,
            2 => IconState::Working,
            3 => IconState::NeedsAttention,
            4 => IconState::ApprovalNeeded,
            5 => IconState::Success,
            6 => IconState::Error,
            7 => IconState::Offline,
            _ => IconState::Idle,
        }
    }
}

impl Default for IconStateMachine {
    fn default() -> Self {
        Self::new()
    }
}
