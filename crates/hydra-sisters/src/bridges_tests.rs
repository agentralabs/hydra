#[cfg(test)]
mod tests {
    use crate::bridge::*;
    use super::*;

    // ── Bridge constructor tests ───────────────────────────

    #[test]
    fn test_memory_bridge_id() {
        let b = memory_bridge();
        assert_eq!(b.id, SisterId::Memory);
        assert_eq!(b.bridge_name, "agentic-memory");
        assert_eq!(b.bridge_version, "0.5.0");
    }

    #[test]
    fn test_memory_bridge_capabilities() {
        let b = memory_bridge();
        // V2 tools
        assert!(b.caps.contains(&"memory_add".to_string()));
        assert!(b.caps.contains(&"memory_query".to_string()));
        assert!(b.caps.contains(&"memory_similar".to_string()));
        assert!(b.caps.contains(&"session_start".to_string()));
        assert!(b.caps.contains(&"conversation_log".to_string()));
        // V3 capture tools (Universal Fix)
        assert!(b.caps.contains(&"memory_capture_message".to_string()));
        assert!(b.caps.contains(&"memory_capture_decision".to_string()));
        assert!(b.caps.contains(&"memory_search_semantic".to_string()));
        // V4 longevity tools
        assert!(b.caps.contains(&"memory_longevity_search".to_string()));
        assert!(b.caps.contains(&"memory_longevity_stats".to_string()));
        assert!(b.caps.contains(&"memory_hierarchy_query".to_string()));
        // Capability reporting
        assert!(b.caps.contains(&"memory_capabilities".to_string()));
        assert!(b.caps.len() >= 40);
    }

    #[test]
    fn test_vision_bridge_id() {
        let b = vision_bridge();
        assert_eq!(b.id, SisterId::Vision);
        assert_eq!(b.bridge_name, "agentic-vision");
        assert_eq!(b.bridge_version, "0.3.0");
    }

    #[test]
    fn test_vision_bridge_capabilities() {
        let b = vision_bridge();
        assert!(b.caps.contains(&"vision_capture".to_string()));
        assert!(b.caps.contains(&"vision_ocr".to_string()));
        assert!(b.caps.contains(&"observation_log".to_string()));
    }

    #[test]
    fn test_codebase_bridge_id() {
        let b = codebase_bridge();
        assert_eq!(b.id, SisterId::Codebase);
        assert_eq!(b.bridge_name, "agentic-codebase");
        assert_eq!(b.bridge_version, "0.3.0");
    }

    #[test]
    fn test_codebase_bridge_capabilities() {
        let b = codebase_bridge();
        assert!(b.caps.contains(&"search_semantic".to_string()));
        assert!(b.caps.contains(&"omniscience_search".to_string()));
        assert!(b.caps.contains(&"analysis_log".to_string()));
    }

    #[test]
    fn test_identity_bridge_id() {
        let b = identity_bridge();
        assert_eq!(b.id, SisterId::Identity);
        assert_eq!(b.bridge_name, "agentic-identity");
    }

    #[test]
    fn test_identity_bridge_capabilities() {
        let b = identity_bridge();
        assert!(b.caps.contains(&"identity_create".to_string()));
        assert!(b.caps.contains(&"action_sign".to_string()));
        assert!(b.caps.contains(&"trust_verify".to_string()));
    }

    #[test]
    fn test_time_bridge_id() {
        let b = time_bridge();
        assert_eq!(b.id, SisterId::Time);
        assert_eq!(b.bridge_name, "agentic-time");
    }

    #[test]
    fn test_time_bridge_capabilities() {
        let b = time_bridge();
        assert!(b.caps.contains(&"time_deadline_add".to_string()));
        assert!(b.caps.contains(&"time_decay_create".to_string()));
    }

    #[test]
    fn test_contract_bridge_id() {
        let b = contract_bridge();
        assert_eq!(b.id, SisterId::Contract);
        assert_eq!(b.bridge_name, "agentic-contract");
    }

    #[test]
    fn test_contract_bridge_capabilities() {
        let b = contract_bridge();
        assert!(b.caps.contains(&"contract_create".to_string()));
        assert!(b.caps.contains(&"policy_check".to_string()));
        assert!(b.caps.contains(&"violation_report".to_string()));
    }

