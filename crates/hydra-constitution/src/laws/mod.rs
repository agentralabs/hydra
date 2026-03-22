//! The seven constitutional laws of Hydra.
//! Every law is a type implementing ConstitutionalLaw.
//! All seven are checked by ConstitutionChecker before any action executes.

pub mod law1_receipt_immutability;
pub mod law2_identity_integrity;
pub mod law3_memory_sovereignty;
pub mod law4_constitution_protection;
pub mod law5_animus_integrity;
pub mod law6_principal_supremacy;
pub mod law7_causal_chain_completeness;

pub use law1_receipt_immutability::ReceiptImmutability;
pub use law2_identity_integrity::IdentityIntegrity;
pub use law3_memory_sovereignty::MemorySovereignty;
pub use law4_constitution_protection::ConstitutionProtection;
pub use law5_animus_integrity::AnimusIntegrity;
pub use law6_principal_supremacy::PrincipalSupremacy;
pub use law7_causal_chain_completeness::CausalChainCompleteness;

use crate::errors::ConstitutionError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Identifies which constitutional law is being referenced.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LawId {
    Law1ReceiptImmutability,
    Law2IdentityIntegrity,
    Law3MemorySovereignty,
    Law4ConstitutionProtection,
    Law5AnimusIntegrity,
    Law6PrincipalSupremacy,
    Law7CausalChainCompleteness,
}

impl std::fmt::Display for LawId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Law1ReceiptImmutability => write!(f, "Law 1: Receipt Immutability"),
            Self::Law2IdentityIntegrity => write!(f, "Law 2: Identity Integrity"),
            Self::Law3MemorySovereignty => write!(f, "Law 3: Memory Sovereignty"),
            Self::Law4ConstitutionProtection => write!(f, "Law 4: Constitution Protection"),
            Self::Law5AnimusIntegrity => write!(f, "Law 5: Animus Integrity"),
            Self::Law6PrincipalSupremacy => write!(f, "Law 6: Principal Supremacy"),
            Self::Law7CausalChainCompleteness => write!(f, "Law 7: Causal Chain Completeness"),
        }
    }
}

/// Every constitutional law implements this trait.
pub trait ConstitutionalLaw: Send + Sync {
    /// Which law this is.
    fn law_id(&self) -> LawId;

    /// Human-readable description of the law.
    fn description(&self) -> &'static str;

    /// Check whether the proposed action violates this law.
    /// Returns Ok(()) if the action is permitted.
    /// Returns Err(ConstitutionError) if the action must be blocked.
    fn check(&self, context: &LawCheckContext) -> Result<(), ConstitutionError>;
}

/// The context passed to every law check.
/// Describes the action being proposed and its environment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LawCheckContext {
    /// Unique ID of the proposed action.
    pub action_id: String,

    /// Dot-namespaced action type, e.g. "receipt.delete" or "agent.spawn".
    pub action_type: String,

    /// Trust tier of the entity proposing the action.
    pub source_tier: u8,

    /// The target of the action (receipt ID, agent ID, memory key, etc.).
    pub target: String,

    /// The causal chain: list of ancestor action IDs.
    /// Must terminate at CONSTITUTIONAL_IDENTITY_ID.
    pub causal_chain: Vec<String>,

    /// Additional key-value context specific to the action type.
    pub metadata: HashMap<String, String>,
}

impl LawCheckContext {
    /// Create a minimal context. Caller should add fields as needed.
    pub fn new(action_id: impl Into<String>, action_type: impl Into<String>) -> Self {
        Self {
            action_id: action_id.into(),
            action_type: action_type.into(),
            source_tier: crate::constants::TRUST_TIER_EXTERNAL,
            target: String::new(),
            causal_chain: vec![crate::constants::CONSTITUTIONAL_IDENTITY_ID.to_string()],
            metadata: HashMap::new(),
        }
    }

    /// Builder: set the source trust tier.
    pub fn with_tier(mut self, tier: u8) -> Self {
        self.source_tier = tier;
        self
    }

    /// Builder: set the action target.
    pub fn with_target(mut self, target: impl Into<String>) -> Self {
        self.target = target.into();
        self
    }

    /// Builder: set the full causal chain.
    pub fn with_causal_chain(mut self, chain: Vec<String>) -> Self {
        self.causal_chain = chain;
        self
    }

    /// Builder: add a metadata entry.
    pub fn with_meta(mut self, key: impl Into<String>, val: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), val.into());
        self
    }

    /// Returns true if the causal chain terminates at the constitutional identity.
    pub fn chain_is_complete(&self) -> bool {
        self.causal_chain
            .last()
            .map(|id| id == crate::constants::CONSTITUTIONAL_IDENTITY_ID)
            .unwrap_or(false)
    }
}
