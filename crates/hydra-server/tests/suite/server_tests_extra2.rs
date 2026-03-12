use std::time::Duration;

use hydra_server::handle_rpc;

use super::helpers::{rpc_body, test_state};

#[tokio::test]
async fn test_full_e2e_run() {
    // Full E2E: send message → cognitive loop → DB updated → SSE events
    let state = test_state();
    let mut rx = state.event_bus.subscribe();

    let resp = handle_rpc(
        &state,
        &rpc_body("hydra.run", serde_json::json!({"intent": "full e2e test"})),
    )
    .await;
    assert!(resp.is_success());
    let run_id = resp.result.unwrap()["run_id"].as_str().unwrap().to_string();

    // Wait for completion
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        tokio::select! {
            Ok(event) = rx.recv() => {
                let evt_type = serde_json::to_string(&event.event_type).unwrap_or_default();
                if evt_type.contains("run_completed") || evt_type.contains("run_error") {
                    break;
                }
            }
            _ = tokio::time::sleep_until(deadline) => {
                break;
            }
        }
    }

    // Verify DB was updated
    let run = state.db.get_run(&run_id).unwrap();
    assert!(
        run.status == hydra_db::RunStatus::Completed || run.status == hydra_db::RunStatus::Failed,
        "Run should be completed or failed, got {:?}",
        run.status
    );

    // Verify steps were created
    let steps = state.db.list_steps(&run_id).unwrap();
    assert!(
        steps.len() > 0,
        "Should have created DB steps for cognitive phases"
    );

    // Verify receipt was generated
    assert!(state.ledger.len() > 0, "Should have generated a receipt");
}
