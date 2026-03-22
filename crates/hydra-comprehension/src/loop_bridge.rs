//! Bridge utilities for the cognitive loop.
//!
//! Provides minimal constructors used by hydra-kernel when building
//! a `ComprehendedInput` without running the full comprehension pipeline.

use crate::domain::Domain;
use crate::output::{ComprehendedInput, InputSource};
use crate::resonance::ResonanceResult;
use crate::temporal::{ConstraintStatus, Horizon, TemporalContext};

impl ComprehendedInput {
    /// Construct a minimal `ComprehendedInput` from raw text.
    ///
    /// All fields are set to neutral defaults. Used by the kernel's
    /// perceive stage when bypassing the full comprehension pipeline.
    pub fn minimal(raw: &str) -> Self {
        Self {
            raw: raw.to_string(),
            primary_domain: Domain::Unknown,
            all_domains: vec![(Domain::Unknown, 0.0)],
            primitives: Vec::new(),
            temporal: TemporalContext {
                urgency: 0.5,
                horizon: Horizon::ShortTerm,
                constraint_status: ConstraintStatus::None,
            },
            resonance: ResonanceResult::empty(),
            source: InputSource::PrincipalText,
            confidence: 0.0,
            used_llm: false,
        }
    }
}
