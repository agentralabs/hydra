use hydra_db::*;

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn make_run(id: &str, intent: &str) -> RunRow {
    let ts = now();
    RunRow {
        id: id.into(),
        intent: intent.into(),
        status: RunStatus::Pending,
        created_at: ts.clone(),
        updated_at: ts,
        completed_at: None,
        parent_run_id: None,
        metadata: None,
    }
}

fn make_step(id: &str, run_id: &str, seq: i32) -> StepRow {
    StepRow {
        id: id.into(),
        run_id: run_id.into(),
        sequence: seq,
        description: format!("Step {seq}"),
        status: StepStatus::Pending,
        started_at: None,
        completed_at: None,
        result: None,
        evidence_refs: None,
    }
}

// ═══════════════════════════════════════════════════════════
// INIT TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_db_init_creates_tables() {
    let dir = tempfile::tempdir().unwrap();
    let db = HydraDb::init(&dir.path().join("hydra.db")).unwrap();
    assert_eq!(db.schema_version().unwrap(), SCHEMA_VERSION);

    // Tables exist — can insert and query
    let run = make_run("r1", "test intent");
    db.create_run(&run).unwrap();
    let fetched = db.get_run("r1").unwrap();
    assert_eq!(fetched.intent, "test intent");
}

#[test]
fn test_db_init_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("hydra.db");

    // First init
    let db1 = HydraDb::init(&path).unwrap();
    db1.create_run(&make_run("r1", "first")).unwrap();

    // Second init — same path, should not lose data
    drop(db1);
    let db2 = HydraDb::init(&path).unwrap();
    let run = db2.get_run("r1").unwrap();
    assert_eq!(run.intent, "first");
    assert_eq!(db2.schema_version().unwrap(), SCHEMA_VERSION);
}

#[test]
fn test_db_migrate() {
    let db = HydraDb::in_memory().unwrap();
    // Migration is a no-op at v1, but should not error
    db.migrate().unwrap();
    assert_eq!(db.schema_version().unwrap(), SCHEMA_VERSION);
}

// ═══════════════════════════════════════════════════════════
// RUN CRUD
// ═══════════════════════════════════════════════════════════

#[test]
fn test_run_crud() {
    let db = HydraDb::in_memory().unwrap();

    // Create
    let run = make_run("r1", "refactor auth");
    db.create_run(&run).unwrap();

    // Read
    let fetched = db.get_run("r1").unwrap();
    assert_eq!(fetched.intent, "refactor auth");
    assert_eq!(fetched.status, RunStatus::Pending);

    // Update
    let ts = now();
    db.update_run_status("r1", RunStatus::Completed, Some(&ts))
        .unwrap();
    let updated = db.get_run("r1").unwrap();
    assert_eq!(updated.status, RunStatus::Completed);
    assert!(updated.completed_at.is_some());

    // List
    let all = db.list_runs(None).unwrap();
    assert_eq!(all.len(), 1);

    let completed = db.list_runs(Some(RunStatus::Completed)).unwrap();
    assert_eq!(completed.len(), 1);

    let pending = db.list_runs(Some(RunStatus::Pending)).unwrap();
    assert_eq!(pending.len(), 0);

    // Delete
    db.delete_run("r1").unwrap();
    assert!(db.get_run("r1").is_err());
}

#[test]
fn test_run_not_found() {
    let db = HydraDb::in_memory().unwrap();
    assert!(matches!(
        db.get_run("nonexistent"),
        Err(DbError::NotFound(_))
    ));
}

// ═══════════════════════════════════════════════════════════
// STEP CRUD
// ═══════════════════════════════════════════════════════════

#[test]
fn test_step_crud() {
    let db = HydraDb::in_memory().unwrap();
    db.create_run(&make_run("r1", "test")).unwrap();

    // Create steps
    db.create_step(&make_step("s1", "r1", 1)).unwrap();
    db.create_step(&make_step("s2", "r1", 2)).unwrap();

    // Read
    let step = db.get_step("s1").unwrap();
    assert_eq!(step.description, "Step 1");
    assert_eq!(step.status, StepStatus::Pending);

    // Update
    let ts = now();
    db.update_step_status("s1", StepStatus::Completed, Some(&ts), Some("success"))
        .unwrap();
    let updated = db.get_step("s1").unwrap();
    assert_eq!(updated.status, StepStatus::Completed);
    assert_eq!(updated.result.as_deref(), Some("success"));

    // List
    let steps = db.list_steps("r1").unwrap();
    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0].sequence, 1);
    assert_eq!(steps[1].sequence, 2);
}

