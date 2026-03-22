//! TrustTier and PrincipalIdentity types.

use crate::constants::*;
use serde::{Deserialize, Serialize};

/// The trust tier of an entity in Hydra.
/// Lower number = higher authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TrustTier(u8);

impl TrustTier {
    /// Constitution tier (highest authority).
    pub fn constitution() -> Self {
        Self(TRUST_TIER_CONSTITUTION)
    }
    /// Hydra kernel tier.
    pub fn hydra() -> Self {
        Self(TRUST_TIER_HYDRA)
    }
    /// Principal (human) tier.
    pub fn principal() -> Self {
        Self(TRUST_TIER_PRINCIPAL)
    }
    /// Fleet agent tier.
    pub fn fleet() -> Self {
        Self(TRUST_TIER_FLEET)
    }
    /// Skills tier.
    pub fn skills() -> Self {
        Self(TRUST_TIER_SKILLS)
    }
    /// External tier (lowest authority).
    pub fn external() -> Self {
        Self(TRUST_TIER_EXTERNAL)
    }

    /// Create from a raw u8. Returns None if out of range.
    pub fn from_u8(v: u8) -> Option<Self> {
        if v <= TRUST_TIER_MAXIMUM {
            Some(Self(v))
        } else {
            None
        }
    }

    /// The raw tier value.
    pub fn value(&self) -> u8 {
        self.0
    }

    /// Returns true if this tier has authority over the other.
    /// Lower number = higher authority.
    pub fn has_authority_over(&self, other: &TrustTier) -> bool {
        self.0 < other.0
    }

    /// Returns true if this is the highest possible tier (constitution).
    pub fn is_supreme(&self) -> bool {
        self.0 == TRUST_TIER_CONSTITUTION
    }
}

impl std::fmt::Display for TrustTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self.0 {
            TRUST_TIER_CONSTITUTION => "Constitution",
            TRUST_TIER_HYDRA => "Hydra",
            TRUST_TIER_PRINCIPAL => "Principal",
            TRUST_TIER_FLEET => "Fleet",
            TRUST_TIER_SKILLS => "Skills",
            TRUST_TIER_EXTERNAL => "External",
            _ => "Unknown",
        };
        write!(f, "Tier{}({})", self.0, name)
    }
}

/// The principal identity — the one human authority above all agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrincipalIdentity {
    /// Human-readable name.
    pub name: String,
    /// Stable identifier (never changes even if name changes).
    pub id: String,
}

impl PrincipalIdentity {
    /// Create a new principal identity.
    pub fn new(name: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            id: id.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constitution_has_authority_over_all() {
        let constitution = TrustTier::constitution();
        assert!(constitution.has_authority_over(&TrustTier::hydra()));
        assert!(constitution.has_authority_over(&TrustTier::principal()));
        assert!(constitution.has_authority_over(&TrustTier::fleet()));
        assert!(constitution.has_authority_over(&TrustTier::external()));
    }

    #[test]
    fn external_has_no_authority_over_others() {
        let external = TrustTier::external();
        assert!(!external.has_authority_over(&TrustTier::fleet()));
        assert!(!external.has_authority_over(&TrustTier::skills()));
        assert!(!external.has_authority_over(&TrustTier::principal()));
    }

    #[test]
    fn trust_tier_ordering() {
        assert!(TrustTier::constitution() < TrustTier::hydra());
        assert!(TrustTier::hydra() < TrustTier::principal());
        assert!(TrustTier::principal() < TrustTier::fleet());
    }

    #[test]
    fn invalid_tier_returns_none() {
        assert!(TrustTier::from_u8(99).is_none());
        assert!(TrustTier::from_u8(6).is_none());
    }

    #[test]
    fn valid_tier_returns_some() {
        assert!(TrustTier::from_u8(0).is_some());
        assert!(TrustTier::from_u8(5).is_some());
    }
}
