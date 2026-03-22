//! `hydra-skills` — Hot-loadable skill substrate.
//! Constitutional gating on every skill.
//! Knowledge persists in genome even after unload.

pub mod constants;
pub mod errors;
pub mod gate;
pub mod loader;
pub mod registry;
pub mod skill;

pub use errors::SkillError;
pub use gate::{GateResult, SkillGate};
pub use loader::SkillLoader;
pub use registry::SkillRegistry;
pub use skill::{LoadedSkill, SkillDomain, SkillManifest};