    #[test]
    fn test_comm_bridge_id() {
        let b = comm_bridge();
        assert_eq!(b.id, SisterId::Comm);
        assert_eq!(b.bridge_name, "agentic-comm");
    }

    #[test]
    fn test_comm_bridge_capabilities() {
        let b = comm_bridge();
        assert!(b.caps.contains(&"comm_channel".to_string()));
        assert!(b.caps.contains(&"comm_message".to_string()));
        assert!(b.caps.contains(&"comm_health".to_string()));
    }

    #[test]
    fn test_planning_bridge_id() {
        let b = planning_bridge();
        assert_eq!(b.id, SisterId::Planning);
        assert_eq!(b.bridge_name, "agentic-planning");
    }

    #[test]
    fn test_planning_bridge_capabilities() {
        let b = planning_bridge();
        assert!(b.caps.contains(&"planning_goal".to_string()));
        assert!(b.caps.contains(&"planning_singularity".to_string()));
    }

    #[test]
    fn test_cognition_bridge_id() {
        let b = cognition_bridge();
        assert_eq!(b.id, SisterId::Cognition);
        assert_eq!(b.bridge_name, "agentic-cognition");
    }

    #[test]
    fn test_cognition_bridge_capabilities() {
        let b = cognition_bridge();
        assert!(b.caps.contains(&"cognition_model_create".to_string()));
        assert!(b.caps.contains(&"cognition_bias_detect".to_string()));
    }

    #[test]
    fn test_reality_bridge_id() {
        let b = reality_bridge();
        assert_eq!(b.id, SisterId::Reality);
        assert_eq!(b.bridge_name, "agentic-reality");
    }

    #[test]
    fn test_reality_bridge_capabilities() {
        let b = reality_bridge();
        assert!(b.caps.contains(&"reality_hallucination".to_string()));
        assert!(b.caps.contains(&"reality_verify".to_string()));
    }

    #[test]
    fn test_forge_bridge_id() {
        let b = forge_bridge();
        assert_eq!(b.id, SisterId::Forge);
        assert_eq!(b.bridge_name, "agentic-forge");
        assert_eq!(b.bridge_version, "0.1.0");
    }

    #[test]
    fn test_forge_bridge_capabilities() {
        let b = forge_bridge();
        assert!(b.caps.contains(&"forge_blueprint_create".to_string()));
        assert!(b.caps.contains(&"forge_validate".to_string()));
    }

    #[test]
    fn test_aegis_bridge_id() {
        let b = aegis_bridge();
        assert_eq!(b.id, SisterId::Aegis);
        assert_eq!(b.bridge_name, "agentic-aegis");
        assert_eq!(b.bridge_version, "0.1.0");
    }

    #[test]
    fn test_aegis_bridge_capabilities() {
        let b = aegis_bridge();
        assert!(b.caps.contains(&"aegis_validate_streaming".to_string()));
        assert!(b.caps.contains(&"aegis_scan_security".to_string()));
    }

    #[test]
    fn test_veritas_bridge_id() {
        let b = veritas_bridge();
        assert_eq!(b.id, SisterId::Veritas);
        assert_eq!(b.bridge_name, "agentic-veritas");
        assert_eq!(b.bridge_version, "0.1.0");
    }

    #[test]
    fn test_veritas_bridge_capabilities() {
        let b = veritas_bridge();
        assert!(b.caps.contains(&"veritas_compile_intent".to_string()));
        assert!(b.caps.contains(&"veritas_confidence_score".to_string()));
    }

    #[test]
    fn test_evolve_bridge_id() {
        let b = evolve_bridge();
        assert_eq!(b.id, SisterId::Evolve);
        assert_eq!(b.bridge_name, "agentic-evolve");
        assert_eq!(b.bridge_version, "0.1.0");
    }

    #[test]
    fn test_evolve_bridge_capabilities() {
        let b = evolve_bridge();
        assert!(b.caps.contains(&"evolve_pattern_store".to_string()));
        assert!(b.caps.contains(&"evolve_crystallize".to_string()));
        assert!(b.caps.contains(&"evolve_collective_sync".to_string()));
    }

    // ── all_bridges tests ──────────────────────────────────

