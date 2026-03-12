#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::bridge::*;
    use crate::live_bridge::{BridgeConfig, LiveMcpBridge};

    #[test]
    fn test_bridge_config_defaults() {
        let config = BridgeConfig::default();
        assert_eq!(config.timeout, Duration::from_secs(5));
        assert_eq!(config.complex_timeout, Duration::from_secs(30));
        assert!(config.auto_start);
    }

    #[test]
    fn test_live_bridge_http_creation() {
        let bridge = LiveMcpBridge::http(
            SisterId::Memory,
            "http://localhost:3001",
            vec!["memory_add".into(), "memory_query".into()],
            BridgeConfig::default(),
        );
        assert_eq!(bridge.sister_id(), SisterId::Memory);
        assert_eq!(bridge.name(), "agentic-memory");
        assert_eq!(bridge.capabilities().len(), 2);
    }

    #[test]
    fn test_live_bridge_stdio_creation() {
        let bridge = LiveMcpBridge::stdio(
            SisterId::Vision,
            "agentic-vision-mcp",
            vec!["--workspace".into(), "/tmp/test".into()],
            vec!["vision_capture".into()],
            BridgeConfig::default(),
        );
        assert_eq!(bridge.sister_id(), SisterId::Vision);
        assert_eq!(bridge.name(), "agentic-vision");
    }

    #[test]
    fn test_timeout_for_complex_tools() {
        let bridge = LiveMcpBridge::http(
            SisterId::Codebase,
            "http://localhost:3003",
            vec![],
            BridgeConfig::default(),
        );
        // Simple tools get default timeout
        assert_eq!(
            bridge.timeout_for_tool("memory_add"),
            Duration::from_secs(5)
        );
        assert_eq!(
            bridge.timeout_for_tool("search_semantic"),
            Duration::from_secs(5)
        );
        // Complex tools (containing "omniscience", "crystallize", etc.) get complex timeout
        assert_eq!(
            bridge.timeout_for_tool("omniscience_search"),
            Duration::from_secs(30)
        );
        assert_eq!(
            bridge.timeout_for_tool("evolve_crystallize"),
            Duration::from_secs(30)
        );
    }

    #[tokio::test]
    async fn test_circuit_breaker_blocks_when_open() {
        let bridge = LiveMcpBridge::http(
            SisterId::Memory,
            "http://localhost:99999", // Non-existent
            vec!["memory_add".into()],
            BridgeConfig::default(),
        );
        // Force circuit open
        bridge
            .circuit_breaker
            .force_state(crate::circuit_breaker::CircuitState::Open);

        let result = bridge
            .call(SisterAction::new("memory_add", serde_json::json!({})))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Circuit breaker open"));
    }

    #[tokio::test]
    async fn test_health_check_with_open_circuit() {
        let bridge = LiveMcpBridge::http(
            SisterId::Memory,
            "http://localhost:99999",
            vec![],
            BridgeConfig::default(),
        );
        bridge
            .circuit_breaker
            .force_state(crate::circuit_breaker::CircuitState::Open);
        assert_eq!(bridge.health_check().await, HealthStatus::Unavailable);
    }
}
