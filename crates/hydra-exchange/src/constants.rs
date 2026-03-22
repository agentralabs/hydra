//! Constants for the exchange subsystem.
//!
//! All tuneable values live here — no magic numbers elsewhere.

/// Maximum offers in the registry.
pub const MAX_EXCHANGE_OFFERS: usize = 1_000;

/// Maximum receipts stored.
pub const MAX_EXCHANGE_RECEIPTS: usize = 1_000_000;

/// Minimum trust score to accept an incoming request.
pub const MIN_TRUST_FOR_EXCHANGE: f64 = 0.60;

/// Minimum wisdom confidence to execute an exchange.
pub const MIN_WISDOM_FOR_EXCHANGE: f64 = 0.65;

/// Maximum exchange value without escalation (cost units).
pub const MAX_UNESCALATED_EXCHANGE_VALUE: f64 = 500.0;

/// Exchange receipt hash label.
pub const EXCHANGE_HASH_LABEL: &str = "sha256-exchange";
