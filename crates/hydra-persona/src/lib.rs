//! `hydra-persona` — Persona blending and voice system for Hydra.
//!
//! This crate provides:
//! - Named behavioral profiles (personas) with vocabulary, priorities, tone
//! - Weighted persona blending with strict weight validation
//! - A persona registry with the core persona pre-loaded
//! - Active voice management

pub mod blend;
pub mod constants;
pub mod errors;
pub mod persona;
pub mod registry;
pub mod voice;

pub use blend::{BlendComponent, BlendedVoice, PersonaBlend};
pub use errors::PersonaError;
pub use persona::Persona;
pub use registry::PersonaRegistry;
pub use voice::PersonaVoice;
