//! Integration tests for hydra-axiom.

use hydra_axiom::{
    synthesize, AxiomMorphism, AxiomPrimitive, DeploymentFunctor, FinanceFunctor, FunctorRegistry,
};

#[test]
fn functor_registry_cross_domain() {
    let mut registry = FunctorRegistry::new();
    registry.register(Box::new(FinanceFunctor)).unwrap();
    registry.register(Box::new(DeploymentFunctor)).unwrap();
    assert_eq!(registry.domain_count(), 2);

    let patterns = registry.find_cross_domain_patterns();
    assert!(!patterns.is_empty(), "should find cross-domain patterns");
}

#[test]
fn synthesis_end_to_end() {
    let cap = synthesize(
        "risk-constrained-deployment",
        vec![
            AxiomPrimitive::Risk,
            AxiomPrimitive::Constraint,
            AxiomPrimitive::ResourceAllocation,
        ],
        vec![
            (0, 1, AxiomMorphism::Constrains),
            (1, 2, AxiomMorphism::Allocates),
        ],
    )
    .unwrap();
    assert_eq!(cap.components.len(), 3);
    assert_eq!(cap.connections.len(), 2);
    assert!(cap.confidence > 0.0);
}

#[test]
fn primitive_labels_unique() {
    let primitives = [
        AxiomPrimitive::Uncertainty,
        AxiomPrimitive::Probability,
        AxiomPrimitive::Constraint,
        AxiomPrimitive::Risk,
        AxiomPrimitive::Optimization,
        AxiomPrimitive::AdversarialModel,
        AxiomPrimitive::CausalLink,
        AxiomPrimitive::TemporalSequence,
        AxiomPrimitive::ResourceAllocation,
        AxiomPrimitive::InformationValue,
        AxiomPrimitive::Dependency,
        AxiomPrimitive::TrustRelation,
        AxiomPrimitive::CoordinationEquilibrium,
        AxiomPrimitive::EmergencePattern,
    ];
    let labels: Vec<&str> = primitives.iter().map(|p| p.label()).collect();
    for (i, label) in labels.iter().enumerate() {
        for (j, other) in labels.iter().enumerate() {
            if i != j {
                assert_ne!(label, other, "duplicate label found");
            }
        }
    }
}
