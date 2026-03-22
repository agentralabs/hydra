//! `hydra-cartography` — Digital topology.
//!
//! Maps the digital systems Hydra encounters, tracking their
//! classifications, interfaces, and relationships. The atlas
//! is append-only: systems are never removed.

pub mod atlas;
pub mod constants;
pub mod errors;
pub mod profile;
pub mod system_class;
pub mod topology;

pub use atlas::CartographyAtlas;
pub use errors::CartographyError;
pub use profile::SystemProfile;
pub use system_class::SystemClass;
pub use topology::TopologyMap;
