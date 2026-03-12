use chrono::Utc;
use rusqlite::params;

use crate::store_types::*;

/// Insert/update/delete methods for HydraDb
impl crate::store::HydraDb {
    // ═══════════════════════════════════════════════════════
    // RUNS
    // ═══════════════════════════════════════════════════════

    pub fn create_run(&self, run: &RunRow) -> Result<(), DbError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO runs (id, intent, status, created_at, updated_at, completed_at, parent_run_id, metadata) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![run.id, run.intent, run.status.as_str(), run.created_at, run.updated_at, run.completed_at, run.parent_run_id, run.metadata],
        )?;
        Ok(())
    }

    pub fn update_run_status(
        &self,
        id: &str,
        status: RunStatus,
        completed_at: Option<&str>,
    ) -> Result<(), DbError> {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();
        let affected = conn.execute(
            "UPDATE runs SET status = ?1, updated_at = ?2, completed_at = ?3 WHERE id = ?4",
            params![status.as_str(), now, completed_at, id],
        )?;
        if affected == 0 {
            return Err(DbError::NotFound(format!("Run {id}")));
        }
        Ok(())
    }

    pub fn delete_run(&self, id: &str) -> Result<(), DbError> {
        let conn = self.conn.lock();
        let affected = conn.execute("DELETE FROM runs WHERE id = ?1", params![id])?;
        if affected == 0 {
            return Err(DbError::NotFound(format!("Run {id}")));
        }
        Ok(())
    }

    // ═══════════════════════════════════════════════════════
    // STEPS
    // ═══════════════════════════════════════════════════════

    pub fn create_step(&self, step: &StepRow) -> Result<(), DbError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO steps (id, run_id, sequence, description, status, started_at, completed_at, result, evidence_refs) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![step.id, step.run_id, step.sequence, step.description, step.status.as_str(), step.started_at, step.completed_at, step.result, step.evidence_refs],
        )?;
        Ok(())
    }

    pub fn update_step_status(
        &self,
        id: &str,
        status: StepStatus,
        completed_at: Option<&str>,
        result: Option<&str>,
    ) -> Result<(), DbError> {
        let conn = self.conn.lock();
        let affected = conn.execute(
            "UPDATE steps SET status = ?1, completed_at = ?2, result = ?3 WHERE id = ?4",
            params![status.as_str(), completed_at, result, id],
        )?;
        if affected == 0 {
            return Err(DbError::NotFound(format!("Step {id}")));
        }
        Ok(())
    }

    // ═══════════════════════════════════════════════════════
    // CHECKPOINTS
    // ═══════════════════════════════════════════════════════

    pub fn create_checkpoint(&self, cp: &CheckpointRow) -> Result<(), DbError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO checkpoints (id, run_id, step_id, created_at, state_snapshot, rollback_commands) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![cp.id, cp.run_id, cp.step_id, cp.created_at, cp.state_snapshot, cp.rollback_commands],
        )?;
        Ok(())
    }

    // ═══════════════════════════════════════════════════════
    // APPROVALS
    // ═══════════════════════════════════════════════════════

    pub fn create_approval(&self, a: &ApprovalRow) -> Result<(), DbError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO approvals (id, run_id, action, target, risk_score, created_at, expires_at, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![a.id, a.run_id, a.action, a.target, a.risk_score, a.created_at, a.expires_at, a.status.as_str()],
        )?;
        Ok(())
    }

    pub fn update_approval_status(&self, id: &str, status: ApprovalStatus) -> Result<(), DbError> {
        let conn = self.conn.lock();
        let affected = conn.execute(
            "UPDATE approvals SET status = ?1 WHERE id = ?2",
            params![status.as_str(), id],
        )?;
        if affected == 0 {
            return Err(DbError::NotFound(format!("Approval {id}")));
        }
        Ok(())
    }

    // ═══════════════════════════════════════════════════════
    // RECEIPTS
    // ═══════════════════════════════════════════════════════

    pub fn create_receipt(&self, r: &ReceiptRow) -> Result<(), DbError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO receipts (id, receipt_type, action, actor, tokens_used, risk_level, hash, prev_hash, sequence, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![r.id, r.receipt_type, r.action, r.actor, r.tokens_used, r.risk_level, r.hash, r.prev_hash, r.sequence, r.created_at],
        )?;
        Ok(())
    }

    // ═══════════════════════════════════════════════════════
    // SHADOW VALIDATIONS
    // ═══════════════════════════════════════════════════════

    pub fn create_shadow_validation(&self, sv: &ShadowValidationRow) -> Result<(), DbError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO shadow_validations (action_description, safe, divergence_count, critical_divergences, recommendation) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![sv.action_description, sv.safe as i32, sv.divergence_count, sv.critical_divergences, sv.recommendation],
        )?;
        Ok(())
    }

    // ═══════════════════════════════════════════════════════
    // ANOMALY EVENTS
    // ═══════════════════════════════════════════════════════

    pub fn create_anomaly_event(&self, ae: &AnomalyEventRow) -> Result<(), DbError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO anomaly_events (event_type, command, detail, severity, kill_switch_engaged) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![ae.event_type, ae.command, ae.detail, ae.severity, ae.kill_switch_engaged as i32],
        )?;
        Ok(())
    }

    // ═══════════════════════════════════════════════════════
    // TRUST SCORES
    // ═══════════════════════════════════════════════════════

    pub fn upsert_trust_score(&self, ts: &TrustScoreRow) -> Result<(), DbError> {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO trust_scores (domain, score, total_actions, successful_actions, failed_actions, autonomy_level, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) ON CONFLICT(domain) DO UPDATE SET score=?2, total_actions=?3, successful_actions=?4, failed_actions=?5, autonomy_level=?6, updated_at=?7",
            params![ts.domain, ts.score, ts.total_actions, ts.successful_actions, ts.failed_actions, ts.autonomy_level, now],
        )?;
        Ok(())
    }

    // ═══════════════════════════════════════════════════════
    // CURSOR SESSIONS & EVENTS
    // ═══════════════════════════════════════════════════════

    pub fn create_cursor_session(&self, id: &str, task_id: &str, mode: &str) -> Result<(), DbError> {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO cursor_sessions (id, task_id, mode, started_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, task_id, mode, now],
        )?;
        Ok(())
    }

    pub fn finish_cursor_session(&self, id: &str, event_count: i64, duration_ms: i64) -> Result<(), DbError> {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE cursor_sessions SET ended_at = ?1, event_count = ?2, total_duration_ms = ?3 WHERE id = ?4",
            params![now, event_count, duration_ms, id],
        )?;
        Ok(())
    }

    pub fn record_cursor_event(
        &self,
        session_id: &str,
        timestamp_ms: i64,
        event_type: &str,
        x: f64,
        y: f64,
        payload: Option<&str>,
    ) -> Result<(), DbError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO cursor_events (session_id, timestamp_ms, event_type, x, y, payload) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![session_id, timestamp_ms, event_type, x, y, payload],
        )?;
        Ok(())
    }

    // ═══════════════════════════════════════════════════════
    // BELIEFS
    // ═══════════════════════════════════════════════════════

    pub fn upsert_belief(&self, b: &BeliefRow) -> Result<(), DbError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO beliefs (id, category, subject, content, confidence, source, confirmations, contradictions, active, supersedes, superseded_by, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13) ON CONFLICT(id) DO UPDATE SET content=?4, confidence=?5, confirmations=?7, contradictions=?8, active=?9, superseded_by=?11, updated_at=?13",
            params![b.id, b.category, b.subject, b.content, b.confidence, b.source, b.confirmations, b.contradictions, b.active as i32, b.supersedes, b.superseded_by, b.created_at, b.updated_at],
        )?;
        Ok(())
    }

    pub fn supersede_belief(&self, old_id: &str, new_id: &str) -> Result<(), DbError> {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE beliefs SET active = 0, superseded_by = ?1, updated_at = ?2 WHERE id = ?3",
            params![new_id, now, old_id],
        )?;
        Ok(())
    }

    pub fn confirm_belief(&self, id: &str) -> Result<(), DbError> {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE beliefs SET confirmations = confirmations + 1, confidence = MIN(1.0, confidence + 0.02), updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    }

    pub fn contradict_belief(&self, id: &str) -> Result<(), DbError> {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE beliefs SET contradictions = contradictions + 1, confidence = MAX(0.0, confidence - 0.05), updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    }

    // ═══════════════════════════════════════════════════════
    // MCP DISCOVERED SKILLS
    // ═══════════════════════════════════════════════════════

    pub fn upsert_mcp_skill(&self, s: &McpDiscoveredSkillRow) -> Result<(), DbError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO mcp_discovered_skills (id, server_name, tool_name, description, input_schema, discovered_at, last_used_at, use_count, active) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) ON CONFLICT(id) DO UPDATE SET description=?4, input_schema=?5, active=?9",
            params![s.id, s.server_name, s.tool_name, s.description, s.input_schema, s.discovered_at, s.last_used_at, s.use_count, s.active as i32],
        )?;
        Ok(())
    }

    pub fn record_mcp_skill_use(&self, id: &str) -> Result<(), DbError> {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE mcp_discovered_skills SET use_count = use_count + 1, last_used_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    }

    // ═══════════════════════════════════════════════════════
    // FEDERATION STATE
    // ═══════════════════════════════════════════════════════

    pub fn upsert_federation_peer(&self, f: &FederationStateRow) -> Result<(), DbError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO federation_state (peer_id, peer_name, endpoint, trust_level, capabilities, federation_type, last_sync_version, last_seen, active_tasks, active) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10) ON CONFLICT(peer_id) DO UPDATE SET peer_name=?2, endpoint=?3, trust_level=?4, capabilities=?5, last_sync_version=?7, last_seen=?8, active_tasks=?9, active=?10",
            params![f.peer_id, f.peer_name, f.endpoint, f.trust_level, f.capabilities, f.federation_type, f.last_sync_version, f.last_seen, f.active_tasks, f.active as i32],
        )?;
        Ok(())
    }

    pub fn update_federation_sync(&self, peer_id: &str, version: i64) -> Result<(), DbError> {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE federation_state SET last_sync_version = ?1, last_seen = ?2 WHERE peer_id = ?3",
            params![version, now, peer_id],
        )?;
        Ok(())
    }

    // ═══════════════════════════════════════════════════════
    // REPAIR RUNS & CHECKS
    // ═══════════════════════════════════════════════════════

    pub fn create_repair_run(&self, r: &RepairRunRow) -> Result<(), DbError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO repair_runs (id, spec_file, task, status, iteration, max_iterations, checks_total, checks_passed, failure_log, started_at, completed_at, duration_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![r.id, r.spec_file, r.task, r.status, r.iteration, r.max_iterations, r.checks_total, r.checks_passed, r.failure_log, r.started_at, r.completed_at, r.duration_ms],
        )?;
        Ok(())
    }

    pub fn update_repair_run(&self, id: &str, status: &str, iteration: i64, checks_passed: i64, failure_log: Option<&str>) -> Result<(), DbError> {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE repair_runs SET status = ?1, iteration = ?2, checks_passed = ?3, failure_log = ?4, completed_at = ?5 WHERE id = ?6",
            params![status, iteration, checks_passed, failure_log, now, id],
        )?;
        Ok(())
    }

    pub fn create_repair_check(&self, c: &RepairCheckRow) -> Result<(), DbError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO repair_checks (run_id, iteration, check_name, check_command, passed, output) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![c.run_id, c.iteration, c.check_name, c.check_command, c.passed as i32, c.output],
        )?;
        Ok(())
    }
}
