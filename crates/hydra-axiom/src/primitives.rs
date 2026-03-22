//! Axiom primitives — the universal building blocks of reasoning.

use serde::{Deserialize, Serialize};

/// A universal reasoning primitive that exists across all domains.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AxiomPrimitive {
    /// Represents unknown or incomplete information.
    Uncertainty,
    /// A probability measure over outcomes.
    Probability,
    /// A hard or soft constraint on a system.
    Constraint,
    /// A risk assessment of potential negative outcomes.
    Risk,
    /// An optimization objective or target.
    Optimization,
    /// A model of adversarial behavior or threats.
    AdversarialModel,
    /// A cause-effect relationship between entities.
    CausalLink,
    /// A temporal ordering of events.
    TemporalSequence,
    /// Allocation of limited resources.
    ResourceAllocation,
    /// The information-theoretic value of a signal.
    InformationValue,
    /// A dependency between components.
    Dependency,
    /// A trust relationship between entities.
    TrustRelation,
    /// A game-theoretic equilibrium in coordination.
    CoordinationEquilibrium,
    /// An emergent pattern arising from simpler components.
    EmergencePattern,
    /// A domain-specific primitive with a custom label.
    DomainPrimitive(String),
}

impl AxiomPrimitive {
    /// Returns true if this is a base (non-domain) primitive.
    pub fn is_base(&self) -> bool {
        !matches!(self, Self::DomainPrimitive(_))
    }

    /// Return a human-readable label for this primitive.
    pub fn label(&self) -> &str {
        match self {
            Self::Uncertainty => "uncertainty",
            Self::Probability => "probability",
            Self::Constraint => "constraint",
            Self::Risk => "risk",
            Self::Optimization => "optimization",
            Self::AdversarialModel => "adversarial-model",
            Self::CausalLink => "causal-link",
            Self::TemporalSequence => "temporal-sequence",
            Self::ResourceAllocation => "resource-allocation",
            Self::InformationValue => "information-value",
            Self::Dependency => "dependency",
            Self::TrustRelation => "trust-relation",
            Self::CoordinationEquilibrium => "coordination-equilibrium",
            Self::EmergencePattern => "emergence-pattern",
            Self::DomainPrimitive(name) => name.as_str(),
        }
    }

    /// Compute similarity between two primitives.
    ///
    /// Returns:
    /// - 0.9 for same variant
    /// - 0.8 for related variants (same semantic cluster)
    /// - 0.5 for semantically close variants
    /// - 0.0 for unrelated variants
    pub fn similarity(&self, other: &Self) -> f64 {
        if self == other {
            return 0.9;
        }

        let group_a = self.semantic_group();
        let group_b = other.semantic_group();

        if group_a == group_b {
            return 0.8;
        }

        // Cross-group closeness
        if self.is_cross_close(other) {
            return 0.5;
        }

        0.0
    }

    /// Internal grouping for similarity computation.
    fn semantic_group(&self) -> u8 {
        match self {
            Self::Uncertainty | Self::Probability | Self::Risk => 0,
            Self::Constraint | Self::Optimization | Self::ResourceAllocation => 1,
            Self::CausalLink | Self::TemporalSequence | Self::Dependency => 2,
            Self::AdversarialModel | Self::TrustRelation => 3,
            Self::CoordinationEquilibrium | Self::EmergencePattern => 4,
            Self::InformationValue => 5,
            Self::DomainPrimitive(_) => 6,
        }
    }

    /// Check if two primitives from different groups are still close.
    fn is_cross_close(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::Risk, Self::AdversarialModel)
                | (Self::AdversarialModel, Self::Risk)
                | (Self::Dependency, Self::Constraint)
                | (Self::Constraint, Self::Dependency)
                | (Self::InformationValue, Self::Probability)
                | (Self::Probability, Self::InformationValue)
                | (Self::EmergencePattern, Self::CausalLink)
                | (Self::CausalLink, Self::EmergencePattern)
                | (Self::TrustRelation, Self::CoordinationEquilibrium)
                | (Self::CoordinationEquilibrium, Self::TrustRelation)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_variant_similarity() {
        let a = AxiomPrimitive::Risk;
        assert!((a.similarity(&a) - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn related_variant_similarity() {
        let a = AxiomPrimitive::Risk;
        let b = AxiomPrimitive::Uncertainty;
        assert!((a.similarity(&b) - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn cross_close_similarity() {
        let a = AxiomPrimitive::Risk;
        let b = AxiomPrimitive::AdversarialModel;
        assert!((a.similarity(&b) - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn unrelated_similarity() {
        let a = AxiomPrimitive::Optimization;
        let b = AxiomPrimitive::TrustRelation;
        assert!((a.similarity(&b)).abs() < f64::EPSILON);
    }

    #[test]
    fn base_check() {
        assert!(AxiomPrimitive::Risk.is_base());
        assert!(!AxiomPrimitive::DomainPrimitive("x".into()).is_base());
    }
}