// ═══════════════════════════════════════════════════════════
// CHECKPOINT CRUD
// ═══════════════════════════════════════════════════════════

#[test]
fn test_checkpoint_crud() {
    let db = HydraDb::in_memory().unwrap();
    db.create_run(&make_run("r1", "test")).unwrap();

    let cp = CheckpointRow {
        id: "cp1".into(),
        run_id: "r1".into(),
        step_id: None,
        created_at: now(),
        state_snapshot: b"snapshot-data-here".to_vec(),
        rollback_commands: Some("undo stuff".into()),
    };
    db.create_checkpoint(&cp).unwrap();

    let fetched = db.get_checkpoint("cp1").unwrap();
    assert_eq!(fetched.state_snapshot, b"snapshot-data-here");
    assert_eq!(fetched.rollback_commands.as_deref(), Some("undo stuff"));

    let list = db.list_checkpoints("r1").unwrap();
    assert_eq!(list.len(), 1);
}

// ═══════════════════════════════════════════════════════════
// APPROVAL CRUD
// ═══════════════════════════════════════════════════════════

#[test]
fn test_approval_crud() {
    let db = HydraDb::in_memory().unwrap();
    db.create_run(&make_run("r1", "test")).unwrap();

    let approval = ApprovalRow {
        id: "a1".into(),
        run_id: "r1".into(),
        action: "delete_file".into(),
        target: Some("/src/old.rs".into()),
        risk_score: 0.7,
        created_at: now(),
        expires_at: now(),
        status: ApprovalStatus::Pending,
    };
    db.create_approval(&approval).unwrap();

    let fetched = db.get_approval("a1").unwrap();
    assert_eq!(fetched.action, "delete_file");
    assert_eq!(fetched.status, ApprovalStatus::Pending);
    assert!((fetched.risk_score - 0.7).abs() < f64::EPSILON);

    // Update
    db.update_approval_status("a1", ApprovalStatus::Approved)
        .unwrap();
    let updated = db.get_approval("a1").unwrap();
    assert_eq!(updated.status, ApprovalStatus::Approved);

    // List pending (should be empty now)
    let pending = db.list_pending_approvals().unwrap();
    assert_eq!(pending.len(), 0);
}

// ═══════════════════════════════════════════════════════════
// CASCADE DELETE
// ═══════════════════════════════════════════════════════════

#[test]
fn test_cascade_delete() {
    let db = HydraDb::in_memory().unwrap();
    db.create_run(&make_run("r1", "test")).unwrap();
    db.create_step(&make_step("s1", "r1", 1)).unwrap();
    db.create_checkpoint(&CheckpointRow {
        id: "cp1".into(),
        run_id: "r1".into(),
        step_id: Some("s1".into()),
        created_at: now(),
        state_snapshot: vec![1, 2, 3],
        rollback_commands: None,
    })
    .unwrap();
    db.create_approval(&ApprovalRow {
        id: "a1".into(),
        run_id: "r1".into(),
        action: "test".into(),
        target: None,
        risk_score: 0.1,
        created_at: now(),
        expires_at: now(),
        status: ApprovalStatus::Pending,
    })
    .unwrap();

    // Delete the run — all children should cascade
    db.delete_run("r1").unwrap();
    assert!(db.get_step("s1").is_err());
    assert!(db.get_checkpoint("cp1").is_err());
    assert!(db.get_approval("a1").is_err());
}

// ═══════════════════════════════════════════════════════════
// CONCURRENT ACCESS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_concurrent_access() {
    let db = HydraDb::in_memory().unwrap();

    // Clone and use from multiple threads
    let handles: Vec<_> = (0..4)
        .map(|i| {
            let db = db.clone();
            std::thread::spawn(move || {
                let id = format!("r{i}");
                db.create_run(&make_run(&id, &format!("intent {i}")))
                    .unwrap();
                let run = db.get_run(&id).unwrap();
                assert_eq!(run.intent, format!("intent {i}"));
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }

    let all = db.list_runs(None).unwrap();
    assert_eq!(all.len(), 4);
}
