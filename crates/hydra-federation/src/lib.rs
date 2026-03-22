//! `hydra-federation` — Peer discovery and trust negotiation.
//!
//! Every Hydra instance is sovereign.
//! Federation connects peers without subordinating either.
//!
//! Identity verified before any scope is proposed.
//! Scope agreed by both sides before any sharing begins.
//! Session bounds all sharing in time.
//! Every sharing event is receipted.
//!
//! Layer 6 begins here.

pub mod constants;
pub mod engine;
pub mod errors;
pub mod peer;
pub mod registry;
pub mod scope;
pub mod session;

pub use engine::{FederationEngine, HandshakeResult};
pub use errors::FederationError;
pub use peer::{test_fingerprint, FederationPeer, PeerAddress, PeerCapability};
pub use registry::PeerRegistry;
pub use scope::{NegotiationState, ScopeItem, TrustScope};
pub use session::{FederationSession, SessionState, SharingEvent};
