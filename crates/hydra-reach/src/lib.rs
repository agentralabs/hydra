//! `hydra-reach` — Universal device connectivity (base types).
//! Hydra as a presence across any surface.
//! Same entity. Same memory. Same identity. Any device.
//!
//! NOTE: This crate provides BASE TYPES for device connectivity.
//! The production implementation is `hydra-reach-extended` which builds
//! on these types and is wired into the kernel at boot.
//! DO NOT wire hydra-reach directly into the kernel — use hydra-reach-extended.
//! This crate exists as the type foundation. Changing it affects reach-extended.

pub mod constants;
pub mod continuity;
pub mod device;
pub mod errors;
pub mod server;
pub mod session;
pub mod surface;

pub use continuity::{apply_handoff, prepare_handoff, HandoffPackage};
pub use device::{DeviceCapabilities, DeviceProfile};
pub use errors::ReachError;
pub use server::ReachServer;
pub use session::{DeviceSession, SessionState};
pub use surface::{OutputMode, SurfaceClass};