    #[test]
    fn test_all_bridges_count() {
        assert_eq!(all_bridges().len(), 14);
    }

    #[test]
    fn test_all_bridges_unique_ids() {
        let bridges = all_bridges();
        let mut ids: Vec<_> = bridges.iter().map(|b| b.id).collect();
        ids.sort_by_key(|id| format!("{:?}", id));
        ids.dedup();
        assert_eq!(ids.len(), 14);
    }

    #[test]
    fn test_all_bridges_have_capabilities() {
        for b in all_bridges() {
            assert!(!b.caps.is_empty(), "{} has no capabilities", b.bridge_name);
        }
    }

    #[test]
    fn test_all_bridges_default_available() {
        for b in all_bridges() {
            assert!(
                b.available.load(std::sync::atomic::Ordering::SeqCst),
                "{} not available by default",
                b.bridge_name
            );
        }
    }

    #[test]
    fn test_all_bridges_default_timeout() {
        for b in all_bridges() {
            assert_eq!(
                b.timeout,
                std::time::Duration::from_secs(5),
                "{} has wrong default timeout",
                b.bridge_name
            );
        }
    }

    // ── McpSisterBridge methods ────────────────────────────

    #[test]
    fn test_set_available() {
        let b = memory_bridge();
        assert!(b.available.load(std::sync::atomic::Ordering::SeqCst));
        b.set_available(false);
        assert!(!b.available.load(std::sync::atomic::Ordering::SeqCst));
        b.set_available(true);
        assert!(b.available.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn test_set_timeout() {
        let mut b = memory_bridge();
        b.set_timeout(std::time::Duration::from_secs(30));
        assert_eq!(b.timeout, std::time::Duration::from_secs(30));
    }

    // ── SisterBridge trait impl tests ──────────────────────

    #[tokio::test]
    async fn test_bridge_health_check_healthy() {
        let b = memory_bridge();
        assert_eq!(b.health_check().await, HealthStatus::Healthy);
    }

    #[tokio::test]
    async fn test_bridge_health_check_unavailable() {
        let b = memory_bridge();
        b.set_available(false);
        assert_eq!(b.health_check().await, HealthStatus::Unavailable);
    }

    #[tokio::test]
    async fn test_bridge_call_success() {
        let b = memory_bridge();
        let action = SisterAction::new("memory_add", serde_json::json!({"content": "test"}));
        let result = b.call(action).await;
        assert!(result.is_ok());
        let r = result.unwrap();
        assert_eq!(r.data["sister"], "agentic-memory");
        assert_eq!(r.data["tool"], "memory_add");
    }

    #[tokio::test]
    async fn test_bridge_call_unavailable() {
        let b = memory_bridge();
        b.set_available(false);
        let action = SisterAction::new("memory_add", serde_json::json!({}));
        let result = b.call(action).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.retryable);
        assert_eq!(err.sister_id, SisterId::Memory);
    }

    #[tokio::test]
    async fn test_bridge_batch_call() {
        let b = memory_bridge();
        let actions = vec![
            SisterAction::new("memory_add", serde_json::json!({})),
            SisterAction::new("memory_query", serde_json::json!({})),
        ];
        let results = b.batch_call(actions).await;
        assert_eq!(results.len(), 2);
        assert!(results[0].is_ok());
        assert!(results[1].is_ok());
    }

    #[tokio::test]
    async fn test_bridge_batch_call_empty() {
        let b = memory_bridge();
        let results = b.batch_call(vec![]).await;
        assert!(results.is_empty());
    }

    #[test]
    fn test_bridge_capabilities_returns_correct_list() {
        let b = memory_bridge();
        let caps = b.capabilities();
        assert!(caps.contains(&"memory_add".to_string()));
    }

    #[test]
    fn test_bridge_sister_id_trait() {
        let b = vision_bridge();
        assert_eq!(b.sister_id(), SisterId::Vision);
    }

    #[test]
    fn test_bridge_name_trait() {
        let b = codebase_bridge();
        assert_eq!(b.name(), "agentic-codebase");
    }

    #[test]
    fn test_bridge_version_trait() {
        let b = identity_bridge();
        assert_eq!(b.version(), "0.3.0");
    }
}
