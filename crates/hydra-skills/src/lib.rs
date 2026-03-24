//! `hydra-skills` — Hot-loadable skill substrate.
//! Constitutional gating on every skill.
//! Knowledge persists in genome even after unload.

pub mod assumptions;
pub mod constants;
pub mod errors;
pub mod gate;
pub mod loader;
pub mod operations;
pub mod registry;
pub mod rubric;
pub mod skill;

pub use errors::SkillError;
pub use gate::{GateResult, SkillGate};
pub use loader::SkillLoader;
pub use registry::SkillRegistry;
pub use skill::{LoadedSkill, SkillDomain, SkillManifest};
