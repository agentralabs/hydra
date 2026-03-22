//! Bridge utilities for the cognitive loop.
//!
//! Provides minimal constructors used by hydra-kernel when building
//! an `AttentionFrame` without running the full attention pipeline.

use crate::budget::AttentionBudget;
use crate::frame::AttentionFrame;

impl AttentionFrame {
    /// Construct a minimal `AttentionFrame` with empty item lists
    /// and a neutral attention budget.
    ///
    /// Used by the kernel's perceive stage when bypassing the full
    /// attention allocation pipeline.
    pub fn minimal() -> Self {
        let budget = AttentionBudget::compute(
            &hydra_language::IntentKind::StatusQuery,
            &hydra_language::AffectSignal {
                register: hydra_language::InteractionRegister::Neutral,
                confidence: 0.5,
                keywords_detected: vec![],
            },
        );
        Self {
            focus_items: Vec::new(),
            summary_items: Vec::new(),
            filtered_count: 0,
            budget,
        }
    }
}
