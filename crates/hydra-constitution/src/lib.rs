//! `hydra-constitution` — The seven constitutional laws of Hydra.
//!
//! Every other crate in Hydra depends on this one.
//! This crate depends on nothing inside Hydra.
//!
//! The constitutional checker is the single entry point for all
//! constitutional enforcement. No action proceeds without passing it.

pub mod checker;
pub mod constants;
pub mod declarations;
pub mod errors;
pub mod identity;
pub mod invariants;
pub mod laws;
pub mod receipt;
pub mod task;

pub use checker::{CheckResult, ConstitutionChecker};
pub use declarations::{
    check_capability_declaration, check_growth_declaration, CapabilityCheckContext,
    GrowthCheckContext, HardStop,
};
pub use errors::ConstitutionError;
pub use identity::{PrincipalIdentity, TrustTier};
pub use laws::{ConstitutionalLaw, LawCheckContext, LawId};
pub use receipt::{Receipt, ReceiptChain, ReceiptId};
pub use task::{ApproachType, AttemptOutcome, AttemptRecord, ObstacleType, TaskId, TaskState};
