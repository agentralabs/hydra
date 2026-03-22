//! `hydra-language` — Semantic intent, affect, and depth analysis.
//!
//! Provides intent extraction, hedge detection, depth analysis, and affect
//! classification. Zero LLM calls — pure structural analysis.

pub mod affect;
pub mod constants;
pub mod depth;
pub mod engine;
pub mod errors;
pub mod hedge;
pub mod intent;

pub use affect::{detect_affect, AffectSignal, InteractionRegister};
pub use depth::{detect_depth, DepthLevel};
pub use engine::{LanguageAnalysis, LanguageEngine, ResponseDepth};
pub use errors::LanguageError;
pub use hedge::{detect_hedges, HedgeResult};
pub use intent::{extract_intent, IntentKind, IntentResult};
