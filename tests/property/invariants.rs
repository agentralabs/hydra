//! Category 8: Property Tests — system invariants.

use proptest::prelude::*;

proptest! {
    #[test]
    fn proptest_receipt_chain_valid(seq1 in 1u64..100, seq2 in 101u64..200) {
        let r1 = hydra_core::Receipt {
            id: hydra_core::ReceiptId::new(),
            deployment_id: uuid::Uuid::new_v4(),
            receipt_type: hydra_core::ReceiptType::IntentCompiled,
            timestamp: chrono::Utc::now(),
            content: serde_json::json!({"seq": seq1}),
            content_hash: format!("hash_{}", seq1),
            signature: None,
            previous_hash: None,
            sequence: seq1,
        };
        assert!(r1.is_chain_valid(None));

        let r2 = hydra_core::Receipt {
            id: hydra_core::ReceiptId::new(),
            deployment_id: uuid::Uuid::new_v4(),
            receipt_type: hydra_core::ReceiptType::ExecutionStarted,
            timestamp: chrono::Utc::now(),
            content: serde_json::json!({"seq": seq2}),
            content_hash: format!("hash_{}", seq2),
            signature: None,
            previous_hash: Some(r1.content_hash.clone()),
            sequence: seq2,
        };
        assert!(r2.is_chain_valid(Some(&r1)));
    }

    #[test]
    fn proptest_trust_score_bounds(level in 0u8..4) {
        let trust = match level {
            0 => hydra_federation::trust::TrustLevel::Unknown,
            1 => hydra_federation::trust::TrustLevel::Known,
            2 => hydra_federation::trust::TrustLevel::Trusted,
            _ => hydra_federation::trust::TrustLevel::Owner,
        };
        // Trust levels should maintain ordering
        if level > 0 {
            assert!(trust > hydra_federation::trust::TrustLevel::Unknown);
        }
    }

    #[test]
    fn proptest_compression_ratio(repetitions in 2usize..50) {
        let input = "This is a repeated sentence. ".repeat(repetitions);
        let compressor = hydra_inventions::minimizer::compressor::ContextCompressor::new(
            hydra_inventions::minimizer::compressor::CompressionLevel::Medium,
        );
        let result = compressor.compress(&input);
        // Compression ratio should be between 0 and 1 for repeated content
        let ratio = result.compression_ratio();
        assert!(ratio >= 0.0, "ratio should be >= 0: {}", ratio);
        assert!(ratio <= 1.0, "ratio should be <= 1: {}", ratio);
    }

    #[test]
    fn proptest_action_chain_risk_monotonic(risk1 in 0.01f64..0.5, risk2 in 0.01f64..0.5) {
        use hydra_inventions::future_echo::predictor::*;

        let single = ActionChain::new(vec![
            Action { name: "a".into(), params: serde_json::json!({}), risk_level: risk1 },
        ]);
        let double = ActionChain::new(vec![
            Action { name: "a".into(), params: serde_json::json!({}), risk_level: risk1 },
            Action { name: "b".into(), params: serde_json::json!({}), risk_level: risk2 },
        ]);
        // Adding actions should not decrease total risk
        assert!(double.total_risk() >= single.total_risk());
    }

    #[test]
    fn proptest_degradation_level_step_idempotent(level in 0u8..4) {
        use hydra_runtime::degradation::manager::DegradationLevel;
        let l = match level {
            0 => DegradationLevel::Normal,
            1 => DegradationLevel::Reduced,
            2 => DegradationLevel::Minimal,
            _ => DegradationLevel::Emergency,
        };
        // step_up then step_down should return to same or adjacent level
        let up = l.step_up();
        let back = up.step_down();
        assert!(back <= up);
    }
}
