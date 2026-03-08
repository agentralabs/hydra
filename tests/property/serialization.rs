//! Category 8: Property Tests — serialization roundtrips.

use proptest::prelude::*;

proptest! {
    #[test]
    fn proptest_token_budget_usage_never_negative(total in 1u64..1_000_000, usage in 0u64..2_000_000) {
        let mut budget = hydra_core::TokenBudget::new(total);
        budget.record_usage(usage);
        // remaining should never be negative (saturating)
        assert!(budget.remaining <= total);
        assert!(budget.utilization() >= 0.0);
        assert!(budget.utilization() <= 100.0);
    }

    #[test]
    fn proptest_risk_score_bounds(score in 0.0f64..1.0) {
        let assessment = hydra_core::RiskAssessment {
            level: if score > 0.9 { hydra_core::RiskLevel::Critical }
                   else if score > 0.7 { hydra_core::RiskLevel::High }
                   else if score > 0.5 { hydra_core::RiskLevel::Medium }
                   else if score > 0.3 { hydra_core::RiskLevel::Low }
                   else { hydra_core::RiskLevel::None },
            factors: vec![],
            mitigations: vec![],
            requires_approval: score > 0.5,
        };
        let computed = hydra_gate::RiskAssessor::risk_score(&assessment);
        assert!(computed >= 0.0);
        assert!(computed <= 1.0);
    }

    #[test]
    fn proptest_checkpoint_roundtrip(label in "[a-z]{1,20}", key in "[a-z]{1,10}", val in "[a-z]{1,50}") {
        let cp = hydra_inventions::resurrection::checkpoint::Checkpoint::create(
            &label,
            serde_json::json!({key: val}),
        );
        let json = serde_json::to_string(&cp).unwrap();
        let restored: hydra_inventions::resurrection::checkpoint::Checkpoint = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.label, label);
    }

    #[test]
    fn proptest_prime_ast_roundtrip(name in "[A-Z][a-z]{2,10}") {
        use animus::prime::ast::*;
        use animus::prime::serialize::PrimeSerializer;

        let node = PrimeNode::Entity {
            name: name.clone(),
            fields: vec![
                Field { name: "id".into(), type_: PrimeType::Uuid, constraints: vec![], optional: false },
            ],
        };
        let json = PrimeSerializer::to_json(&node).unwrap();
        let restored = PrimeSerializer::from_json(&json).unwrap();
        assert_eq!(restored.type_name(), "entity");
    }
}
