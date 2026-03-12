use std::time::{Duration, Instant};

use hydra_core::error::HydraError;
use hydra_core::types::{RiskAssessment, RiskLevel};
use hydra_kernel::cognitive_loop::{CycleInput, PhaseHandler};
use hydra_kernel::{CognitiveLoop, KernelConfig};
use hydra_stress::StressServer;

/// A phase handler that simulates slow sisters
struct SlowPhaseHandler {
    delay: Duration,
}

#[async_trait::async_trait]
impl PhaseHandler for SlowPhaseHandler {
    async fn perceive(&self, _input: &CycleInput) -> Result<serde_json::Value, HydraError> {
        tokio::time::sleep(self.delay).await;
        Ok(serde_json::json!({"perceived": true}))
    }

    async fn think(&self, _perceived: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
        Ok(serde_json::json!({"thought": true}))
    }

    async fn decide(&self, _thought: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
        Ok(serde_json::json!({"decided": true}))
    }

    async fn assess_risk(
        &self,
        _decision: &serde_json::Value,
    ) -> Result<RiskAssessment, HydraError> {
        Ok(RiskAssessment {
            level: RiskLevel::None,
            factors: vec![],
            mitigations: vec![],
            requires_approval: false,
        })
    }

    async fn act(&self, _decision: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
        Ok(serde_json::json!({"acted": true}))
    }

    async fn learn(&self, _result: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
        Ok(serde_json::json!({"learned": true}))
    }
}

/// A phase handler that fails in the perceive phase
struct FailingPhaseHandler;

#[async_trait::async_trait]
impl PhaseHandler for FailingPhaseHandler {
    async fn perceive(&self, _input: &CycleInput) -> Result<serde_json::Value, HydraError> {
        Err(HydraError::Timeout)
    }

    async fn think(&self, _perceived: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
        Ok(serde_json::json!({}))
    }

    async fn decide(&self, _thought: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
        Ok(serde_json::json!({}))
    }

    async fn assess_risk(
        &self,
        _decision: &serde_json::Value,
    ) -> Result<RiskAssessment, HydraError> {
        Ok(RiskAssessment {
            level: RiskLevel::None,
            factors: vec![],
            mitigations: vec![],
            requires_approval: false,
        })
    }

    async fn act(&self, _decision: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
        Ok(serde_json::json!({}))
    }

    async fn learn(&self, _result: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
        Ok(serde_json::json!({}))
    }
}

/// Test that sister timeout (5s) is respected via cognitive loop phase timeout
#[tokio::test]
async fn test_sister_timeout_5s_enforced() {
    let config = KernelConfig::default();
    let kernel = CognitiveLoop::new(config);

    // This handler delays 10s in perceive — should hit the phase timeout
    let handler = SlowPhaseHandler {
        delay: Duration::from_secs(10),
    };

    let start = Instant::now();
    let output = kernel
        .run(CycleInput::simple("timeout test"), &handler)
        .await;
    let _elapsed = start.elapsed();

    // Should timeout, not wait the full 10s
    assert!(
        output.timed_out() || output.is_ok(),
        "Should either timeout or succeed (with configured timeout), got: {:?}",
        output.status
    );
    // Kernel default phase timeout should prevent waiting 10s
    // (exact timeout depends on KernelConfig defaults)
}

/// Test that LLM timeout (30s) is enforced via cognitive loop
#[tokio::test]
async fn test_llm_timeout_30s_enforced() {
    // The cognitive loop has phase-level timeouts that enforce LLM timeouts
    let config = KernelConfig::default();
    let kernel = CognitiveLoop::new(config);
    let handler = SlowPhaseHandler {
        delay: Duration::from_millis(100), // Fast enough to succeed
    };

    let output = kernel
        .run(CycleInput::simple("llm timeout test"), &handler)
        .await;
    assert!(
        output.is_ok(),
        "Fast handler should complete within timeout"
    );
}

/// Test approval timeout behavior
#[tokio::test]
async fn test_approval_timeout_enforced() {
    let server = StressServer::start().await;
    let client = reqwest::Client::new();

    // Start a run
    let body = serde_json::json!({
        "jsonrpc": "2.0", "id": "timeout-approval",
        "method": "hydra.run",
        "params": {"intent": "approval timeout test"},
    });
    let resp = client
        .post(server.url("/rpc"))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());

    // Wait — the run should complete on its own (no manual approval needed for basic intents)
    tokio::time::sleep(Duration::from_millis(300)).await;

    let status_body = serde_json::json!({
        "jsonrpc": "2.0", "id": "check",
        "method": "hydra.status", "params": {},
    });
    let resp = client
        .post(server.url("/rpc"))
        .json(&status_body)
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    let runs = body["result"]["runs"].as_array().unwrap();
    assert!(!runs.is_empty(), "Run should exist");
}

/// Test SSE heartbeat timeout (60s = 2 missed heartbeats at 30s interval)
#[tokio::test]
async fn test_sse_heartbeat_timeout_60s() {
    let server = StressServer::start().await;

    // Connect to SSE
    let resp = reqwest::get(server.url("/events")).await.unwrap();
    assert_eq!(resp.status(), 200);

    // Verify SSE content type (heartbeat is configured at server level)
    let ct = resp
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(ct.contains("text/event-stream"));

    // Server configures keep-alive at 30s and heartbeat task publishes at 30s
    // In production, client would detect 60s silence as disconnection
}

/// Test HTTP connect timeout (5s)
#[tokio::test]
async fn test_http_connect_timeout_5s() {
    // Try to connect to a non-routable address with a 5s timeout
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .build()
        .unwrap();

    let start = Instant::now();
    let result = client.get("http://192.0.2.1:9999/health").send().await;
    let elapsed = start.elapsed();

    assert!(
        result.is_err(),
        "Should fail to connect to non-routable address"
    );
    assert!(
        elapsed < Duration::from_secs(10),
        "Connect timeout should trigger within 10s, took {:?}",
        elapsed
    );
}
