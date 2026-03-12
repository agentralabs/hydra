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

    // --- Row types ---

    #[test]
    fn test_run_row_serde() {
        let run = make_run("r1", "test", RunStatus::Completed);
        let json = serde_json::to_string(&run).unwrap();
        let restored: RunRow = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, "r1");
        assert_eq!(restored.status, RunStatus::Completed);
    }

    #[test]
    fn test_step_row_serde() {
        let step = make_step("s1", "r1", 3);
        let json = serde_json::to_string(&step).unwrap();
        let restored: StepRow = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.sequence, 3);
    }

    #[test]
    fn test_checkpoint_row_serde() {
        let cp = make_checkpoint("cp1", "r1");
        let json = serde_json::to_string(&cp).unwrap();
        let restored: CheckpointRow = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, "cp1");
    }

    #[test]
    fn test_approval_row_serde() {
        let a = make_approval("a1", "r1");
        let json = serde_json::to_string(&a).unwrap();
        let restored: ApprovalRow = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.action, "delete_file");
    }

    // --- Run with metadata and parent ---

    #[test]
    fn test_run_with_metadata() {
        let db = HydraDb::in_memory().unwrap();
        let mut run = make_run("r1", "test", RunStatus::Pending);
        run.metadata = Some(r#"{"key":"value"}"#.into());
        db.create_run(&run).unwrap();
        let fetched = db.get_run("r1").unwrap();
        assert_eq!(fetched.metadata, Some(r#"{"key":"value"}"#.into()));
    }

    #[test]
    fn test_run_with_parent() {
        let db = HydraDb::in_memory().unwrap();
        db.create_run(&make_run("parent", "parent intent", RunStatus::Running)).unwrap();
        let mut child = make_run("child", "child intent", RunStatus::Pending);
        child.parent_run_id = Some("parent".into());
        db.create_run(&child).unwrap();
        let fetched = db.get_run("child").unwrap();
        assert_eq!(fetched.parent_run_id, Some("parent".into()));
    }

    // --- Cascade delete ---

    #[test]
    fn test_delete_run_cascades_steps() {
        let db = HydraDb::in_memory().unwrap();
        db.create_run(&make_run("r1", "test", RunStatus::Running)).unwrap();
        db.create_step(&make_step("s1", "r1", 1)).unwrap();
        db.delete_run("r1").unwrap();
        assert!(matches!(db.get_step("s1").unwrap_err(), DbError::NotFound(_)));
    }

    // --- Connection sharing ---

    #[test]
    fn test_connection_returns_arc() {
        let db = HydraDb::in_memory().unwrap();
        let conn = db.connection();
        let guard = conn.lock();
        let v: u32 = guard.query_row("SELECT version FROM schema_version LIMIT 1", [], |row| row.get(0)).unwrap();
        assert_eq!(v, SCHEMA_VERSION);
    }

    // --- Belief CRUD ---

    #[test]
    fn test_belief_upsert_and_retrieve() {
        let db = HydraDb::in_memory().unwrap();
        let now = "2026-03-09T00:00:00Z".to_string();
        let belief = BeliefRow {
            id: "b1".into(),
            category: "fact".into(),
            subject: "PostgreSQL and Express for this project".into(),
            content: "we're using PostgreSQL and Express for this project".into(),
            confidence: 0.95,
            source: "user_stated".into(),
            confirmations: 0,
            contradictions: 0,
            active: true,
            supersedes: None,
            superseded_by: None,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        db.upsert_belief(&belief).unwrap();
        let active = db.get_active_beliefs(10).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].subject, "PostgreSQL and Express for this project");
    }

    #[test]
    fn test_belief_supersede() {
        let db = HydraDb::in_memory().unwrap();
        let now = "2026-03-09T00:00:00Z".to_string();
        // Store original belief
        db.upsert_belief(&BeliefRow {
            id: "b1".into(),
            category: "fact".into(),
            subject: "PostgreSQL and Express for this project".into(),
            content: "we're using PostgreSQL and Express for this project".into(),
            confidence: 0.95,
            source: "user_stated".into(),
            confirmations: 0, contradictions: 0, active: true,
            supersedes: None, superseded_by: None,
            created_at: now.clone(), updated_at: now.clone(),
        }).unwrap();

        // Supersede with correction
        db.supersede_belief("b1", "b2").unwrap();
        db.upsert_belief(&BeliefRow {
            id: "b2".into(),
            category: "correction".into(),
            subject: "FastAPI instead of Express".into(),
            content: "actually, we switched to FastAPI instead of Express".into(),
            confidence: 0.99,
            source: "corrected".into(),
            confirmations: 0, contradictions: 0, active: true,
            supersedes: Some("b1".into()), superseded_by: None,
            created_at: now.clone(), updated_at: now.clone(),
        }).unwrap();

        // Old belief should be inactive
        let active = db.get_active_beliefs(10).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, "b2");
        assert_eq!(active[0].confidence, 0.99);
    }

    #[test]
    fn test_belief_keyword_search() {
        let db = HydraDb::in_memory().unwrap();
        let now = "2026-03-09T00:00:00Z".to_string();
        db.upsert_belief(&BeliefRow {
            id: "b1".into(),
            category: "fact".into(),
            subject: "PostgreSQL and Express for this project".into(),
            content: "we're using PostgreSQL and Express for this project".into(),
            confidence: 0.95,
            source: "user_stated".into(),
            confirmations: 0, contradictions: 0, active: true,
            supersedes: None, superseded_by: None,
            created_at: now.clone(), updated_at: now.clone(),
        }).unwrap();

        // Full subject search — should find
        let results = db.get_beliefs_by_subject("PostgreSQL and Express for this project").unwrap();
        assert_eq!(results.len(), 1);

        // Keyword search "Express" — should find (LIKE match)
        let results = db.get_beliefs_by_subject("Express").unwrap();
        assert_eq!(results.len(), 1);

        // Keyword search "FastAPI" — should NOT find (new tech not in old belief)
        let results = db.get_beliefs_by_subject("FastAPI").unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_belief_confirm_increases_confidence() {
        let db = HydraDb::in_memory().unwrap();
        let now = "2026-03-09T00:00:00Z".to_string();
        db.upsert_belief(&BeliefRow {
            id: "b1".into(),
            category: "fact".into(),
            subject: "PostgreSQL".into(),
            content: "we use PostgreSQL".into(),
            confidence: 0.95,
            source: "user_stated".into(),
            confirmations: 0, contradictions: 0, active: true,
            supersedes: None, superseded_by: None,
            created_at: now.clone(), updated_at: now.clone(),
        }).unwrap();

        db.confirm_belief("b1").unwrap();
        let active = db.get_active_beliefs(10).unwrap();
        assert_eq!(active[0].confirmations, 1);
        assert!((active[0].confidence - 0.97).abs() < 0.001);
    }
}
