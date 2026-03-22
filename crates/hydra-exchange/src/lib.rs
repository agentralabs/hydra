//! `hydra-exchange` — Capability exchange.
//!
//! Offer. Request. Execute. Receipt.
//! Trust-gated. Wisdom-checked. Immutably recorded.
//!
//! The economic interface through which Hydra's accumulated
//! intelligence flows to systems that need it.
//!
//! Layer 5 closes here.

pub mod constants;
pub mod engine;
pub mod errors;
pub mod offer;
pub mod receipt;
pub mod registry;
pub mod request;

pub use engine::{ExchangeEngine, ExchangeResult};
pub use errors::ExchangeError;
pub use offer::{ExchangeOffer, OfferKind, OfferState};
pub use receipt::{ExchangeOutcome, ExchangeReceipt};
pub use registry::ExchangeRegistry;
pub use request::{ExchangeRequest, RequestState};
