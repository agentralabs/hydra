//! The Animus internal bus.

pub mod channel;
pub mod router;
pub mod signature;

pub use channel::SignalChannel;
pub use router::{route, RoutingDecision};
pub use signature::{BusSigningKey, BusVerifyingKey};

use crate::{
    errors::AnimusError,
    semiring::{signal::Signal, validate_chain},
};

/// Validates a signal before it enters the bus.
pub fn validate_for_bus(signal: &Signal) -> Result<(), AnimusError> {
    validate_chain(signal)?;
    signal.validate_chain_depth()?;
    Ok(())
}
