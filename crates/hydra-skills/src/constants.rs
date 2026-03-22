//! All constants for hydra-skills.
//! No magic numbers or strings anywhere else in this crate.

/// Maximum number of simultaneously loaded skills.
pub const MAX_LOADED_SKILLS: usize = 128;

/// Maximum length of a skill identifier string.
pub const SKILL_ID_MAX_LEN: usize = 64;

/// Maximum length of a skill version string.
pub const SKILL_VERSION_MAX_LEN: usize = 32;

/// Number of genome entries contributed per loaded skill.
pub const GENOME_ENTRIES_PER_SKILL: usize = 10;
