//! `hydra-settlement` — Execution cost accounting.
//!
//! Every action Hydra takes settles into a record.
//! Every record is classified, hashed, and immutable.
//! Periods aggregate records into intelligence briefs.
//!
//! Without this: Hydra is a black box.
//! With this: "What did we spend this on?"
//!            is answerable in milliseconds.
//!
//! Layer 5 begins here.

pub mod constants;
pub mod cost;
pub mod engine;
pub mod errors;
pub mod ledger;
pub mod period;
pub mod persistence;
pub mod record;

pub use cost::{CostClass, CostItem};
pub use engine::SettlementEngine;
pub use errors::SettlementError;
pub use ledger::{SettlementLedger, SettlementQuery};
pub use period::{SettlementPeriod, SpendTrend};
pub use record::{Outcome, SettlementRecord};
