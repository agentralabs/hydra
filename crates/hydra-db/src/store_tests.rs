#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::schema::SCHEMA_VERSION;
    use crate::store::HydraDb;
    use crate::store_types::*;

    fn make_run(id: &str, intent: &str, status: RunStatus) -> RunRow {
        let now = Utc::now().to_rfc3339();
        RunRow {
            id: id.into(),
            intent: intent.into(),
            status,
            created_at: now.clone(),
            updated_at: now,
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
            description: format!("Step {}", seq),
            status: StepStatus::Pending,
            started_at: None,
            completed_at: None,
            result: None,
            evidence_refs: None,
        }
    }

    fn make_checkpoint(id: &str, run_id: &str) -> CheckpointRow {
        CheckpointRow {
            id: id.into(),
            run_id: run_id.into(),
            step_id: None,
            created_at: Utc::now().to_rfc3339(),
            state_snapshot: b"snapshot data".to_vec(),
            rollback_commands: None,
        }
    }

    fn make_approval(id: &str, run_id: &str) -> ApprovalRow {
        let now = Utc::now().to_rfc3339();
        ApprovalRow {
            id: id.into(),
            run_id: run_id.into(),
            action: "delete_file".into(),
            target: Some("/tmp/test".into()),
            risk_score: 0.8,
            created_at: now.clone(),
            expires_at: now,
            status: ApprovalStatus::Pending,
        }
    }

    // --- DB Init ---

    #[test]
    fn test_in_memory() {
        let db = HydraDb::in_memory().unwrap();
        assert_eq!(db.schema_version().unwrap(), SCHEMA_VERSION);
    }

    #[test]
    fn test_init_file_db() {
        let dir = std::env::temp_dir().join(format!("hydra_db_test_{}", std::process::id()));
        let path = dir.join("test.db");
        let db = HydraDb::init(&path).unwrap();
        assert_eq!(db.schema_version().unwrap(), SCHEMA_VERSION);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_migrate() {
        let db = HydraDb::in_memory().unwrap();
        db.migrate().unwrap();
        assert_eq!(db.schema_version().unwrap(), SCHEMA_VERSION);
    }

    #[test]
    fn test_clone_shares_connection() {
        let db = HydraDb::in_memory().unwrap();
        let db2 = db.clone();
        db.create_run(&make_run("r1", "test", RunStatus::Pending)).unwrap();
        let run = db2.get_run("r1").unwrap();
        assert_eq!(run.intent, "test");
    }

    // --- RunStatus ---

    #[test]
    fn test_run_status_as_str() {
        assert_eq!(RunStatus::Pending.as_str(), "pending");
        assert_eq!(RunStatus::Running.as_str(), "running");
        assert_eq!(RunStatus::Paused.as_str(), "paused");
        assert_eq!(RunStatus::Completed.as_str(), "completed");
        assert_eq!(RunStatus::Failed.as_str(), "failed");
        assert_eq!(RunStatus::Cancelled.as_str(), "cancelled");
    }

    #[test]
    fn test_run_status_from_str() {
        assert_eq!(RunStatus::from_str("pending"), Some(RunStatus::Pending));
        assert_eq!(RunStatus::from_str("running"), Some(RunStatus::Running));
        assert_eq!(RunStatus::from_str("invalid"), None);
    }

    #[test]
    fn test_run_status_serde() {
        for s in [RunStatus::Pending, RunStatus::Running, RunStatus::Paused, RunStatus::Completed, RunStatus::Failed, RunStatus::Cancelled] {
            let json = serde_json::to_string(&s).unwrap();
            let restored: RunStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(restored, s);
        }
    }

    // --- StepStatus ---

    #[test]
    fn test_step_status_as_str() {
        assert_eq!(StepStatus::Pending.as_str(), "pending");
        assert_eq!(StepStatus::Running.as_str(), "running");
        assert_eq!(StepStatus::Completed.as_str(), "completed");
        assert_eq!(StepStatus::Failed.as_str(), "failed");
        assert_eq!(StepStatus::Skipped.as_str(), "skipped");
    }

    #[test]
    fn test_step_status_from_str() {
        assert_eq!(StepStatus::from_str("skipped"), Some(StepStatus::Skipped));
        assert_eq!(StepStatus::from_str("nope"), None);
    }

    #[test]
    fn test_step_status_serde() {
        for s in [StepStatus::Pending, StepStatus::Running, StepStatus::Completed, StepStatus::Failed, StepStatus::Skipped] {
            let json = serde_json::to_string(&s).unwrap();
            let restored: StepStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(restored, s);
        }
    }

    // --- ApprovalStatus ---

    #[test]
    fn test_approval_status_as_str() {
        assert_eq!(ApprovalStatus::Pending.as_str(), "pending");
        assert_eq!(ApprovalStatus::Approved.as_str(), "approved");
        assert_eq!(ApprovalStatus::Denied.as_str(), "denied");
        assert_eq!(ApprovalStatus::Expired.as_str(), "expired");
    }

    #[test]
    fn test_approval_status_from_str() {
        assert_eq!(ApprovalStatus::from_str("approved"), Some(ApprovalStatus::Approved));
        assert_eq!(ApprovalStatus::from_str("unknown"), None);
    }

    #[test]
    fn test_approval_status_serde() {
        for s in [ApprovalStatus::Pending, ApprovalStatus::Approved, ApprovalStatus::Denied, ApprovalStatus::Expired] {
            let json = serde_json::to_string(&s).unwrap();
            let restored: ApprovalStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(restored, s);
        }
    }

    // --- CRUD Runs ---

    #[test]
    fn test_create_and_get_run() {
        let db = HydraDb::in_memory().unwrap();
        let run = make_run("r1", "refactor code", RunStatus::Pending);
        db.create_run(&run).unwrap();
        let fetched = db.get_run("r1").unwrap();
        assert_eq!(fetched.intent, "refactor code");
        assert_eq!(fetched.status, RunStatus::Pending);
    }

    #[test]
    fn test_get_run_not_found() {
        let db = HydraDb::in_memory().unwrap();
        let err = db.get_run("nonexistent").unwrap_err();
        assert!(matches!(err, DbError::NotFound(_)));
    }

    #[test]
    fn test_update_run_status() {
        let db = HydraDb::in_memory().unwrap();
        db.create_run(&make_run("r1", "test", RunStatus::Pending)).unwrap();
        db.update_run_status("r1", RunStatus::Running, None).unwrap();
        let run = db.get_run("r1").unwrap();
        assert_eq!(run.status, RunStatus::Running);
    }

    #[test]
    fn test_update_run_status_not_found() {
        let db = HydraDb::in_memory().unwrap();
        let err = db.update_run_status("nope", RunStatus::Running, None).unwrap_err();
        assert!(matches!(err, DbError::NotFound(_)));
    }

    #[test]
    fn test_list_runs_all() {
        let db = HydraDb::in_memory().unwrap();
        db.create_run(&make_run("r1", "a", RunStatus::Pending)).unwrap();
        db.create_run(&make_run("r2", "b", RunStatus::Running)).unwrap();
        let runs = db.list_runs(None).unwrap();
        assert_eq!(runs.len(), 2);
    }

    #[test]
    fn test_list_runs_by_status() {
        let db = HydraDb::in_memory().unwrap();
        db.create_run(&make_run("r1", "a", RunStatus::Pending)).unwrap();
        db.create_run(&make_run("r2", "b", RunStatus::Running)).unwrap();
        let pending = db.list_runs(Some(RunStatus::Pending)).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, "r1");
    }

    #[test]
    fn test_list_runs_empty() {
        let db = HydraDb::in_memory().unwrap();
        let runs = db.list_runs(None).unwrap();
        assert!(runs.is_empty());
    }

    #[test]
    fn test_delete_run() {
        let db = HydraDb::in_memory().unwrap();
        db.create_run(&make_run("r1", "test", RunStatus::Pending)).unwrap();
        db.delete_run("r1").unwrap();
        assert!(matches!(db.get_run("r1").unwrap_err(), DbError::NotFound(_)));
    }

    #[test]
    fn test_delete_run_not_found() {
        let db = HydraDb::in_memory().unwrap();
        let err = db.delete_run("nope").unwrap_err();
        assert!(matches!(err, DbError::NotFound(_)));
    }

    // --- CRUD Steps ---

    #[test]
    fn test_create_and_get_step() {
        let db = HydraDb::in_memory().unwrap();
        db.create_run(&make_run("r1", "test", RunStatus::Running)).unwrap();
        let step = make_step("s1", "r1", 1);
        db.create_step(&step).unwrap();
        let fetched = db.get_step("s1").unwrap();
        assert_eq!(fetched.run_id, "r1");
        assert_eq!(fetched.sequence, 1);
    }

    #[test]
    fn test_get_step_not_found() {
        let db = HydraDb::in_memory().unwrap();
        assert!(matches!(db.get_step("nope").unwrap_err(), DbError::NotFound(_)));
    }

    #[test]
    fn test_list_steps() {
        let db = HydraDb::in_memory().unwrap();
        db.create_run(&make_run("r1", "test", RunStatus::Running)).unwrap();
        db.create_step(&make_step("s1", "r1", 1)).unwrap();
        db.create_step(&make_step("s2", "r1", 2)).unwrap();
        let steps = db.list_steps("r1").unwrap();
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].sequence, 1);
        assert_eq!(steps[1].sequence, 2);
    }

    #[test]
    fn test_update_step_status() {
        let db = HydraDb::in_memory().unwrap();
        db.create_run(&make_run("r1", "test", RunStatus::Running)).unwrap();
        db.create_step(&make_step("s1", "r1", 1)).unwrap();
        db.update_step_status("s1", StepStatus::Completed, Some("2026-01-01"), Some("ok")).unwrap();
        let step = db.get_step("s1").unwrap();
        assert_eq!(step.status, StepStatus::Completed);
        assert_eq!(step.result, Some("ok".into()));
    }

    #[test]
    fn test_update_step_not_found() {
        let db = HydraDb::in_memory().unwrap();
        let err = db.update_step_status("nope", StepStatus::Failed, None, None).unwrap_err();
        assert!(matches!(err, DbError::NotFound(_)));
    }

    // --- CRUD Checkpoints ---

    #[test]
    fn test_create_and_get_checkpoint() {
        let db = HydraDb::in_memory().unwrap();
        db.create_run(&make_run("r1", "test", RunStatus::Running)).unwrap();
        let cp = make_checkpoint("cp1", "r1");
        db.create_checkpoint(&cp).unwrap();
        let fetched = db.get_checkpoint("cp1").unwrap();
        assert_eq!(fetched.state_snapshot, b"snapshot data");
    }

    #[test]
    fn test_get_checkpoint_not_found() {
        let db = HydraDb::in_memory().unwrap();
        assert!(matches!(db.get_checkpoint("nope").unwrap_err(), DbError::NotFound(_)));
    }

    #[test]
    fn test_list_checkpoints() {
        let db = HydraDb::in_memory().unwrap();
        db.create_run(&make_run("r1", "test", RunStatus::Running)).unwrap();
        db.create_checkpoint(&make_checkpoint("cp1", "r1")).unwrap();
        db.create_checkpoint(&make_checkpoint("cp2", "r1")).unwrap();
        let cps = db.list_checkpoints("r1").unwrap();
        assert_eq!(cps.len(), 2);
    }

    // --- CRUD Approvals ---

    #[test]
    fn test_create_and_get_approval() {
        let db = HydraDb::in_memory().unwrap();
        db.create_run(&make_run("r1", "test", RunStatus::Running)).unwrap();
        let a = make_approval("a1", "r1");
        db.create_approval(&a).unwrap();
        let fetched = db.get_approval("a1").unwrap();
        assert_eq!(fetched.action, "delete_file");
        assert_eq!(fetched.risk_score, 0.8);
    }

    #[test]
    fn test_get_approval_not_found() {
        let db = HydraDb::in_memory().unwrap();
        assert!(matches!(db.get_approval("nope").unwrap_err(), DbError::NotFound(_)));
    }

    #[test]
    fn test_update_approval_status() {
        let db = HydraDb::in_memory().unwrap();
        db.create_run(&make_run("r1", "test", RunStatus::Running)).unwrap();
        db.create_approval(&make_approval("a1", "r1")).unwrap();
        db.update_approval_status("a1", ApprovalStatus::Approved).unwrap();
        let a = db.get_approval("a1").unwrap();
        assert_eq!(a.status, ApprovalStatus::Approved);
    }

    #[test]
    fn test_update_approval_not_found() {
        let db = HydraDb::in_memory().unwrap();
        let err = db.update_approval_status("nope", ApprovalStatus::Denied).unwrap_err();
        assert!(matches!(err, DbError::NotFound(_)));
    }

    #[test]
    fn test_list_pending_approvals() {
        let db = HydraDb::in_memory().unwrap();
        db.create_run(&make_run("r1", "test", RunStatus::Running)).unwrap();
        db.create_approval(&make_approval("a1", "r1")).unwrap();
        db.create_approval(&make_approval("a2", "r1")).unwrap();
        db.update_approval_status("a2", ApprovalStatus::Approved).unwrap();
        let pending = db.list_pending_approvals().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, "a1");
    }
}
