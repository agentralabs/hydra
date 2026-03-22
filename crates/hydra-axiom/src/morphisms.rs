//! Axiom morphisms — the arrows in the axiom category.

use serde::{Deserialize, Serialize};

/// A morphism (arrow) connecting axiom primitives.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AxiomMorphism {
    /// Source causes target.
    Causes,
    /// Source constrains target.
    Constrains,
    /// Source optimizes for target.
    OptimizesFor,
    /// Source depends on target.
    DependsOn,
    /// Source allocates resources to target.
    Allocates,
    /// Source propagates information to target.
    PropagatesTo,
    /// Source emerges from target.
    EmergesFrom,
    /// Source informs target.
    Informs,
    /// Source coordinates with target.
    CoordinatesWith,
    /// A domain-specific morphism.
    DomainMorphism(String),
}

impl AxiomMorphism {
    /// Returns true if this morphism supports composition (a -> b -> c).
    pub fn is_compositional(&self) -> bool {
        match self {
            Self::Causes | Self::DependsOn | Self::PropagatesTo | Self::Informs => true,
            Self::Constrains
            | Self::OptimizesFor
            | Self::Allocates
            | Self::EmergesFrom
            | Self::CoordinatesWith => false,
            Self::DomainMorphism(_) => false,
        }
    }

    /// Return a human-readable label for this morphism.
    pub fn label(&self) -> &str {
        match self {
            Self::Causes => "causes",
            Self::Constrains => "constrains",
            Self::OptimizesFor => "optimizes-for",
            Self::DependsOn => "depends-on",
            Self::Allocates => "allocates",
            Self::PropagatesTo => "propagates-to",
            Self::EmergesFrom => "emerges-from",
            Self::Informs => "informs",
            Self::CoordinatesWith => "coordinates-with",
            Self::DomainMorphism(name) => name.as_str(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compositional_morphisms() {
        assert!(AxiomMorphism::Causes.is_compositional());
        assert!(AxiomMorphism::DependsOn.is_compositional());
        assert!(!AxiomMorphism::Constrains.is_compositional());
        assert!(!AxiomMorphism::CoordinatesWith.is_compositional());
    }

    #[test]
    fn labels() {
        assert_eq!(AxiomMorphism::Causes.label(), "causes");
        assert_eq!(
            AxiomMorphism::DomainMorphism("custom".into()).label(),
            "custom"
        );
    }
}
