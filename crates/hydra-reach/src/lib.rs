//! `hydra-reach` — Universal device connectivity.
//! Hydra as a presence across any surface.
//! Same entity. Same memory. Same identity. Any device.

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
