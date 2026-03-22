//! All constants for hydra-persona.
//! No magic numbers or strings anywhere else in this crate.

/// Maximum number of personas that can be registered.
pub const MAX_PERSONAS: usize = 64;

/// Maximum number of personas in a single blend.
pub const MAX_BLEND_PERSONAS: usize = 4;

/// Tolerance for blend weight sum validation (must sum to 1.0 within this).
pub const BLEND_WEIGHT_TOLERANCE: f64 = 1e-10;

/// Default persona name for the core Hydra persona.
pub const DEFAULT_PERSONA_NAME: &str = "hydra-core";
