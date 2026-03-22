//! Trust score and tier definitions.

use crate::constants::*;
use crate::errors::TrustError;
use serde::{Deserialize, Serialize};

/// A trust score bounded to [0.0, 1.0].
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct TrustScore(f64);

impl TrustScore {
    /// Create a new trust score, validating the range.
    pub fn new(value: f64) -> Result<Self, TrustError> {
        if !(TRUST_SCORE_MIN..=TRUST_SCORE_MAX).contains(&value) || value.is_nan() {
            return Err(TrustError::ScoreOutOfRange { value });
        }
        Ok(Self(value))
    }

    /// Create the default trust score.
    pub fn default_score() -> Self {
        Self(TRUST_SCORE_DEFAULT)
    }

    /// Return the raw f64 value.
    pub fn value(&self) -> f64 {
        self.0
    }

    /// Apply a positive adjustment (clamped to max).
    pub fn increase(&mut self, delta: f64) {
        self.0 = (self.0 + delta).min(TRUST_SCORE_MAX);
    }

    /// Apply a negative adjustment (clamped to min).
    pub fn decrease(&mut self, delta: f64) {
        self.0 = (self.0 - delta).max(TRUST_SCORE_MIN);
    }

    /// Return the tier this score corresponds to.
    pub fn tier(&self) -> TrustTier {
        if self.0 >= T_FLEET_HIGHLY_TRUSTED {
            TrustTier::Platinum
        } else if self.0 >= T_FLEET_TRUSTED {
            TrustTier::Gold
        } else if self.0 >= T_FLEET_UNRELIABLE {
            TrustTier::Silver
        } else {
            TrustTier::Bronze
        }
    }
}

impl std::fmt::Display for TrustScore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.4}", self.0)
    }
}

/// Trust tiers based on score ranges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TrustTier {
    /// Lowest tier: score < 0.3.
    Bronze,
    /// Mid tier: 0.3 <= score < 0.7.
    Silver,
    /// High tier: 0.7 <= score < 0.9.
    Gold,
    /// Highest tier: score >= 0.9.
    Platinum,
}

impl TrustTier {
    /// Return the energy level for this tier (used in Hamiltonian).
    pub fn energy(&self) -> f64 {
        match self {
            Self::Platinum => ENERGY_CONSTITUTION,
            Self::Gold => ENERGY_HYDRA,
            Self::Silver => ENERGY_FLEET,
            Self::Bronze => ENERGY_EXTERNAL,
        }
    }

    /// Return a numeric index for this tier.
    pub fn index(&self) -> usize {
        match self {
            Self::Bronze => 0,
            Self::Silver => 1,
            Self::Gold => 2,
            Self::Platinum => 3,
        }
    }
}

impl std::fmt::Display for TrustTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Bronze => "Bronze",
            Self::Silver => "Silver",
            Self::Gold => "Gold",
            Self::Platinum => "Platinum",
        };
        write!(f, "{name}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_scores() {
        assert!(TrustScore::new(0.0).is_ok());
        assert!(TrustScore::new(0.5).is_ok());
        assert!(TrustScore::new(1.0).is_ok());
    }

    #[test]
    fn invalid_scores() {
        assert!(TrustScore::new(-0.1).is_err());
        assert!(TrustScore::new(1.1).is_err());
        assert!(TrustScore::new(f64::NAN).is_err());
    }

    #[test]
    fn tier_from_score() {
        assert_eq!(TrustScore::new(0.1).unwrap().tier(), TrustTier::Bronze);
        assert_eq!(TrustScore::new(0.5).unwrap().tier(), TrustTier::Silver);
        assert_eq!(TrustScore::new(0.8).unwrap().tier(), TrustTier::Gold);
        assert_eq!(TrustScore::new(0.95).unwrap().tier(), TrustTier::Platinum);
    }

    #[test]
    fn tier_energy_ordering() {
        assert!(TrustTier::Platinum.energy() < TrustTier::Gold.energy());
        assert!(TrustTier::Gold.energy() < TrustTier::Silver.energy());
        assert!(TrustTier::Silver.energy() < TrustTier::Bronze.energy());
    }
}
