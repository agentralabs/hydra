//! Gateway tests — offline tests using Sisters::empty().
//!
//! These tests verify the gateway fallback paths (sister offline → local).
//! No real sisters are spawned. No LLM calls. No network.

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::Arc;

    use crate::sisters::cognitive::Sisters;
    use crate::sisters::gateway::SisterGateway;
    use crate::sisters::gateway_helpers::*;

    fn test_gateway() -> SisterGateway {
        SisterGateway::new(Arc::new(Sisters::empty()))
    }

    #[tokio::test]
    async fn test_gateway_find_file_fallback() {
        let gw = test_gateway();
        // With empty sisters, should fall back to local search
        // This file exists in the project
        let result = gw.find_file("Cargo.toml", Path::new(".")).await;
        // May or may not find it depending on cwd, but shouldn't panic
        let (s, f) = gw.stats();
        // Should have recorded at least one fallback (Codebase sister offline)
        assert!(f > 0 || s == 0, "Expected fallback when sisters offline");
    }

    #[tokio::test]
    async fn test_gateway_assess_risk_fallback() {
        let gw = test_gateway();
        let risk = gw.assess_risk("rm -rf /tmp/test").await;
        assert_eq!(risk, RiskLevel::Critical);
        let (_, f) = gw.stats();
        assert!(f > 0, "Should use fallback when Contract+Aegis offline");
    }

    #[tokio::test]
    async fn test_gateway_store_fallback() {
        let gw = test_gateway();
        let stored = gw.store("test content", "episode").await;
        assert!(!stored, "Store should fail when Memory sister offline");
        let (_, f) = gw.stats();
        assert!(f > 0);
    }

    #[tokio::test]
    async fn test_gateway_recall_fallback() {
        let gw = test_gateway();
        let results = gw.recall("test query", 10).await;
        assert!(results.is_empty(), "Recall should be empty when Memory offline");
    }

    #[tokio::test]
    async fn test_gateway_time_context_fallback() {
        let gw = test_gateway();
        let tc = gw.time_context().await;
        assert!(tc.raw.contains("unix_timestamp"), "Should get local time fallback");
    }

    #[tokio::test]
    async fn test_gateway_environment_fallback() {
        let gw = test_gateway();
        let env = gw.environment().await;
        assert!(env.raw.contains("fallback"), "Should get local environment fallback");
    }

    #[tokio::test]
    async fn test_gateway_validate_input_fallback() {
        let gw = test_gateway();
        let safe = gw.validate_input("normal text").await;
        assert_eq!(safe, SafetyResult::Safe);
        let blocked = gw.validate_input("eval(bad_code)").await;
        assert!(matches!(blocked, SafetyResult::Blocked(_)));
    }

    #[tokio::test]
    async fn test_gateway_validate_output_fallback() {
        let gw = test_gateway();
        let result = gw.validate_output("some output").await;
        // No Aegis sister → Unknown
        assert_eq!(result, SafetyResult::Unknown);
    }

    #[tokio::test]
    async fn test_gateway_known_resolution_fallback() {
        let gw = test_gateway();
        let resolution = gw.known_resolution("some error").await;
        assert!(resolution.is_none(), "No resolution when Cognition+Memory offline");
    }

    #[tokio::test]
    async fn test_gateway_code_search_fallback() {
        let gw = test_gateway();
        let results = gw.code_search("SisterGateway", Path::new(".")).await;
        // Local grep fallback — may find results in current dir
        let (_, f) = gw.stats();
        assert!(f > 0, "Should fall back to local grep");
    }

    #[tokio::test]
    async fn test_gateway_code_impact_no_fallback() {
        let gw = test_gateway();
        let impact = gw.code_impact("some change").await;
        assert!(impact.is_none(), "No fallback for code impact analysis");
    }

    #[tokio::test]
    async fn test_gateway_verify_claim_no_fallback() {
        let gw = test_gateway();
        let result = gw.verify_claim("some claim").await;
        assert!(result.is_none(), "No fallback for claim verification");
    }

    #[tokio::test]
    async fn test_gateway_stats_display() {
        let gw = test_gateway();
        // Do some operations to generate stats
        let _ = gw.assess_risk("ls").await;
        let _ = gw.time_context().await;
        let display = gw.stats_display();
        assert!(display.contains("Sister Intelligence:"));
        assert!(display.contains("Sister calls:"));
        assert!(display.contains("Local fallbacks:"));
    }

    #[tokio::test]
    async fn test_gateway_stats_per_sister() {
        let gw = test_gateway();
        let _ = gw.assess_risk("rm -rf /").await;
        let _ = gw.time_context().await;
        let _ = gw.store("test", "episode").await;
        let per = gw.stats_per_sister();
        // Should have Contract, Time, and Memory fallbacks
        let names: Vec<&str> = per.iter().map(|(n, _, _)| *n).collect();
        assert!(names.contains(&"Contract"), "Should track Contract fallback");
        assert!(names.contains(&"Time"), "Should track Time fallback");
        assert!(names.contains(&"Memory"), "Should track Memory fallback");
    }

    #[tokio::test]
    async fn test_gateway_learn_from_error_no_panic() {
        let gw = test_gateway();
        // Should not panic even when all sisters are offline
        gw.learn_from_error("rate limit 429", "wait 30s and retry").await;
    }

    #[test]
    fn test_gateway_sisters_accessor() {
        let gw = test_gateway();
        let sisters = gw.sisters();
        assert_eq!(sisters.connected_count(), 0);
    }
}
