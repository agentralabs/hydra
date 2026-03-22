//! Domain functors — mappings from domain concepts to axiom primitives.

use crate::constants::{CROSS_DOMAIN_SIMILARITY_THRESHOLD, MAX_DOMAIN_FUNCTORS};
use crate::errors::AxiomError;
use crate::primitives::AxiomPrimitive;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A mapping from a domain concept to an axiom primitive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctorMapping {
    /// The domain-specific concept name.
    pub domain_concept: String,
    /// The axiom primitive it maps to.
    pub axiom_primitive: AxiomPrimitive,
    /// Confidence in this mapping (0.0 to 1.0).
    pub confidence: f64,
}

/// Trait for domain-specific functors that map domain concepts to axiom primitives.
pub trait DomainFunctor: Send + Sync {
    /// The unique domain name for this functor.
    fn domain_name(&self) -> &str;

    /// Map a domain concept string to an axiom primitive.
    fn map_concept(&self, concept: &str) -> Option<FunctorMapping>;

    /// List all known mappings for this domain.
    fn all_mappings(&self) -> Vec<FunctorMapping>;
}

/// Registry of domain functors for cross-domain pattern detection.
pub struct FunctorRegistry {
    functors: HashMap<String, Box<dyn DomainFunctor>>,
}

impl FunctorRegistry {
    /// Create a new empty functor registry.
    pub fn new() -> Self {
        Self {
            functors: HashMap::new(),
        }
    }

    /// Register a domain functor.
    pub fn register(&mut self, functor: Box<dyn DomainFunctor>) -> Result<(), AxiomError> {
        let name = functor.domain_name().to_string();
        if self.functors.len() >= MAX_DOMAIN_FUNCTORS {
            return Err(AxiomError::RegistryFull {
                max: MAX_DOMAIN_FUNCTORS,
            });
        }
        if self.functors.contains_key(&name) {
            return Err(AxiomError::DomainAlreadyRegistered { domain: name });
        }
        self.functors.insert(name, functor);
        Ok(())
    }

    /// Map a concept in a specific domain to its axiom primitive.
    pub fn map_concept(&self, domain: &str, concept: &str) -> Option<FunctorMapping> {
        self.functors
            .get(domain)
            .and_then(|f| f.map_concept(concept))
    }

    /// Find cross-domain patterns by comparing axiom primitives.
    ///
    /// Returns pairs of (domain_a, concept_a, domain_b, concept_b, similarity)
    /// where the underlying axiom primitives are sufficiently similar.
    pub fn find_cross_domain_patterns(&self) -> Vec<CrossDomainPattern> {
        let mut patterns = Vec::new();
        let domains: Vec<&String> = self.functors.keys().collect();

        for (i, domain_a) in domains.iter().enumerate() {
            let functor_a = &self.functors[*domain_a];
            let mappings_a = functor_a.all_mappings();

            for domain_b in domains.iter().skip(i + 1) {
                let functor_b = &self.functors[*domain_b];
                let mappings_b = functor_b.all_mappings();

                for ma in &mappings_a {
                    for mb in &mappings_b {
                        let sim = ma.axiom_primitive.similarity(&mb.axiom_primitive);
                        if sim >= CROSS_DOMAIN_SIMILARITY_THRESHOLD {
                            patterns.push(CrossDomainPattern {
                                domain_a: (*domain_a).clone(),
                                concept_a: ma.domain_concept.clone(),
                                domain_b: (*domain_b).clone(),
                                concept_b: mb.domain_concept.clone(),
                                similarity: sim,
                            });
                        }
                    }
                }
            }
        }

        patterns
    }

    /// Return the number of registered domains.
    pub fn domain_count(&self) -> usize {
        self.functors.len()
    }
}

impl Default for FunctorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// A detected cross-domain pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossDomainPattern {
    /// First domain name.
    pub domain_a: String,
    /// First domain concept.
    pub concept_a: String,
    /// Second domain name.
    pub domain_b: String,
    /// Second domain concept.
    pub concept_b: String,
    /// Similarity score between the axiom primitives.
    pub similarity: f64,
}

/// A test functor for the finance domain.
pub struct FinanceFunctor;

impl DomainFunctor for FinanceFunctor {
    fn domain_name(&self) -> &str {
        "finance"
    }

    fn map_concept(&self, concept: &str) -> Option<FunctorMapping> {
        let primitive = match concept {
            "volatility" => AxiomPrimitive::Uncertainty,
            "credit-risk" => AxiomPrimitive::Risk,
            "portfolio-optimization" => AxiomPrimitive::Optimization,
            "interest-rate-dependency" => AxiomPrimitive::Dependency,
            "market-emergence" => AxiomPrimitive::EmergencePattern,
            _ => return None,
        };
        Some(FunctorMapping {
            domain_concept: concept.to_string(),
            axiom_primitive: primitive,
            confidence: 0.9,
        })
    }

    fn all_mappings(&self) -> Vec<FunctorMapping> {
        [
            "volatility",
            "credit-risk",
            "portfolio-optimization",
            "interest-rate-dependency",
            "market-emergence",
        ]
        .iter()
        .filter_map(|c| self.map_concept(c))
        .collect()
    }
}

/// A test functor for the deployment domain.
pub struct DeploymentFunctor;

impl DomainFunctor for DeploymentFunctor {
    fn domain_name(&self) -> &str {
        "deployment"
    }

    fn map_concept(&self, concept: &str) -> Option<FunctorMapping> {
        let primitive = match concept {
            "rollback-risk" => AxiomPrimitive::Risk,
            "resource-quota" => AxiomPrimitive::ResourceAllocation,
            "service-dependency" => AxiomPrimitive::Dependency,
            "canary-constraint" => AxiomPrimitive::Constraint,
            "cascade-failure" => AxiomPrimitive::EmergencePattern,
            _ => return None,
        };
        Some(FunctorMapping {
            domain_concept: concept.to_string(),
            axiom_primitive: primitive,
            confidence: 0.85,
        })
    }

    fn all_mappings(&self) -> Vec<FunctorMapping> {
        [
            "rollback-risk",
            "resource-quota",
            "service-dependency",
            "canary-constraint",
            "cascade-failure",
        ]
        .iter()
        .filter_map(|c| self.map_concept(c))
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_map() {
        let mut registry = FunctorRegistry::new();
        registry.register(Box::new(FinanceFunctor)).unwrap();
        let mapping = registry.map_concept("finance", "volatility").unwrap();
        assert_eq!(mapping.axiom_primitive, AxiomPrimitive::Uncertainty);
    }

    #[test]
    fn duplicate_domain_rejected() {
        let mut registry = FunctorRegistry::new();
        registry.register(Box::new(FinanceFunctor)).unwrap();
        let err = registry.register(Box::new(FinanceFunctor)).unwrap_err();
        assert!(matches!(err, AxiomError::DomainAlreadyRegistered { .. }));
    }

    #[test]
    fn cross_domain_patterns_found() {
        let mut registry = FunctorRegistry::new();
        registry.register(Box::new(FinanceFunctor)).unwrap();
        registry.register(Box::new(DeploymentFunctor)).unwrap();
        let patterns = registry.find_cross_domain_patterns();
        // credit-risk and rollback-risk both map to Risk (0.9 similarity)
        assert!(!patterns.is_empty());
        let risk_pattern = patterns
            .iter()
            .find(|p| p.concept_a.contains("risk") && p.concept_b.contains("risk"));
        assert!(risk_pattern.is_some());
    }
}
