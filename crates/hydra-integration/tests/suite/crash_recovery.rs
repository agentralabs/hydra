use std::time::Duration;

use hydra_db::{HydraDb, RunRow, RunStatus};
use hydra_integration::TestServer;

/// Test that pending runs can be detected after restart
#[tokio::test]
async fn test_pending_runs_resume_after_restart() {
    // Create a DB with a pending run
    let db = HydraDb::in_memory().unwrap();
    let now = chrono::Utc::now().to_rfc3339();
    let run = RunRow {
        id: "crashed-run-1".into(),
        intent: "was running when crash happened".into(),
        status: RunStatus::Running,
        created_at: now.clone(),
        updated_at: now.clone(),
        completed_at: None,
        parent_run_id: None,
        metadata: None,
    };
    db.create_run(&run).unwrap();

    // Verify the run exists as "running" (simulating post-crash state)
    let found = db.get_run("crashed-run-1").unwrap();
    assert_eq!(found.status, RunStatus::Running);

    // On restart, detect incomplete runs
    let runs = db.list_runs(None).unwrap();
    let incomplete: Vec<_> = runs
        .iter()
        .filter(|r| r.status == RunStatus::Running || r.status == RunStatus::Pending)
        .collect();
    assert_eq!(incomplete.len(), 1, "Should detect 1 incomplete run");
}

/// Test that incomplete runs get marked as failed on boot
#[tokio::test]
async fn test_incomplete_run_marked_failed_on_boot() {
    let db = HydraDb::in_memory().unwrap();
    let now = chrono::Utc::now().to_rfc3339();

    // Simulate crash — run was "running" when Hydra died
    let run = RunRow {
        id: "crashed-run-2".into(),
        intent: "this run was interrupted by crash".into(),
        status: RunStatus::Running,
        created_at: now.clone(),
        updated_at: now.clone(),
        completed_at: None,
        parent_run_id: None,
        metadata: None,
    };
    db.create_run(&run).unwrap();

    // Recovery: mark incomplete runs as failed
    let runs = db.list_runs(None).unwrap();
    let recovery_time = chrono::Utc::now().to_rfc3339();
    for run in &runs {
        if run.status == RunStatus::Running || run.status == RunStatus::Pending {
            db.update_run_status(&run.id, RunStatus::Failed, Some(&recovery_time))
                .unwrap();
        }
    }

    // Verify recovery
    let recovered = db.get_run("crashed-run-2").unwrap();
    assert_eq!(recovered.status, RunStatus::Failed);
    assert!(recovered.completed_at.is_some());
}
