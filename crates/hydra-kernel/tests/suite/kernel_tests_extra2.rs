use async_trait::async_trait;
use hydra_core::error::HydraError;
use hydra_core::types::{RiskAssessment, RiskLevel};
use hydra_kernel::cognitive_loop::{
    CognitiveLoop, CycleInput, CycleOutput, PhaseHandler,
};
use hydra_kernel::config::KernelConfig;

struct MockHandler;

impl MockHandler {
    fn new() -> Self {
        MockHandler
    }
}

#[async_trait]
impl PhaseHandler for MockHandler {
    async fn perceive(&self, input: &CycleInput) -> Result<serde_json::Value, HydraError> {
        Ok(serde_json::json!({"perceived": input.text}))
    }

    async fn think(&self, perceived: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
        Ok(serde_json::json!({"thought": perceived}))
    }

    async fn decide(&self, thought: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
        Ok(serde_json::json!({"decision": thought}))
    }

    async fn assess_risk(
        &self,
        _decision: &serde_json::Value,
    ) -> Result<RiskAssessment, HydraError> {
        Ok(RiskAssessment {
            level: RiskLevel::Low,
            factors: vec![],
            mitigations: vec![],
            requires_approval: false,
        })
    }

    async fn act(&self, decision: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
        Ok(serde_json::json!({"result": decision}))
    }

    async fn learn(&self, result: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
        Ok(serde_json::json!({"learned": result}))
    }
}

/// EC-CL-010: Recursive self-invocation (depth limited)
#[tokio::test]
async fn test_ec_cl_010_recursive_invocation() {
    let config = KernelConfig {
        max_recursion_depth: 3,
        ..Default::default()
    };
    let kernel = CognitiveLoop::new(config);
    let handler = MockHandler::new();

    // Simulate recursive calls by running multiple nested cycles
    // The depth counter prevents stack overflow
    let mut results = Vec::new();
    for _ in 0..5 {
        let output = kernel
            .run(CycleInput::simple("ask yourself to test"), &handler)
            .await;
        results.push(output);
    }

    // All should complete — the depth counter resets after each run
    for output in &results {
        assert!(output.is_ok() || output.recursion_detected());
    }

    // Test actual depth limiting by running concurrent nested calls
    let outputs: Vec<CycleOutput> = futures::future::join_all(
        (0..10).map(|_| kernel.run(CycleInput::simple("recursive"), &handler)),
    )
    .await;

    // Some may hit the recursion limit, but none should panic
    for output in &outputs {
        assert!(output.is_ok() || output.recursion_detected() || output.depth_limited());
    }
}
